// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The seam between the room logic and SpacetimeDB: what a connection to the rooms can do, and
//! what it reports. [`crate::SpacetimeConnector`] is the real one, a client of SpacetimeDB's
//! WebSocket API; tests use a fake.

use std::sync::Arc;
use std::time::Duration;

use crate::target::Target;

/// What a participant sees of the room they are in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RoomView {
    pub code: String,
    /// The deck the room was created for, as its creator's Sceneuron names it.
    pub deck_name: String,
    pub round: u32,
    pub revealed: bool,
    /// Everyone in the room, in the order they joined.
    pub members: Vec<Member>,
    /// Each vote of this round, once revealed.
    pub results: Vec<Shown>,
}

/// Someone in the room.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    /// Their SpacetimeDB identity, which is public in the room's tables.
    pub id: String,
    pub name: String,
    /// They voted this round; what they voted stays hidden until the reveal.
    pub voted: bool,
    /// This connection.
    pub me: bool,
}

/// A revealed vote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shown {
    /// The voter's identity, as in [`Member::id`].
    pub id: String,
    /// The card id in the voter's deck.
    pub card: String,
}

impl RoomView {
    /// The card this connection voted, once revealed.
    pub fn my_result(&self) -> Option<&str> {
        let me = self.members.iter().find(|member| member.me)?;
        self.results
            .iter()
            .find(|shown| shown.id == me.id)
            .map(|shown| shown.card.as_str())
    }
}

/// A reducer call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Call {
    Create {
        deck_name: String,
        name: String,
    },
    Join {
        code: String,
        name: String,
    },
    /// Joins the room with a chosen code, creating it when nobody has it.
    Open {
        code: String,
        deck_name: String,
        name: String,
    },
    Leave,
    Vote {
        card: String,
    },
    Reveal,
    NextRound,
}

/// Why a call did not succeed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallError {
    /// The module refused it, with its reason, such as a wrong room code.
    Refused(String),
    /// It never ran or its result was lost, such as when the connection ended.
    Failed(String),
}

/// What a connection reports, from its own thread and in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackendEvent {
    /// The room this connection is in changed, or `None` while it is in no room.
    Room(Option<RoomView>),
    /// The connection ended without being closed, with the reason when there is one.
    Disconnected(Option<String>),
}

pub type Events = Box<dyn Fn(BackendEvent) + Send + Sync>;

pub type Done = Box<dyn FnOnce(Result<(), CallError>) + Send>;

/// One connection to the rooms.
pub trait RoomBackend: Send + Sync {
    /// Waits at most `timeout` for the connection to see the rooms; an error says why it could
    /// not. Calls are sent only once this returned `Ok`.
    fn ready(&self, _timeout: Duration) -> Result<(), String> {
        Ok(())
    }

    /// Asks the module to run `call`. `done` gets its answer, after the event that shows its
    /// effect on the room, or a failure when the connection ends first.
    fn send(&self, call: Call, done: Done);

    /// Ends the connection; it reports nothing more, and calls still waiting fail.
    fn close(&self);

    /// Waits at most `timeout` for the threads of a closed connection to end; true once they did.
    fn join(&self, _timeout: Duration) -> bool {
        true
    }
}

/// Opens connections to the rooms.
pub trait Connector: Send + Sync {
    /// Starts connecting to `target` to follow the rooms, reporting to `events` from then on, and
    /// returns the connection at once, so it can be closed while it is still being made; its
    /// [`RoomBackend::ready`] says when it sees the rooms. An error means it could not start.
    fn connect(&self, target: &Target, events: Events) -> Result<Arc<dyn RoomBackend>, String>;
}
