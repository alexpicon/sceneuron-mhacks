// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What a connection's thread shares with its callers: the messages waiting to go out, the calls
//! waiting for an answer, whether the connection sees the rooms yet, and whether the thread ended.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde_json::Value;

use super::protocol;
use crate::backend::{BackendEvent, CallError, Done, Events};

pub(super) struct Link {
    pub(super) events: Events,
    /// Messages for the thread to send, in order.
    outbox: Mutex<Vec<String>>,
    pending: Mutex<BTreeMap<u32, Done>>,
    next_request: AtomicU32,
    pub(super) closed: AtomicBool,
    /// How connecting ended; `None` while the connection is still being made.
    ready: Mutex<Option<Result<(), String>>>,
    connected: Condvar,
    /// The thread ended.
    finished: Mutex<bool>,
    ended: Condvar,
}

/// Marks the thread as ended however it ends, so a host that unloads this code can wait for it,
/// and fails whatever still waits for it.
pub(super) struct Finish(pub(super) Arc<Link>);

impl Drop for Finish {
    fn drop(&mut self) {
        let link = &self.0;
        link.set_ready(Err("the connection ended".to_owned()));
        link.fail_pending("the connection to the room ended");
        *lock(&link.finished) = true;
        link.ended.notify_all();
    }
}

impl Link {
    pub(super) fn new(events: Events) -> Self {
        Self {
            events,
            outbox: Mutex::new(Vec::new()),
            pending: Mutex::new(BTreeMap::new()),
            next_request: AtomicU32::new(1),
            closed: AtomicBool::new(false),
            ready: Mutex::new(None),
            connected: Condvar::new(),
            finished: Mutex::new(false),
            ended: Condvar::new(),
        }
    }

    pub(super) fn request_id(&self) -> u32 {
        self.next_request.fetch_add(1, Ordering::SeqCst)
    }

    pub(super) fn queue(&self, message: String) {
        lock(&self.outbox).push(message);
    }

    pub(super) fn take_outbox(&self) -> Vec<String> {
        std::mem::take(&mut *lock(&self.outbox))
    }

    /// Queues a call of `reducer`; `done` gets its outcome once the server answers.
    pub(super) fn call(&self, reducer: &str, args: &Value, done: Done) {
        if self.closed.load(Ordering::SeqCst) {
            return done(Err(CallError::Failed("the room connection is closed".to_owned())));
        }
        let id = self.request_id();
        lock(&self.pending).insert(id, done);
        self.queue(protocol::call_reducer(reducer, args, id));
        // Closing fails the calls it finds; one added while it ran is failed here.
        if self.closed.load(Ordering::SeqCst) {
            self.finish(id, Err(CallError::Failed("the room connection is closed".to_owned())));
        }
    }

    pub(super) fn finish(&self, id: u32, outcome: Result<(), CallError>) {
        let done = lock(&self.pending).remove(&id);
        if let Some(done) = done {
            done(outcome);
        }
    }

    pub(super) fn fail_pending(&self, reason: &str) {
        let pending = std::mem::take(&mut *lock(&self.pending));
        for done in pending.into_values() {
            done(Err(CallError::Failed(reason.to_owned())));
        }
    }

    /// Records how connecting ended; only the first outcome counts.
    pub(super) fn set_ready(&self, outcome: Result<(), String>) {
        let mut ready = lock(&self.ready);
        if ready.is_none() {
            *ready = Some(outcome);
            self.connected.notify_all();
        }
    }

    pub(super) fn is_ready(&self) -> bool {
        matches!(*lock(&self.ready), Some(Ok(())))
    }

    /// Waits at most `timeout` for the connection to see the rooms.
    pub(super) fn wait_ready(&self, timeout: Duration) -> Result<(), String> {
        let ready = lock(&self.ready);
        let (ready, _) = self
            .connected
            .wait_timeout_while(ready, timeout, |ready| ready.is_none())
            .unwrap_or_else(PoisonError::into_inner);
        ready
            .clone()
            .unwrap_or_else(|| Err(format!("no answer in {} seconds", timeout.as_secs_f64())))
    }

    /// Stops the connection: the thread ends soon after, and calls still waiting fail.
    pub(super) fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.set_ready(Err("the connection was closed".to_owned()));
        self.fail_pending("the room connection was closed");
    }

    /// The thread's last report when the connection ended without being closed: callers learn
    /// it from `ready` until the connection saw the rooms, and from an event afterwards.
    pub(super) fn lost(&self, reason: Option<String>) {
        let was_ready = self.is_ready();
        self.set_ready(Err(reason
            .clone()
            .unwrap_or_else(|| "the server closed the connection".to_owned())));
        self.fail_pending("the connection to the room ended");
        if !self.closed.swap(true, Ordering::SeqCst) && was_ready {
            (self.events)(BackendEvent::Disconnected(reason));
        }
    }

    /// Waits at most `timeout` for the thread to end; true once it did.
    pub(super) fn join(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut finished = lock(&self.finished);
        while !*finished {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            finished = self
                .ended
                .wait_timeout(finished, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        true
    }
}

pub(super) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
