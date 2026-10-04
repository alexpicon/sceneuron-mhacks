// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What the server needs from the shared room client: the room's state for the editor, and the
//! actions the editor and control surfaces take in it.

use serde::Deserialize;
use serde_json::{Value, json};

const NOT_AVAILABLE: &str = "the shared room is not available in this build";

/// A refusal from the room, answered with this HTTP status and message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomError {
    pub status: u16,
    pub message: String,
}

impl RoomError {
    pub fn new(status: u16, message: impl Into<String>) -> RoomError {
        RoomError {
            status,
            message: message.into(),
        }
    }
}

/// What a request to the room does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoomAction {
    /// Creates a room with a fresh code and joins it.
    Create,
    /// Joins the room with the given code.
    Join,
    /// Joins the room with a code of one's choosing, such as CHASKI, creating it when nobody has it.
    Open,
    /// Brings simulated teammates into the room this Sceneuron is in, to practice or demo alone.
    Practice,
    /// Takes the simulated teammates out of the room again.
    #[serde(rename = "practice-stop")]
    PracticeStop,
    Leave,
    /// Shows everyone's votes at once.
    Reveal,
    /// Lowers every card and clears the votes for the next round.
    Next,
}

/// A request to the room: the action and the fields it uses. Fields an action does not use are
/// ignored; missing ones take the room client's defaults.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomRequest {
    pub action: RoomAction,
    /// The room code to join.
    #[serde(default)]
    pub code: Option<String>,
    /// The display name others see.
    #[serde(default)]
    pub name: Option<String>,
    /// The id of the local deck whose cards vote.
    #[serde(default)]
    pub deck: Option<String>,
    /// The SpacetimeDB server's address.
    #[serde(default)]
    pub server: Option<String>,
    /// The SpacetimeDB database holding the rooms.
    #[serde(default)]
    pub database: Option<String>,
}

/// The shared room behind `/api/room`.
///
/// The server calls these from its request threads, outside every hub lock; an action may wait
/// several seconds for SpacetimeDB.
pub trait Room: Send + Sync {
    /// The room as JSON for the editor.
    fn state(&self) -> Value;

    /// Runs the request's action and returns the new state.
    fn act(&self, request: RoomRequest) -> Result<Value, RoomError>;
}

/// The room of a build without one: its state says it is unavailable.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullRoom;

impl Room for NullRoom {
    fn state(&self) -> Value {
        json!({ "available": false })
    }

    fn act(&self, _request: RoomRequest) -> Result<Value, RoomError> {
        Err(RoomError::new(400, NOT_AVAILABLE))
    }
}
