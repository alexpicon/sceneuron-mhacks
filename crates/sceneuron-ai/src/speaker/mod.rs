// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Spoken cards: a background thread that turns speech cues into cached audio.
//!
//! A cue is spoken only while spoken cards are on and an ElevenLabs key is set, in a voice the
//! account has (see [`worker`]). Audio already in the cache costs nothing; new audio is
//! generated only while the hourly budget allows, and its characters count toward the totals
//! and the budget. Only the newest cue waits: a newer one replaces a cue that was not picked up
//! yet, because its card is no longer up.

mod worker;

use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};

use crate::director::{Ledger, Sink};
use crate::settings::AiSettings;
use crate::speech_cache::SpeechCache;
use worker::Worker;

/// Handle to the speaker thread.
pub(crate) struct Speaker {
    shared: Arc<Shared>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

struct Shared {
    inbox: Mutex<Inbox>,
    wake: Condvar,
}

#[derive(Default)]
struct Inbox {
    settings: Option<AiSettings>,
    job: Option<Job>,
    shutdown: bool,
}

struct Job {
    nonce: u64,
    text: String,
}

impl Speaker {
    /// Starts the thread, named `sceneuron-speaker`, with `settings` including keys from the
    /// environment. Audio goes to `cache`, and requests to `base_url`.
    pub(crate) fn spawn(
        settings: AiSettings,
        cache: SpeechCache,
        base_url: &str,
        ledger: Ledger,
        sink: Arc<dyn Sink>,
    ) -> Speaker {
        let shared = Arc::new(Shared {
            inbox: Mutex::new(Inbox::default()),
            wake: Condvar::new(),
        });
        let mut worker = Worker::new(Arc::clone(&shared), cache, base_url, ledger, Arc::clone(&sink));
        worker.apply(settings);
        let thread_sink = Arc::clone(&sink);
        let spawned = thread::Builder::new()
            .name("sceneuron-speaker".to_owned())
            .spawn(move || {
                if panic::catch_unwind(AssertUnwindSafe(|| worker.run())).is_err() {
                    thread_sink.speech_failed("the speaker stopped unexpectedly; restart to speak cards again");
                }
            });
        let thread = match spawned {
            Ok(thread) => Some(thread),
            Err(error) => {
                sink.speech_failed(&format!("cannot start the speaker thread: {error}"));
                None
            }
        };
        Speaker {
            shared,
            thread: Mutex::new(thread),
        }
    }

    /// Switches to new settings, including keys from the environment.
    pub(crate) fn set_settings(&self, settings: AiSettings) {
        self.post(|inbox| inbox.settings = Some(settings));
    }

    /// Speaks the cue `nonce`, replacing a cue not picked up yet.
    pub(crate) fn speak(&self, nonce: u64, text: &str) {
        let text = text.to_owned();
        self.post(|inbox| inbox.job = Some(Job { nonce, text }));
    }

    /// Stops the thread and waits for it, which takes at most one request's deadline.
    pub(crate) fn shutdown(&self) {
        self.post(|inbox| inbox.shutdown = true);
        let thread = self.thread.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(thread) = thread
            && thread.thread().id() != thread::current().id()
        {
            let _ = thread.join();
        }
    }

    fn post(&self, change: impl FnOnce(&mut Inbox)) {
        change(&mut lock(&self.shared.inbox));
        self.shared.wake.notify_one();
    }
}

impl Drop for Speaker {
    /// Asks the thread to stop without waiting for it.
    fn drop(&mut self) {
        self.post(|inbox| inbox.shutdown = true);
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
