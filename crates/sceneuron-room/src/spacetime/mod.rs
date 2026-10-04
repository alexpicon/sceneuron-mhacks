// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The real connection to the rooms: Sceneuron's own client of SpacetimeDB's documented WebSocket
//! API, speaking its JSON text protocol. Each connection is a new anonymous identity with its own
//! thread, which subscribes to the room module's public tables, calls its reducers, and reports
//! the caller's room after every change; a reducer's answer comes after the room it changed.
//!
//! Reducers are called over the WebSocket rather than SpacetimeDB's HTTP API: an HTTP call is a
//! connection of its own, and when it ends the module's disconnect reducer would take the caller
//! out of the room.

mod cache;
mod endpoint;
mod link;
mod protocol;
mod socket;
mod tcp;
#[cfg(test)]
mod tests;
mod worker;

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

use self::link::{Finish, Link};
use crate::backend::{Call, Connector, Done, Events, RoomBackend};
use crate::target::Target;

/// Connects to SpacetimeDB over its WebSocket API, as a new anonymous identity each time.
#[derive(Clone, Copy, Debug)]
pub struct SpacetimeConnector {
    /// How long a connection may hear nothing from the server before it counts as lost, as after
    /// a computer slept or its network changed; after a third of it, the connection pings the
    /// server. 45 seconds by default, three of the pings SpacetimeDB sends every 15 seconds.
    pub idle_limit: Duration,
}

impl Default for SpacetimeConnector {
    fn default() -> Self {
        Self {
            idle_limit: worker::IDLE_LIMIT,
        }
    }
}

impl Connector for SpacetimeConnector {
    fn connect(&self, target: &Target, events: Events) -> Result<Arc<dyn RoomBackend>, String> {
        let link = Arc::new(Link::new(events));
        let worker = Arc::clone(&link);
        let target = target.clone();
        let idle_limit = self.idle_limit;
        thread::Builder::new()
            .name("sceneuron-room".to_owned())
            .spawn(move || {
                let _finish = Finish(Arc::clone(&worker));
                worker::run(&worker, &target, idle_limit);
            })
            .map_err(|error| format!("cannot start the room thread: {error}"))?;
        Ok(Arc::new(SpacetimeBackend(link)))
    }
}

struct SpacetimeBackend(Arc<Link>);

impl RoomBackend for SpacetimeBackend {
    fn ready(&self, timeout: Duration) -> Result<(), String> {
        self.0.wait_ready(timeout.min(worker::CONNECT_TIMEOUT))
    }

    fn send(&self, call: Call, done: Done) {
        let (reducer, args) = reducer_call(call);
        self.0.call(reducer, &args, done);
    }

    fn close(&self) {
        self.0.close();
    }

    fn join(&self, timeout: Duration) -> bool {
        self.0.join(timeout)
    }
}

/// The reducer of `room/module` that runs `call`, with its arguments.
fn reducer_call(call: Call) -> (&'static str, Value) {
    match call {
        Call::Create { deck_name, name } => ("create_room", json!([deck_name, name])),
        Call::Join { code, name } => ("join_room", json!([code, name])),
        Call::Open { code, deck_name, name } => ("open_room", json!([code, deck_name, name])),
        Call::Leave => ("leave_room", json!([])),
        Call::Vote { card } => ("cast_vote", json!([card])),
        Call::Reveal => ("reveal", json!([])),
        Call::NextRound => ("next_round", json!([])),
    }
}
