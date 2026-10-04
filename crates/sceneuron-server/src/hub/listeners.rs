// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Listener delivery in the order changes happened.
//!
//! Events are queued while the engine lock is held, so the queue order is the change order.
//! Whichever thread delivers next drains the queue under a delivery lock, so listeners never
//! see two changes reversed even when the changes came from different threads.

use std::cell::Cell;
use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex};

use super::HubEvent;
use crate::lock;

type Listener = dyn Fn(&HubEvent) + Send + Sync;

thread_local! {
    /// This thread is inside a listener; a change it makes is delivered by the outer loop.
    static DELIVERING: Cell<bool> = const { Cell::new(false) };
}

#[derive(Default)]
pub(super) struct Listeners {
    list: Mutex<Vec<Arc<Listener>>>,
    pending: Mutex<VecDeque<HubEvent>>,
    delivery: Mutex<()>,
}

impl Listeners {
    pub(super) fn add(&self, listener: Box<Listener>) {
        lock(&self.list).push(Arc::from(listener));
    }

    /// Queues events for delivery; callers hold the engine lock so the order is the change order.
    pub(super) fn queue(&self, events: impl IntoIterator<Item = HubEvent>) {
        lock(&self.pending).extend(events);
    }

    /// Delivers every queued event in order, outside the engine lock.
    pub(super) fn deliver(&self) {
        if DELIVERING.get() {
            return;
        }
        let _order = lock(&self.delivery);
        DELIVERING.set(true);
        let _reset = Reset;
        loop {
            let Some(event) = lock(&self.pending).pop_front() else {
                break;
            };
            let listeners = lock(&self.list).clone();
            for listener in &listeners {
                // A listener bug must not abort the change that is already applied.
                let _ = catch_unwind(AssertUnwindSafe(|| listener(&event)));
            }
        }
    }

    /// Drops every listener, which also breaks reference cycles through listeners that own
    /// the hub.
    pub(super) fn close(&self) {
        let listeners = std::mem::take(&mut *lock(&self.list));
        drop(listeners);
        lock(&self.pending).clear();
    }
}

/// Clears the delivering flag even if delivery unwinds.
struct Reset;

impl Drop for Reset {
    fn drop(&mut self) {
        DELIVERING.set(false);
    }
}
