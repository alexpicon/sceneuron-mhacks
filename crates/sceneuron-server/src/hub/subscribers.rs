// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Bounded channels of published snapshots and control views that never deliver an older one
//! after a newer one.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

use crate::control_view::ControlView;

/// Messages a subscriber may have queued before it counts as stalled and is dropped.
const CAPACITY: usize = 32;

/// Snapshot subscribers and control view subscribers, published to together.
#[derive(Default)]
pub(super) struct Feeds {
    pub(super) snapshots: Subscribers<Arc<str>>,
    pub(super) controls: Subscribers<Arc<ControlView>>,
}

impl Feeds {
    pub(super) fn close(&mut self) {
        self.snapshots.close();
        self.controls.close();
    }
}

struct Subscriber<T> {
    sender: SyncSender<T>,
    /// Sequence number of the newest message this subscriber was sent.
    last_sequence: u64,
}

pub(super) struct Subscribers<T> {
    list: Vec<Subscriber<T>>,
    /// The newest published message, for subscribers that join while it is in flight.
    latest: Option<(u64, T)>,
    closed: bool,
}

impl<T> Default for Subscribers<T> {
    fn default() -> Self {
        Self {
            list: Vec::new(),
            latest: None,
            closed: false,
        }
    }
}

impl<T: Clone> Subscribers<T> {
    /// Sends a message to every subscriber that has not seen a newer one, dropping
    /// subscribers whose channel is full or closed.
    pub(super) fn publish(&mut self, sequence: u64, message: &T) {
        if self.latest.as_ref().is_none_or(|(latest, _)| sequence > *latest) {
            self.latest = Some((sequence, message.clone()));
        }
        self.list.retain_mut(|subscriber| {
            if sequence <= subscriber.last_sequence {
                return true;
            }
            let sent = subscriber.sender.try_send(message.clone()).is_ok();
            if sent {
                subscriber.last_sequence = sequence;
            }
            sent
        });
    }

    /// Registers a subscriber whose first message is the newer of `message` and the
    /// latest published one. After `close`, the receiver gets that one message and then
    /// disconnects.
    pub(super) fn add(&mut self, sequence: u64, message: T) -> Receiver<T> {
        let (first_sequence, first) = match &self.latest {
            Some((latest, newer)) if *latest > sequence => (*latest, newer.clone()),
            _ => (sequence, message),
        };
        let (sender, receiver) = sync_channel(CAPACITY);
        // A new channel with capacity left always accepts its first message.
        let _ = sender.try_send(first);
        if !self.closed {
            self.list.push(Subscriber {
                sender,
                last_sequence: first_sequence,
            });
        }
        receiver
    }

    fn close(&mut self) {
        self.closed = true;
        self.list.clear();
    }
}
