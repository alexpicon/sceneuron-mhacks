// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The shared runtime around the overlay engine: one lock for every change, snapshot
//! publishing, throttled telemetry, presence timeouts, and debounced saving.

mod listeners;
mod schedule;
mod shared;
mod state;
mod subscribers;
mod worker;

use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Waits between the attempts of the final save at shutdown.
const FINAL_SAVE_RETRIES: [Duration; 2] = [Duration::from_millis(200), Duration::from_millis(600)];

use sceneuron_core::{Engine, Outcome, Presence, Project, Snapshot, ValidationError};

use self::shared::Shared;
use crate::control_view::ControlView;
use crate::{lock, to_json};

/// Timing and persistence settings for a [`Hub`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HubConfig {
    /// Where the project is saved; `None` keeps changes in memory only.
    pub project_path: Option<PathBuf>,
    /// Quiet time after the last project change before it is saved.
    pub save_debounce: Duration,
    /// How often presence timeouts are checked.
    pub tick_interval: Duration,
    /// Minimum time between snapshots published only for live telemetry.
    pub telemetry_interval: Duration,
}

impl Default for HubConfig {
    fn default() -> Self {
        Self {
            project_path: None,
            save_debounce: Duration::from_millis(500),
            tick_interval: Duration::from_millis(250),
            telemetry_interval: Duration::from_millis(500),
        }
    }
}

/// A change reported to listeners after it happened, in the order changes happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HubEvent {
    /// The effective overlay state changed; `revision` is the new snapshot revision.
    StateChanged { revision: u64 },
    /// Presence changed to this state.
    PresenceChanged(Presence),
    /// The saved project changed and will be persisted.
    ProjectChanged,
    /// Saving the project failed; it is retried. Sent once per streak of failures, and
    /// again when the final save at shutdown fails.
    SaveFailed(String),
}

/// Owns the engine and tells everyone else about its changes.
///
/// Every engine call runs under one lock with the hub clock. Snapshots are sent to
/// subscribers, listeners are called, and the project is saved only after that lock
/// is released, so a slow consumer never blocks a change. Calling a hub method from
/// inside an `update`, `try_update`, or `read` closure deadlocks.
pub struct Hub {
    shared: Arc<Shared>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl Hub {
    /// Validates the project and starts the background thread.
    ///
    /// The error also reports the unlikely case that the thread cannot be started.
    pub fn start(project: Project, config: HubConfig) -> Result<Arc<Hub>, ValidationError> {
        let engine = Engine::new(project, 0)?;
        let shared = Arc::new(Shared::new(engine, config));
        let background = Arc::clone(&shared);
        let worker = thread::Builder::new()
            .name("sceneuron-hub".to_owned())
            .spawn(move || worker::run(&background))
            .map_err(|error| ValidationError(format!("cannot start the hub thread: {error}")))?;
        Ok(Arc::new(Hub {
            shared,
            worker: Mutex::new(Some(worker)),
        }))
    }

    /// Monotonic milliseconds since the hub started; the clock for every engine call.
    pub fn now_ms(&self) -> u64 {
        self.shared.now_ms()
    }

    /// Changes the engine and publishes the result.
    pub fn update(&self, f: impl FnOnce(&mut Engine, u64) -> Outcome) -> Outcome {
        let Ok(outcome) = self
            .shared
            .apply(|engine, now| Ok::<_, std::convert::Infallible>(f(engine, now)));
        outcome
    }

    /// Like [`Hub::update`] for changes that can be refused; an error publishes nothing.
    pub fn try_update<E>(&self, f: impl FnOnce(&mut Engine, u64) -> Result<Outcome, E>) -> Result<Outcome, E> {
        self.shared.apply(f)
    }

    /// Reads engine state under the lock.
    pub fn read<R>(&self, f: impl FnOnce(&Engine) -> R) -> R {
        f(&self.shared.lock_state().engine)
    }

    pub fn snapshot(&self) -> Snapshot {
        self.shared.snapshot()
    }

    pub fn snapshot_json(&self) -> String {
        to_json(&self.snapshot())
    }

    /// Receives every published snapshot as JSON, starting with the current one.
    ///
    /// The channel is bounded: a subscriber that falls behind is dropped and its
    /// receiver disconnects, and so does every receiver after [`Hub::shutdown`].
    pub fn subscribe(&self) -> Receiver<Arc<str>> {
        self.shared.subscribe()
    }

    /// The control view now.
    pub(crate) fn control_view(&self) -> ControlView {
        let state = self.shared.lock_state();
        state.control_view(self.shared.now_ms())
    }

    /// Receives the control view after every change, starting with the current one, on a
    /// channel bounded like [`Hub::subscribe`]. Views of changes that left it the same are
    /// sent too; event streams leave them out.
    pub(crate) fn subscribe_control(&self) -> Receiver<Arc<ControlView>> {
        self.shared.subscribe_control()
    }

    /// Calls `listener` after each change, outside every hub lock, in the order changes
    /// happened. A panicking listener is isolated from the change and other listeners.
    /// Listeners run on a thread that made a change; changes they make are delivered after
    /// they return.
    pub fn add_listener(&self, listener: Box<dyn Fn(&HubEvent) + Send + Sync>) {
        self.shared.add_listener(listener);
    }

    /// Saves the project now if it has unsaved changes; returns the failure, if any.
    pub fn flush(&self) -> Result<(), String> {
        self.shared.save_now()
    }

    /// Stops the background thread, saves unsaved changes, disconnects subscribers, and
    /// drops listeners. Idempotent. Later project changes are saved immediately.
    ///
    /// The final save is tried a few times, since a sync tool or antivirus can hold the
    /// file briefly; if it still fails, listeners get [`HubEvent::SaveFailed`] first.
    pub fn shutdown(&self) {
        self.shared.stop();
        let worker = lock(&self.worker).take();
        if let Some(worker) = worker
            && worker.thread().id() != thread::current().id()
        {
            let _ = worker.join();
        }
        let mut result = self.shared.save_now();
        for wait in FINAL_SAVE_RETRIES {
            if result.is_ok() || !self.shared.unsaved() {
                break;
            }
            thread::sleep(wait);
            result = self.shared.save_now();
        }
        if let Err(message) = result {
            self.shared.final_save_failed(&message);
        }
        self.shared.close();
    }
}

impl Drop for Hub {
    fn drop(&mut self) {
        self.shutdown();
    }
}
