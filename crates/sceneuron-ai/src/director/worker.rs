// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The director thread: picks up inputs, tracks whether scene understanding runs, asks the gate,
//! and reports its status.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use sceneuron_core::AiStatus;

use super::Sink;
use super::cues::CueWatch;
use super::inbox::{Events, Shared, Watch};
use super::status::Totals;
use crate::gate::{Decision, Gate, Reason};
use crate::image::{Frame, Thumbnail};
use crate::provider::{AiError, Connector, Provider};
use crate::settings::AiSettings;

/// A frame older than this is not judged; the host is expected to send a newer one.
const MAX_FRAME_AGE_MS: u64 = 5_000;

/// A frame waiting to be judged, with its thumbnail and when it arrived on the director clock.
pub(super) struct Candidate {
    pub frame: Frame,
    pub thumbnail: Thumbnail,
    pub received_ms: u64,
}

pub(super) struct Worker {
    pub(super) shared: Arc<Shared>,
    connector: Connector,
    pub(super) sink: Arc<dyn Sink>,
    clock: Instant,
    pub(super) settings: AiSettings,
    pub(super) watch: Watch,
    pub(super) provider: Option<Arc<dyn Provider>>,
    /// Why the provider cannot be used yet, such as a missing key.
    pub(super) setup_error: Option<String>,
    /// A permanent error; calls stop until the settings change.
    pub(super) failure: Option<String>,
    pub(super) gate: Gate,
    pub(super) totals: Totals,
    /// Models whose spend counts as zero because their price is unknown.
    pub(super) unpriced: BTreeSet<String>,
    /// A note for the status while watching, such as a declined image or a retry.
    pub(super) note: String,
    pub(super) paused: Option<String>,
    /// The reason for the call in flight.
    pub(super) thinking: Option<&'static str>,
    pub(super) hints: Vec<String>,
    pub(super) cues: CueWatch,
    pub(super) candidate: Option<Candidate>,
    pub(super) wake_at_ms: Option<u64>,
    was_running: bool,
    last_status: Option<AiStatus>,
}

impl Worker {
    pub(super) fn new(shared: Arc<Shared>, settings: AiSettings, connector: Connector, sink: Arc<dyn Sink>) -> Worker {
        let mut worker = Worker {
            shared,
            connector,
            sink,
            clock: Instant::now(),
            settings,
            watch: Watch::default(),
            provider: None,
            setup_error: None,
            failure: None,
            gate: Gate::new(),
            totals: Totals::default(),
            unpriced: BTreeSet::new(),
            note: String::new(),
            paused: None,
            thinking: None,
            hints: Vec::new(),
            cues: CueWatch::default(),
            candidate: None,
            wake_at_ms: None,
            was_running: false,
            last_status: None,
        };
        worker.connect();
        worker
    }

    pub(super) fn run(mut self) {
        self.step();
        while let Some(events) = self.wait() {
            self.apply(events);
            self.step();
        }
    }

    pub(super) fn now_ms(&self) -> u64 {
        u64::try_from(self.clock.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// Blocks until there is news or the wake-up time passes; `None` means shut down.
    fn wait(&self) -> Option<Events> {
        let mut inbox = self.shared.lock();
        loop {
            if inbox.shutdown {
                return None;
            }
            if inbox.has_news() {
                return Some(inbox.take());
            }
            let now_ms = self.now_ms();
            inbox = match self.wake_at_ms {
                Some(at_ms) if at_ms <= now_ms => return Some(Events::default()),
                Some(at_ms) => {
                    let timeout = Duration::from_millis(at_ms - now_ms);
                    let waited = self.shared.wake.wait_timeout(inbox, timeout);
                    waited.unwrap_or_else(|poisoned| poisoned.into_inner()).0
                }
                None => {
                    let waited = self.shared.wake.wait(inbox);
                    waited.unwrap_or_else(|poisoned| poisoned.into_inner())
                }
            };
        }
    }

    fn apply(&mut self, events: Events) {
        let now_ms = self.now_ms();
        if let Some(settings) = events.settings
            && settings != self.settings
        {
            self.settings = settings;
            self.failure = None;
            self.note.clear();
            self.connect();
        }
        if let Some(watch) = events.watch {
            if watch.uncued() != self.watch.uncued() {
                self.gate.note(Reason::ConditionsChanged);
            }
            self.watch = watch;
            self.forget_cues(false);
        }
        if events.presence_changed {
            self.gate.note(Reason::PresenceChanged);
        }
        if let Some((hints, observed_ms)) = events.hints {
            self.take_hints(hints, observed_ms);
        }
        if events.novel {
            self.gate.note(Reason::Novel);
        }
        if events.analyze_now {
            self.gate.note(Reason::Asked);
        }
        for (model, usage, cost) in events.usage {
            self.account(&model, &usage, cost, now_ms);
        }
        for (_usage, cost) in events.quick_usage {
            self.totals.add_quick(cost);
            self.gate.spent(now_ms, cost);
        }
        for (usage, cost) in events.voice_usage {
            self.totals.add_voice(&usage, cost);
            self.gate.spent(now_ms, cost);
        }
        for (model, usage, cost) in events.translation_usage {
            self.totals.add_translation(&usage, cost);
            self.gate.spent(now_ms, cost);
            self.note_price(&model);
        }
        if let Some(frame) = events.frame {
            let thumbnail = Thumbnail::of(&frame);
            self.candidate = Some(Candidate {
                frame,
                thumbnail,
                received_ms: now_ms,
            });
        }
        self.keep_fresh_candidate();
    }

    fn connect(&mut self) {
        self.provider = None;
        self.setup_error = None;
        if !(self.settings.enabled && self.settings.consent) {
            return;
        }
        match (self.connector)(&self.settings) {
            Ok(provider) => self.provider = Some(provider),
            Err(AiError::Config(message)) => self.setup_error = Some(message),
            Err(error) => self.failure = Some(error.to_string()),
        }
    }

    /// Whether scene understanding is on, set up, healthy, and has something to watch.
    fn running(&self) -> bool {
        self.settings.enabled
            && self.settings.consent
            && self.watch.policy.enabled
            && self.has_work()
            && self.provider.is_some()
            && self.failure.is_none()
    }

    pub(super) fn has_work(&self) -> bool {
        self.watch.wants_caption || !self.watch.enabled().is_empty()
    }

    /// Reacts to the current state: starts or stops, and makes a call when one is due.
    fn step(&mut self) {
        self.wake_at_ms = None;
        self.paused = None;
        if self.update_running()
            && let Some(decision) = self.decide()
        {
            match decision {
                Decision::Call { reason } => self.call(reason),
                Decision::Wait { until_ms } => self.wake_at_ms = Some(until_ms),
                Decision::Paused { message, until_ms } => {
                    self.paused = Some(message);
                    self.wake_at_ms = Some(until_ms);
                }
            }
        }
        self.update_running();
        self.shared.budget_spent.store(self.paused.is_some(), Ordering::Relaxed);
        self.watch_budget();
        self.publish();
    }

    /// Tracks starts and stops; a stop after running resets the host's conditions. Frames are
    /// wanted only while a look would ask something.
    fn update_running(&mut self) -> bool {
        let running = self.running();
        let wants_frames = running && self.has_question();
        self.shared.active.store(wants_frames, Ordering::Relaxed);
        if running && !self.was_running {
            self.gate.restart();
            self.note.clear();
        }
        if !running && self.was_running {
            self.publish();
            let (_, why) = self.state();
            self.sink.reset(&why);
            self.forget_cues(true);
        }
        self.was_running = running;
        running
    }

    /// The gate's decision for the newest unjudged frame. Without a recent frame, or with nothing
    /// to ask, no call is made, but a spent budget still shows as a pause.
    fn decide(&self) -> Option<Decision> {
        let now_ms = self.now_ms();
        let recent = self
            .candidate
            .as_ref()
            .filter(|candidate| now_ms.saturating_sub(candidate.received_ms) <= MAX_FRAME_AGE_MS)
            .filter(|_| self.has_question());
        match recent {
            Some(candidate) => Some(self.gate.decide(&self.watch.policy, now_ms, &candidate.thumbnail)),
            None => self.gate.pause(&self.watch.policy, now_ms),
        }
    }

    /// Sends the status when it differs from the last one sent.
    pub(super) fn publish(&mut self) {
        let status = self.status(self.now_ms());
        if self.last_status.as_ref() != Some(&status) {
            self.last_status = Some(status.clone());
            self.sink.status(status);
        }
    }
}
