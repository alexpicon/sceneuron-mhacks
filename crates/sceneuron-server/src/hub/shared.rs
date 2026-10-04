// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Hub state shared with the background thread, and the change pipeline.
//!
//! Lock order: `save_lock` before `state`. While holding `state`, only the listener queue
//! is locked, to keep events in change order; the `feeds`, listener, and `schedule` locks
//! otherwise never nest.

use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use sceneuron_core::{Engine, Outcome, Snapshot, save_project};

use super::listeners::Listeners;
use super::schedule::Schedule;
use super::state::EngineState;
use super::subscribers::Feeds;
use super::{HubConfig, HubEvent};
use crate::control_view::ControlView;
use crate::{lock, to_json};

/// Wait before retrying a failed save, so a broken disk does not cause a busy loop.
const SAVE_RETRY: Duration = Duration::from_secs(5);

pub(super) struct Shared {
    started: Instant,
    pub(super) config: HubConfig,
    state: Mutex<EngineState>,
    feeds: Mutex<Feeds>,
    listeners: Listeners,
    schedule: Mutex<Schedule>,
    pub(super) wake: Condvar,
    save_lock: Mutex<()>,
    /// The last save failed; cleared by the next successful one.
    save_failing: AtomicBool,
}

impl Shared {
    pub(super) fn new(engine: Engine, config: HubConfig) -> Self {
        let started = Instant::now();
        Self {
            started,
            schedule: Mutex::new(Schedule::new(started, &config)),
            config,
            state: Mutex::new(EngineState::new(engine)),
            feeds: Mutex::new(Feeds::default()),
            listeners: Listeners::default(),
            wake: Condvar::new(),
            save_lock: Mutex::new(()),
            save_failing: AtomicBool::new(false),
        }
    }

    pub(super) fn now_ms(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    pub(super) fn lock_state(&self) -> MutexGuard<'_, EngineState> {
        lock(&self.state)
    }

    pub(super) fn lock_schedule(&self) -> MutexGuard<'_, Schedule> {
        lock(&self.schedule)
    }

    /// Runs one engine change, then publishes, schedules, and notifies outside the lock.
    pub(super) fn apply<E>(&self, f: impl FnOnce(&mut Engine, u64) -> Result<Outcome, E>) -> Result<Outcome, E> {
        let (outcome, published) = {
            let mut state = self.lock_state();
            // Read under the lock so engine calls see a clock that never goes back.
            let now = self.now_ms();
            let before = state.engine.presence();
            let outcome = f(&mut state.engine, now)?;
            let after = state.engine.presence();
            let published = if outcome.changed {
                state.telemetry_pending = false;
                Some(state.take_published(now))
            } else {
                state.telemetry_pending |= outcome.telemetry;
                None
            };
            let mut events = Vec::new();
            if let Some(published) = &published {
                events.push(HubEvent::StateChanged {
                    revision: published.snapshot.revision,
                });
            }
            if before != after {
                events.push(HubEvent::PresenceChanged(after));
            }
            if outcome.project_dirty {
                events.push(HubEvent::ProjectChanged);
            }
            self.listeners.queue(events);
            (outcome, published)
        };
        if let Some(published) = published {
            self.publish(published.sequence, &published.snapshot, Some(&published.control));
        } else if outcome.telemetry {
            self.lock_schedule().want_telemetry();
            self.wake.notify_all();
        }
        if outcome.project_dirty {
            self.mark_dirty();
        }
        self.listeners.deliver();
        Ok(outcome)
    }

    pub(super) fn tick(&self) {
        let Ok(_) = self.apply(|engine, now| Ok::<_, Infallible>(engine.tick(now)));
    }

    pub(super) fn snapshot(&self) -> Snapshot {
        let state = self.lock_state();
        state.engine.snapshot(self.now_ms())
    }

    /// Publishes telemetry that no snapshot has carried yet.
    pub(super) fn publish_telemetry(&self) {
        let taken = {
            let mut state = self.lock_state();
            if !state.telemetry_pending {
                return;
            }
            state.telemetry_pending = false;
            let now = self.now_ms();
            state.take_snapshot(now)
        };
        // Telemetry never changes the control view.
        self.publish(taken.0, &taken.1, None);
    }

    fn publish(&self, sequence: u64, snapshot: &Snapshot, control: Option<&Arc<ControlView>>) {
        let message: Arc<str> = to_json(snapshot).into();
        {
            let mut feeds = lock(&self.feeds);
            feeds.snapshots.publish(sequence, &message);
            if let Some(control) = control {
                feeds.controls.publish(sequence, control);
            }
        }
        self.lock_schedule().published(Instant::now());
    }

    pub(super) fn subscribe(&self) -> Receiver<Arc<str>> {
        let (sequence, snapshot) = {
            let mut state = self.lock_state();
            let now = self.now_ms();
            state.take_snapshot(now)
        };
        let message: Arc<str> = to_json(&snapshot).into();
        lock(&self.feeds).snapshots.add(sequence, message)
    }

    pub(super) fn subscribe_control(&self) -> Receiver<Arc<ControlView>> {
        let (sequence, view) = {
            let mut state = self.lock_state();
            let now = self.now_ms();
            state.take_control_view(now)
        };
        lock(&self.feeds).controls.add(sequence, view)
    }

    pub(super) fn add_listener(&self, listener: Box<dyn Fn(&HubEvent) + Send + Sync>) {
        self.listeners.add(listener);
    }

    fn mark_dirty(&self) {
        let stopped = {
            let mut schedule = self.lock_schedule();
            schedule.mark_dirty(Instant::now(), self.config.save_debounce);
            schedule.stopped
        };
        if stopped {
            // A failure is reported to listeners; there is no later retry once stopped.
            let _ = self.save_now();
        } else {
            self.wake.notify_all();
        }
    }

    /// Saves the project if it has unsaved changes; saves never overlap or reorder.
    ///
    /// The first failure of a streak is logged for the editor and sent to listeners as
    /// [`HubEvent::SaveFailed`]; the save is retried later. Returns the failure, if any.
    pub(super) fn save_now(&self) -> Result<(), String> {
        let _serial = lock(&self.save_lock);
        if !self.lock_schedule().take_dirty() {
            return Ok(());
        }
        let Some(path) = self.config.project_path.as_deref() else {
            return Ok(());
        };
        let project = self.lock_state().engine.project().clone();
        match save_project(path, &project) {
            Ok(()) => {
                if self.save_failing.swap(false, Ordering::SeqCst) {
                    self.note("saved the project again");
                }
                Ok(())
            }
            Err(error) => {
                let message = format!("could not save the project to {}: {error}", path.display());
                let retry = Instant::now() + SAVE_RETRY.max(self.config.save_debounce);
                self.lock_schedule().retry_save(retry);
                self.wake.notify_all();
                if !self.save_failing.swap(true, Ordering::SeqCst) {
                    self.note(&message);
                    self.listeners.queue([HubEvent::SaveFailed(message.clone())]);
                    self.listeners.deliver();
                }
                Err(message)
            }
        }
    }

    /// Tells listeners that changes are lost because the final save failed.
    pub(super) fn final_save_failed(&self, message: &str) {
        let lost = format!("{message}; the last changes are lost");
        self.listeners.queue([HubEvent::SaveFailed(lost)]);
        self.listeners.deliver();
    }

    /// Whether project changes are waiting to be saved.
    pub(super) fn unsaved(&self) -> bool {
        self.lock_schedule().is_dirty()
    }

    fn note(&self, detail: &str) {
        let Ok(_) = self.apply(|engine, now| Ok::<_, Infallible>(engine.note("system", detail, now)));
    }

    pub(super) fn stop(&self) {
        self.lock_schedule().stopped = true;
        self.wake.notify_all();
    }

    /// Disconnects subscribers and drops listeners, which also breaks reference cycles
    /// through listeners that own the hub.
    pub(super) fn close(&self) {
        lock(&self.feeds).close();
        self.listeners.close();
    }
}
