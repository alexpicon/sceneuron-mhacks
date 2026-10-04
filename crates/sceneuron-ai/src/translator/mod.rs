// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The subtitle translator: a thread that translates each final subtitle segment once, into every
//! target in one request, with the AI provider the settings choose and its key, and reports the
//! answer or why there is none by the segment's id. Its spend counts toward the hourly budget, and
//! while the budget is spent nothing is sent.
//!
//! Only final text goes out, and only while subtitles are on, agreed to, and have targets, for the
//! targets set when it goes out. What it reports never quotes the line or the provider's message,
//! so a failure in the event log cannot carry what was said.

mod request;

use std::collections::{BTreeMap, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};

use sceneuron_core::TranslationJob;

use crate::cost::{cost_usd, prices_for};
use crate::director::{Ledger, Sink};
use crate::provider::{AiError, Connector, Provider};
use crate::settings::AiSettings;
pub use request::{TRANSLATE_PROMPT, parse_translations, translation_request, translation_schema};

/// Jobs that may wait while one is translated; older ones are dropped, so live subtitles stay
/// current when the provider falls behind.
const MAX_WAITING: usize = 3;
const FELL_BEHIND: &str = "translations fell behind, so a line was skipped";

#[derive(Default)]
struct Inbox {
    jobs: VecDeque<TranslationJob>,
    settings: Option<AiSettings>,
    shutdown: bool,
}

#[derive(Default)]
struct Shared {
    inbox: Mutex<Inbox>,
    wake: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, Inbox> {
        self.inbox.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn post(&self, change: impl FnOnce(&mut Inbox)) {
        change(&mut self.lock());
        self.wake.notify_one();
    }
}

/// Handle to the translator thread.
pub(crate) struct Translator {
    shared: Arc<Shared>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Translator {
    /// Starts the thread, named `sceneuron-translate`, with `settings`, keys included; spend goes
    /// to `ledger` and results to `sink`.
    pub(crate) fn spawn(settings: AiSettings, connector: Connector, ledger: Ledger, sink: Arc<dyn Sink>) -> Translator {
        let shared = Arc::new(Shared::default());
        let worker = Worker {
            settings,
            connector,
            ledger,
            sink,
            provider: None,
        };
        let inbox = Arc::clone(&shared);
        let thread = thread::Builder::new()
            .name("sceneuron-translate".to_owned())
            .spawn(move || worker.run(&inbox))
            .map_err(|error| log::error!("subtitles: cannot start the translator: {error}"))
            .ok();
        Translator {
            shared,
            thread: Mutex::new(thread),
        }
    }

    /// Queues a segment to translate without waiting.
    pub(crate) fn translate(&self, job: TranslationJob) {
        self.shared.post(|inbox| inbox.jobs.push_back(job));
    }

    /// New settings, keys included, for the jobs after the one in flight.
    pub(crate) fn set_settings(&self, settings: AiSettings) {
        self.shared.post(|inbox| inbox.settings = Some(settings));
    }

    /// Stops the thread and waits for it; a call in flight finishes first, within its timeout.
    pub(crate) fn shutdown(&self) {
        self.shared.post(|inbox| inbox.shutdown = true);
        let thread = self.thread.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(thread) = thread {
            let _ = thread.join();
        }
    }
}

struct Worker {
    settings: AiSettings,
    connector: Connector,
    ledger: Ledger,
    sink: Arc<dyn Sink>,
    /// The provider and the settings it was built from, reused while they match.
    provider: Option<(AiSettings, Arc<dyn Provider>)>,
}

impl Worker {
    fn run(mut self, shared: &Shared) {
        while let Some((job, dropped)) = self.wait(shared) {
            for skipped in dropped {
                self.sink.translation_failed(skipped.id, FELL_BEHIND);
            }
            let result = catch_unwind(AssertUnwindSafe(|| self.translate(&job)))
                .unwrap_or_else(|_| Err("the translator failed unexpectedly".to_owned()));
            match result {
                Ok(translations) => self.sink.translated(job.id, &translations),
                Err(why) => {
                    log::warn!("subtitles: a translation failed: {why}");
                    self.sink.translation_failed(job.id, &why);
                }
            }
        }
    }

    /// The next job and those dropped to reach it, once there is one; `None` on shutdown.
    fn wait(&mut self, shared: &Shared) -> Option<(TranslationJob, Vec<TranslationJob>)> {
        let mut inbox = shared.lock();
        loop {
            if inbox.shutdown {
                return None;
            }
            if let Some(settings) = inbox.settings.take() {
                self.settings = settings;
            }
            let excess = inbox.jobs.len().saturating_sub(MAX_WAITING + 1);
            let dropped: Vec<TranslationJob> = inbox.jobs.drain(..excess).collect();
            if let Some(job) = inbox.jobs.pop_front() {
                return Some((job, dropped));
            }
            inbox = shared.wake.wait(inbox).unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Translates one segment; the error says why without quoting the line or the provider.
    fn translate(&mut self, job: &TranslationJob) -> Result<BTreeMap<String, String>, String> {
        let subtitles = &self.settings.subtitles;
        if !(subtitles.enabled && subtitles.consent) {
            return Err("subtitles are off".to_owned());
        }
        if self.ledger.over_budget() {
            return Err("the hourly budget is spent".to_owned());
        }
        // A job queued before the targets changed goes out only for those still wanted, and not
        // at all once none are, as the consent promises.
        let wanted = |code: &String| subtitles.targets.iter().any(|target| target.eq_ignore_ascii_case(code));
        let targets: Vec<String> = job.targets.iter().filter(|code| wanted(code)).cloned().collect();
        if targets.is_empty() {
            return Err("no languages to translate into".to_owned());
        }
        let job = TranslationJob { targets, ..job.clone() };
        let request = translation_request(&job, &subtitles.translation_model);
        let provider = self.provider().map_err(|error| reason(&error))?;
        let reply = provider.complete(&request).map_err(|error| reason(&error))?;
        let prices = prices_for(&self.settings, &reply.model);
        let cost = prices.map_or(0.0, |prices| cost_usd(&reply.usage, &prices));
        self.ledger.record_translation(&reply.model, reply.usage, cost);
        parse_translations(&reply.json, &job.targets).map_err(|error| reason(&error))
    }

    fn provider(&mut self) -> Result<Arc<dyn Provider>, AiError> {
        if let Some((built_for, provider)) = &self.provider
            && *built_for == self.settings
        {
            return Ok(Arc::clone(provider));
        }
        let provider = (self.connector)(&self.settings)?;
        self.provider = Some((self.settings.clone(), Arc::clone(&provider)));
        Ok(provider)
    }
}

/// Why a translation failed, in Sceneuron's own words: a provider's message may quote the request.
fn reason(error: &AiError) -> String {
    let reason = match error {
        AiError::Config(message) => return message.clone(),
        AiError::Auth(_) => "the AI provider rejected the API key",
        AiError::BadRequest(_) => "the AI provider rejected the request; check the translation model",
        AiError::RateLimited(_) => "the AI provider asked to slow down",
        AiError::Unavailable(_) => "the AI provider is unavailable",
        AiError::Refused(_) => "the model declined to translate",
        AiError::Invalid(_) => "the model's answer could not be used",
    };
    reason.to_owned()
}
