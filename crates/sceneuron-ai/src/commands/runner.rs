// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The voice commands thread: it carries out jobs one at a time, in order, so a model call never
//! holds up the thread that heard the words, and the assistant can stop it before its code is
//! unloaded.

use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};

type Job = Box<dyn FnOnce() + Send>;

#[derive(Default)]
struct Inbox {
    jobs: VecDeque<Job>,
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

/// Handle to the voice commands thread.
pub(crate) struct Runner {
    shared: Arc<Shared>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Runner {
    /// Starts the thread, named `sceneuron-voice-ai`.
    pub(crate) fn spawn() -> Runner {
        let shared = Arc::new(Shared::default());
        let inbox = Arc::clone(&shared);
        let thread = thread::Builder::new()
            .name("sceneuron-voice-ai".to_owned())
            .spawn(move || run(&inbox))
            .map_err(|error| log::error!("voice: cannot start the voice commands thread: {error}"))
            .ok();
        Runner {
            shared,
            thread: Mutex::new(thread),
        }
    }

    /// Queues a job without waiting; after shutdown it is dropped.
    #[cfg(feature = "hub")]
    pub(crate) fn post(&self, job: Job) {
        self.shared.post(|inbox| {
            if !inbox.shutdown {
                inbox.jobs.push_back(job);
            }
        });
    }

    /// Stops the thread and waits for it, unless called from it; a job in flight finishes first,
    /// within its model call's timeout, and jobs still queued are dropped.
    pub(crate) fn shutdown(&self) {
        self.shared.post(|inbox| {
            inbox.shutdown = true;
            inbox.jobs.clear();
        });
        let thread = self.thread.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(thread) = thread
            && thread.thread().id() != thread::current().id()
        {
            let _ = thread.join();
        }
    }
}

impl Drop for Runner {
    /// Asks the thread to stop without waiting for it, since the last owner may be a job on it.
    fn drop(&mut self) {
        self.shared.post(|inbox| inbox.shutdown = true);
    }
}

fn run(shared: &Shared) {
    loop {
        let job = {
            let mut inbox = shared.lock();
            loop {
                if inbox.shutdown {
                    return;
                }
                if let Some(job) = inbox.jobs.pop_front() {
                    break job;
                }
                inbox = shared.wake.wait(inbox).unwrap_or_else(PoisonError::into_inner);
            }
        };
        if catch_unwind(AssertUnwindSafe(job)).is_err() {
            log::error!("voice: a voice command failed unexpectedly");
        }
    }
}
