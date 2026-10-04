// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The open connections: their limit, making room by closing idle ones, and closing every
//! one of them when the server stops.

use std::collections::HashMap;
use std::net::{Shutdown, TcpStream};
use std::sync::{Arc, Mutex};

use crate::lock;

/// Most connections open at once, event streams included; each holds a thread.
const MAX_CONNECTIONS: usize = 64;

#[derive(Debug, Default)]
pub(crate) struct Connections {
    state: Mutex<State>,
}

#[derive(Debug, Default)]
struct State {
    next_id: u64,
    /// Every open connection by id; ids grow, so a smaller id is an older connection.
    open: HashMap<u64, Slot>,
    closed: bool,
}

#[derive(Debug)]
struct Slot {
    /// A second handle to the socket, so the server can shut it down from outside the
    /// thread that serves it.
    stream: TcpStream,
    /// The client sent a complete request head, so it is a real request or event stream.
    started: bool,
}

impl Connections {
    /// Takes a place for a new connection, or `None` when the server is stopping or every
    /// place belongs to a connection that already sent its request head.
    ///
    /// At the limit, the oldest connection still waiting for its head is closed to make
    /// room, so idle sockets cannot lock out the editor and overlays.
    pub(crate) fn register(self: &Arc<Self>, stream: &TcpStream) -> Option<Entry> {
        let mut state = lock(&self.state);
        if state.closed {
            return None;
        }
        if state.open.len() >= MAX_CONNECTIONS {
            let idle = state
                .open
                .iter()
                .filter(|(_, slot)| !slot.started)
                .map(|(id, _)| *id)
                .min()?;
            if let Some(slot) = state.open.remove(&idle) {
                let _ = slot.stream.shutdown(Shutdown::Both);
            }
        }
        let handle = stream.try_clone().ok()?;
        let id = state.next_id;
        state.next_id += 1;
        let slot = Slot {
            stream: handle,
            started: false,
        };
        state.open.insert(id, slot);
        Some(Entry {
            connections: Arc::clone(self),
            id,
        })
    }

    pub(crate) fn count(&self) -> usize {
        lock(&self.state).open.len()
    }

    pub(crate) fn started(&self) -> usize {
        lock(&self.state).open.values().filter(|slot| slot.started).count()
    }

    /// Refuses new connections and shuts down every open socket, which wakes the threads
    /// blocked reading from or writing to them.
    pub(crate) fn close_all(&self) {
        let mut state = lock(&self.state);
        state.closed = true;
        for slot in state.open.values() {
            let _ = slot.stream.shutdown(Shutdown::Both);
        }
    }
}

/// A connection's place; dropping it frees the place and the second socket handle.
#[derive(Debug)]
pub(crate) struct Entry {
    connections: Arc<Connections>,
    id: u64,
}

impl Entry {
    /// Records that the connection sent its request head, so it is no longer idle.
    pub(crate) fn mark_started(&self) {
        if let Some(slot) = lock(&self.connections.state).open.get_mut(&self.id) {
            slot.started = true;
        }
    }
}

impl Drop for Entry {
    fn drop(&mut self) {
        lock(&self.connections.state).open.remove(&self.id);
    }
}
