// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The AI director: a background thread that decides when to ask the vision model about the
//! camera, asks it, and reports what it saw and what it is doing.
//!
//! Hosts talk to it through cheap, non-blocking calls: frames go into a one-slot mailbox where
//! the latest frame wins, and every other input is recorded and picked up by the thread. The
//! thread reports through a [`Sink`], never while holding the director's own locks, so a sink
//! may call back into the director.

mod call;
mod cues;
mod inbox;
mod quick;
mod sink;
mod status;
mod worker;

use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread::{self, JoinHandle};

use sceneuron_core::{AiPolicy, AiStatus, Condition};

use crate::image::Frame;
use crate::provider::{Connector, Usage};
use crate::settings::AiSettings;
use crate::voice_cost::VoiceUsage;
use inbox::{Inbox, Shared, Watch};
use quick::{Quick, QuickStatus};
pub use sink::Sink;
use worker::Worker;

/// Frames are converted for the director at most this often.
const FRAME_INTERVAL_MS: u64 = 1_000;
const NO_FRAME: u64 = u64::MAX;

/// Handle to the director thread.
pub struct Director {
    shared: Arc<Shared>,
    quick: Quick,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Director {
    /// Starts the director thread, named `sceneuron-ai`. It reports its first status at once.
    ///
    /// If the thread cannot start, or stops on a bug, the sink receives a `failed` status (and
    /// a reset) and the director stays inert.
    pub fn spawn(settings: AiSettings, connector: Connector, sink: Arc<dyn Sink>) -> Director {
        let shared = Arc::new(Shared {
            inbox: Mutex::new(Inbox::default()),
            wake: Condvar::new(),
            active: AtomicBool::new(false),
            last_frame_ms: AtomicU64::new(NO_FRAME),
            budget_spent: AtomicBool::new(false),
            over_budget: AtomicBool::new(false),
            budget_watchers: Mutex::new(Vec::new()),
            quick: Mutex::new(QuickStatus::default()),
        });
        let quick = Quick::spawn(settings.clone(), Arc::clone(&shared), Arc::clone(&sink));
        let (worker_shared, worker_sink) = (Arc::clone(&shared), Arc::clone(&sink));
        let spawned = thread::Builder::new().name("sceneuron-ai".to_owned()).spawn(move || {
            let worker = Worker::new(
                Arc::clone(&worker_shared),
                settings,
                connector,
                Arc::clone(&worker_sink),
            );
            if panic::catch_unwind(AssertUnwindSafe(|| worker.run())).is_err() {
                worker_shared.active.store(false, Ordering::Relaxed);
                let message = "the AI thread stopped unexpectedly; restart OBS to use scene understanding";
                worker_sink.status(failed(message.to_owned()));
                worker_sink.reset(message);
            }
        });
        let thread = match spawned {
            Ok(thread) => Some(thread),
            Err(error) => {
                sink.status(failed(format!("cannot start the AI thread: {error}")));
                None
            }
        };
        Director {
            shared,
            quick,
            thread: Mutex::new(thread),
        }
    }

    /// Hands over a camera frame without waiting; a newer frame replaces one not yet used.
    pub fn offer_frame(&self, frame: Frame) {
        self.shared.last_frame_ms.store(frame.captured_ms, Ordering::Relaxed);
        self.shared.post(|inbox| {
            inbox.latest = Some(frame);
            inbox.fresh_frame = true;
        });
    }

    /// Whether a frame offered at `now_ms` would be used: only while scene understanding runs,
    /// and at most about once per second, so the video thread converts few large frames.
    /// `now_ms` is on the same clock as the frames' `captured_ms`.
    pub fn wants_frame(&self, now_ms: u64) -> bool {
        if !self.shared.active.load(Ordering::Relaxed) {
            return false;
        }
        let last = self.shared.last_frame_ms.load(Ordering::Relaxed);
        last == NO_FRAME || now_ms.abs_diff(last) >= FRAME_INTERVAL_MS
    }

    /// Sets what to watch: the project's conditions, its AI policy, and whether any overlay
    /// text shows the scene caption.
    pub fn set_project(&self, conditions: Vec<Condition>, policy: AiPolicy, wants_caption: bool) {
        self.set_project_with_cues(conditions, Vec::new(), policy, wants_caption);
    }

    fn set_watch(&self, watch: Watch) {
        self.quick.set_watch(watch.clone());
        self.shared.post(|inbox| inbox.watch = Some(watch));
    }

    /// Switches to new settings. A permanent failure is forgotten when the settings differ.
    pub fn set_settings(&self, settings: AiSettings) {
        self.quick.set_settings(settings.clone());
        self.shared.post(|inbox| inbox.settings = Some(settings));
    }

    /// Presence changed, which is worth a new look.
    pub fn presence_changed(&self) {
        self.shared.post(|inbox| inbox.presence_changed = true);
    }

    /// The latest facts from local detectors, observed at `observed_ms` on the host's clock:
    /// passed to the vision model as hints, to the quick judge when it has a Jev key, and to the
    /// cues of cued conditions. `novel` means a detector saw something new, worth a new look.
    pub fn hint(&self, facts: Vec<String>, novel: bool, observed_ms: u64) {
        self.quick.facts(facts.clone(), observed_ms);
        self.shared.post(|inbox| {
            inbox.hints = Some((facts, observed_ms));
            inbox.novel |= novel;
        });
    }

    /// The streamer asked for a look now; limits on rate and spend still apply.
    pub fn analyze_now(&self) {
        self.shared.post(|inbox| inbox.analyze_now = true);
    }

    /// Adds a call made elsewhere, such as by the assistant, to the totals and the hourly
    /// budget. `model` names the model that answered; the status mentions it when its price is
    /// unknown, because its spend then counts as zero.
    pub fn record_usage(&self, model: &str, usage: Usage, cost_usd: f64) {
        let model = model.to_owned();
        self.shared.post(|inbox| inbox.usage.push((model, usage, cost_usd)));
    }

    /// Adds voice usage, such as a spoken card, to the totals and the hourly budget.
    pub fn record_voice(&self, usage: VoiceUsage, cost_usd: f64) {
        self.ledger().record_voice(usage, cost_usd);
    }

    /// Whether the last hour's spend reached the project's hourly budget, whether or not scene
    /// understanding runs. Spoken cards wait, and voice control stops listening, while it has.
    pub fn over_budget(&self) -> bool {
        self.ledger().over_budget()
    }

    /// A handle for voice spend that other threads can keep.
    pub(crate) fn ledger(&self) -> Ledger {
        Ledger {
            shared: Arc::clone(&self.shared),
        }
    }

    /// The most recently offered frame, whether or not it was analyzed.
    pub fn latest_frame(&self) -> Option<Frame> {
        self.shared.lock().latest.clone()
    }

    /// Stops the thread and waits for it; later calls do nothing. When a model call is in
    /// flight it finishes first, which takes at most that call's timeout (30 seconds for scene
    /// understanding). Hosts that unload code must call this before unloading.
    pub fn shutdown(&self) {
        self.quick.shutdown();
        self.signal_shutdown();
        let thread = self.thread.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(thread) = thread
            && thread.thread().id() != thread::current().id()
        {
            let _ = thread.join();
        }
    }

    fn signal_shutdown(&self) {
        self.shared.post(|inbox| inbox.shutdown = true);
    }
}

/// Records voice and translation spend and reads the hourly budget for the director, from any
/// thread.
#[derive(Clone)]
pub(crate) struct Ledger {
    shared: Arc<Shared>,
}

impl Ledger {
    pub(crate) fn record_voice(&self, usage: VoiceUsage, cost_usd: f64) {
        self.shared.post(|inbox| inbox.voice_usage.push((usage, cost_usd)));
    }

    /// Adds a subtitle translation by `model` to the totals and the hourly budget.
    pub(crate) fn record_translation(&self, model: &str, usage: Usage, cost_usd: f64) {
        let model = model.to_owned();
        self.shared
            .post(|inbox| inbox.translation_usage.push((model, usage, cost_usd)));
    }

    pub(crate) fn over_budget(&self) -> bool {
        self.shared.over_budget.load(Ordering::Relaxed)
    }

    /// Calls `watch` on the director thread with whether the hourly budget is spent, each time
    /// that changes; `watch` must not wait for the director.
    pub(crate) fn watch(&self, watch: impl Fn(bool) + Send + Sync + 'static) {
        let mut watchers = self
            .shared
            .budget_watchers
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        watchers.push(Arc::new(watch));
    }
}

impl Drop for Director {
    /// Asks the thread to stop without waiting for it.
    fn drop(&mut self) {
        self.signal_shutdown();
    }
}

fn failed(message: String) -> AiStatus {
    AiStatus {
        state: "failed".to_owned(),
        message,
        ..AiStatus::default()
    }
}
