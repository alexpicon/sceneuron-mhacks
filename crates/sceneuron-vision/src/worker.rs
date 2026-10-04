// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Runs a perceiver on one background thread so video threads never wait for it.
//!
//! The worker holds at most one sample: [`VisionWorker::try_submit`] drops new samples
//! while an analysis is in progress instead of queueing them, so results always describe
//! a recent frame.

use std::any::Any;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use crate::detect::{DetectError, InputSize, Perceiver};
use crate::sample::{Sample, WorkerEvent};

const STARTING: u8 = 0;
const IDLE: u8 = 1;
const BUSY: u8 = 2;
const STOPPED: u8 = 3;

struct Job {
    sample: Sample,
    accepted: Instant,
}

/// Owns the analysis thread. Dropping it stops and joins the thread.
pub struct VisionWorker {
    state: Arc<AtomicU8>,
    input: Arc<OnceLock<InputSize>>,
    jobs: Option<SyncSender<Job>>,
    thread: Option<JoinHandle<()>>,
}

impl VisionWorker {
    /// Starts the worker thread, which creates the perceiver with `make_perceiver` and then
    /// waits for samples. `on_event` runs on the worker thread, and dropping the worker
    /// waits for that thread, so `on_event` must never block on a lock held by whoever
    /// drops the worker.
    pub fn spawn<F, P>(make_perceiver: F, on_event: impl Fn(WorkerEvent) + Send + 'static) -> Self
    where
        F: FnOnce() -> Result<P, DetectError> + Send + 'static,
        P: Perceiver + 'static,
    {
        let state = Arc::new(AtomicU8::new(STARTING));
        let input = Arc::new(OnceLock::new());
        // A capacity of one never blocks: a job is only sent after the IDLE to BUSY
        // transition, and the worker returns to IDLE only after taking the job out.
        let (jobs, inbox) = sync_channel(1);
        // The setup parts are shared so they can be recovered to report a spawn failure.
        let parts = Arc::new(Mutex::new(Some((make_perceiver, on_event))));
        let thread = thread::Builder::new().name("sceneuron-vision".into()).spawn({
            let parts = Arc::clone(&parts);
            let input = Arc::clone(&input);
            let guard = StopOnExit(Arc::clone(&state));
            move || {
                let taken = parts.lock().unwrap_or_else(PoisonError::into_inner).take();
                if let Some((make_perceiver, on_event)) = taken {
                    run(make_perceiver, on_event, &inbox, &guard.0, &input);
                }
            }
        });
        let thread = match thread {
            Ok(handle) => Some(handle),
            Err(error) => {
                state.store(STOPPED, Ordering::Release);
                if let Some((_, on_event)) = parts.lock().unwrap_or_else(PoisonError::into_inner).take() {
                    on_event(WorkerEvent::Failed(format!(
                        "could not start the analysis thread: {error}"
                    )));
                }
                None
            }
        };
        VisionWorker {
            state,
            input,
            jobs: Some(jobs),
            thread,
        }
    }

    /// True when the worker stopped for good: its perceiver could not start or panicked, or it
    /// was shut down. A stopped worker never accepts samples again.
    pub fn is_stopped(&self) -> bool {
        self.state.load(Ordering::Acquire) == STOPPED
    }

    /// True when the perceiver is ready and not processing a sample.
    pub fn is_idle(&self) -> bool {
        self.state.load(Ordering::Acquire) == IDLE
    }

    /// The perceiver's [`InputSize`], known once it has been created.
    pub fn input_size(&self) -> Option<InputSize> {
        self.input.get().copied()
    }

    /// Hands `sample` to the worker without blocking. Returns false, dropping the
    /// sample, when the worker is starting, busy, failed, or stopped.
    pub fn try_submit(&self, sample: Sample) -> bool {
        if self
            .state
            .compare_exchange(IDLE, BUSY, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return false;
        }
        let Some(jobs) = &self.jobs else {
            return false;
        };
        let job = Job {
            sample,
            accepted: Instant::now(),
        };
        match jobs.try_send(job) {
            Ok(()) => true,
            // Unreachable by the IDLE to BUSY gate; the queued job returns the worker to IDLE.
            Err(TrySendError::Full(_)) => false,
            Err(TrySendError::Disconnected(_)) => {
                self.state.store(STOPPED, Ordering::Release);
                false
            }
        }
    }

    /// Stops the worker and waits for the thread. An analysis in progress finishes
    /// first, but its result is not reported. Dropping the worker does the same.
    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        self.state.store(STOPPED, Ordering::Release);
        self.jobs = None;
        if let Some(thread) = self.thread.take()
            && thread.thread().id() != thread::current().id()
        {
            // A panic escaping the worker thread was already reported by the panic hook.
            let _ = thread.join();
        }
    }
}

impl Drop for VisionWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

impl fmt::Debug for VisionWorker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = match self.state.load(Ordering::Acquire) {
            STARTING => "starting",
            IDLE => "idle",
            BUSY => "busy",
            _ => "stopped",
        };
        f.debug_struct("VisionWorker")
            .field("state", &state)
            .finish_non_exhaustive()
    }
}

/// Marks the worker stopped however its thread ends, including by unwinding out of
/// `on_event`, so `try_submit` stops accepting samples.
struct StopOnExit(Arc<AtomicU8>);

impl Drop for StopOnExit {
    fn drop(&mut self) {
        self.0.store(STOPPED, Ordering::Release);
    }
}

fn run<F, P, E>(make_perceiver: F, on_event: E, inbox: &Receiver<Job>, state: &AtomicU8, input: &OnceLock<InputSize>)
where
    F: FnOnce() -> Result<P, DetectError>,
    P: Perceiver,
    E: Fn(WorkerEvent),
{
    let mut perceiver = match catch_unwind(AssertUnwindSafe(make_perceiver)) {
        Ok(Ok(perceiver)) => perceiver,
        Ok(Err(error)) => return fail(state, &on_event, error.to_string()),
        Err(panic) => return fail(state, &on_event, panicked("starting", &*panic)),
    };
    let described = catch_unwind(AssertUnwindSafe(|| (perceiver.name(), perceiver.input_size())));
    let name = match described {
        Ok((name, size)) => {
            let _ = input.set(size);
            name
        }
        Err(panic) => return fail(state, &on_event, panicked("starting", &*panic)),
    };
    if !transition(state, STARTING, IDLE) {
        return;
    }
    on_event(WorkerEvent::Ready { perceiver: name });
    while let Ok(Job { sample, accepted }) = inbox.recv() {
        if state.load(Ordering::Acquire) == STOPPED {
            return;
        }
        let result = catch_unwind(AssertUnwindSafe(|| {
            perceiver.perceive(&sample.rgb, sample.width, sample.height)
        }));
        let event = match result {
            Ok(Ok(perception)) => WorkerEvent::Detected {
                perception,
                captured_ms: sample.captured_ms,
                latency_ms: accepted.elapsed().as_secs_f64() * 1000.0,
            },
            Ok(Err(error)) => WorkerEvent::Failed(error.to_string()),
            Err(panic) => return fail(state, &on_event, panicked("analyzing a frame", &*panic)),
        };
        if !transition(state, BUSY, IDLE) {
            return;
        }
        on_event(event);
    }
}

/// Moves `state` from `from` to `to`; false when the worker was stopped meanwhile.
fn transition(state: &AtomicU8, from: u8, to: u8) -> bool {
    state
        .compare_exchange(from, to, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
}

/// Stops the worker for good and reports why, unless it was already shut down.
fn fail(state: &AtomicU8, on_event: &impl Fn(WorkerEvent), message: String) {
    if state.swap(STOPPED, Ordering::AcqRel) != STOPPED {
        on_event(WorkerEvent::Failed(message));
    }
}

fn panicked(during: &str, payload: &(dyn Any + Send)) -> String {
    let detail = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("unknown panic");
    format!("the perceiver panicked while {during}: {detail}")
}
