// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The quick judge: a thread that asks Jev about each condition whenever the on-device facts
//! change, so overlays react in well under a second, and that asks the director for a full look
//! when Jev cannot settle a condition or a condition that reports a name turns on.
//!
//! It runs only while scene understanding is on, with consent, and with a Jev key; without the
//! key nothing changes. It never runs while the hourly budget is spent, and never judges cued
//! conditions, since facts cannot show what those wait for the vision model to see.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use sceneuron_core::Condition;

use super::Sink;
use super::inbox::{Shared, Watch};
use crate::jev::{JevClient, QuickJudgment, quick_verdict};
use crate::provider::AiError;
use crate::settings::AiSettings;

/// Shortest time between two quick calls.
const MIN_GAP: Duration = Duration::from_millis(500);
/// Longest wait after transient failures.
const MAX_BACKOFF: Duration = Duration::from_secs(30);
/// A lower score can mean that local facts missed an object, so the vision model checks it.
const FULL_LOOK_THRESHOLD: f64 = 0.7;

/// What the quick judge reports for the status.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct QuickStatus {
    pub state: &'static str,
    pub message: String,
    pub calls: u64,
    pub latency_ms: Option<f64>,
}

/// The newest inputs; facts carry the host time they were observed.
#[derive(Default)]
struct Inputs {
    settings: Option<AiSettings>,
    watch: Option<Watch>,
    facts: Option<(Vec<String>, u64)>,
    shutdown: bool,
}

/// Handle to the quick judge thread.
pub(super) struct Quick {
    inputs: Arc<(Mutex<Inputs>, Condvar)>,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
}

impl Quick {
    pub fn spawn(settings: AiSettings, director: Arc<Shared>, sink: Arc<dyn Sink>) -> Quick {
        let inputs = Arc::new((Mutex::new(Inputs::default()), Condvar::new()));
        let shared = Arc::clone(&inputs);
        let thread = thread::Builder::new()
            .name("sceneuron-quick".to_owned())
            .spawn(move || Judge::new(settings, director, sink).run(&shared))
            .ok();
        Quick {
            inputs,
            thread: Mutex::new(thread),
        }
    }

    fn post(&self, change: impl FnOnce(&mut Inputs)) {
        let (inputs, wake) = &*self.inputs;
        change(&mut inputs.lock().unwrap_or_else(PoisonError::into_inner));
        wake.notify_one();
    }

    pub fn set_settings(&self, settings: AiSettings) {
        self.post(|inputs| inputs.settings = Some(settings));
    }

    pub fn set_watch(&self, watch: Watch) {
        self.post(|inputs| inputs.watch = Some(watch));
    }

    pub fn facts(&self, facts: Vec<String>, observed_ms: u64) {
        self.post(|inputs| inputs.facts = Some((facts, observed_ms)));
    }

    /// Stops the thread and waits for it; a call in flight finishes first, within seconds.
    pub fn shutdown(&self) {
        self.post(|inputs| inputs.shutdown = true);
        let thread = self.thread.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(thread) = thread {
            let _ = thread.join();
        }
    }
}

struct Judge {
    director: Arc<Shared>,
    sink: Arc<dyn Sink>,
    settings: AiSettings,
    watch: Watch,
    client: Option<JevClient>,
    facts: Option<(Vec<String>, u64)>,
    /// Facts and conditions the last answer was about, so unchanged ones are not asked again.
    asked: Option<(Vec<String>, Vec<Condition>)>,
    /// Probabilities of the last answer, by condition id.
    last: Vec<(String, f64)>,
    failure: Option<String>,
    failures: u32,
    next_call: Instant,
    status: QuickStatus,
}

impl Judge {
    fn new(settings: AiSettings, director: Arc<Shared>, sink: Arc<dyn Sink>) -> Judge {
        let client = JevClient::new(&settings);
        Judge {
            director,
            sink,
            settings,
            watch: Watch::default(),
            client,
            facts: None,
            asked: None,
            last: Vec::new(),
            failure: None,
            failures: 0,
            next_call: Instant::now(),
            status: QuickStatus::default(),
        }
    }

    fn run(mut self, inputs: &(Mutex<Inputs>, Condvar)) {
        self.report();
        loop {
            {
                let (lock, wake) = inputs;
                let mut guard = lock.lock().unwrap_or_else(PoisonError::into_inner);
                loop {
                    if guard.shutdown {
                        return;
                    }
                    if has_news(&guard) {
                        break;
                    }
                    if self.due() {
                        let wait = self.next_call.saturating_duration_since(Instant::now());
                        if wait.is_zero() {
                            break;
                        }
                        guard = wake.wait_timeout(guard, wait).unwrap_or_else(PoisonError::into_inner).0;
                    } else {
                        guard = wake.wait(guard).unwrap_or_else(PoisonError::into_inner);
                    }
                }
                self.take(&mut guard);
            }
            if self.due() && Instant::now() >= self.next_call {
                self.ask();
            }
            self.report();
        }
    }

    fn take(&mut self, inputs: &mut MutexGuard<'_, Inputs>) {
        if let Some(settings) = inputs.settings.take()
            && settings != self.settings
        {
            self.client = JevClient::new(&settings);
            self.settings = settings;
            self.failure = None;
            self.failures = 0;
            self.asked = None;
        }
        if let Some(watch) = inputs.watch.take() {
            self.watch = watch;
        }
        if let Some(facts) = inputs.facts.take() {
            self.facts = Some(facts);
        }
    }

    fn active(&self) -> bool {
        self.settings.enabled
            && self.settings.consent
            && self.watch.policy.enabled
            && !self.watch.uncued().is_empty()
            && self.client.is_some()
            && self.failure.is_none()
            && !self.director.budget_spent.load(Ordering::Relaxed)
    }

    /// Whether there are new facts or conditions to ask about.
    fn due(&self) -> bool {
        let Some((facts, _)) = &self.facts else {
            return false;
        };
        let conditions: Vec<Condition> = self.watch.uncued().into_iter().cloned().collect();
        self.active() && self.asked.as_ref() != Some(&(facts.clone(), conditions))
    }

    fn ask(&mut self) {
        let Some((facts, observed_ms)) = self.facts.clone() else {
            return;
        };
        let conditions: Vec<Condition> = self.watch.uncued().into_iter().cloned().collect();
        let refs: Vec<&Condition> = conditions.iter().collect();
        self.next_call = Instant::now() + MIN_GAP;
        let Some(result) = self.client.as_ref().map(|client| client.judge(&refs, &facts)) else {
            return;
        };
        match result {
            Ok(judgment) => {
                self.failures = 0;
                self.asked = Some((facts, conditions.clone()));
                self.answered(&judgment, &conditions, observed_ms);
            }
            Err(error) if error.is_permanent() => self.failure = Some(format!("quick judgments stopped: {error}")),
            Err(error) => self.retry_later(&error),
        }
    }

    fn answered(&mut self, judgment: &QuickJudgment, conditions: &[Condition], observed_ms: u64) {
        let verdicts: Vec<_> = judgment
            .probabilities
            .iter()
            .filter(|(_, probability)| *probability >= 0.5)
            .map(|(id, probability)| quick_verdict(id, *probability))
            .collect();
        if !verdicts.is_empty() {
            self.sink.verdicts(verdicts, observed_ms);
        }
        let needs_look = judgment
            .probabilities
            .iter()
            .any(|(_, p)| *p < FULL_LOOK_THRESHOLD);
        let needs_name = judgment.probabilities.iter().any(|(id, p)| {
            let was = self.last.iter().find(|(known, _)| known == id).map_or(0.0, |(_, p)| *p);
            let reports_name = conditions.iter().any(|c| &c.id == id && !c.detail.is_empty());
            reports_name && *p >= 0.5 && was < 0.5
        });
        self.last.clone_from(&judgment.probabilities);
        let cost = judgment.cost_usd();
        let usage = judgment.usage;
        self.director.post(|inbox| {
            inbox.quick_usage.push((usage, cost));
            inbox.novel |= needs_look || needs_name;
        });
        self.status.calls += 1;
        self.status.latency_ms = Some((judgment.latency_ms * 10.0).round() / 10.0);
        self.status.message = if needs_look || needs_name {
            "asked the vision model to take a closer look".to_owned()
        } else {
            String::new()
        };
    }

    fn retry_later(&mut self, error: &AiError) {
        self.failures += 1;
        let backoff = Duration::from_secs(1 << self.failures.min(5)).min(MAX_BACKOFF);
        self.next_call = Instant::now() + backoff;
        self.status.message = format!("retrying in {} s: {error}", backoff.as_secs());
    }

    /// Updates the shared status and wakes the director to publish it when it changed.
    fn report(&mut self) {
        let (state, message) = if self.client.is_none() {
            (
                "off",
                "add a Jev key for quick judgments from on-device facts".to_owned(),
            )
        } else if let Some(failure) = &self.failure {
            ("failed", failure.clone())
        } else if self.active() {
            ("watching", self.status.message.clone())
        } else {
            ("off", "quick judgments run while scene understanding is on".to_owned())
        };
        let next = QuickStatus {
            state,
            message,
            ..self.status.clone()
        };
        let mut shared = self.director.quick.lock().unwrap_or_else(PoisonError::into_inner);
        if *shared != next {
            *shared = next;
            drop(shared);
            self.director.post(|inbox| inbox.quick_changed = true);
        }
    }
}

fn has_news(inputs: &Inputs) -> bool {
    inputs.shutdown || inputs.settings.is_some() || inputs.watch.is_some() || inputs.facts.is_some()
}
