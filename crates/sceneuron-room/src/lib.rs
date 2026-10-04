// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Sceneuron's shared planning poker room, held by SpacetimeDB.
//!
//! Everyone in a call runs their own Sceneuron and joins the same room. While joined, raising a
//! card in the bound deck, from any source, casts a hidden vote: the voter's overlay shows the
//! card covered ("✓ Voted") and everyone sees who voted. A reveal from anyone uncovers each
//! person's own card at the same moment, and the next round lowers them. `{{room}}` in overlay
//! text shows "Voted: 3 of 5", then the revealed cards.
//!
//! [`RoomClient`] connects only when someone creates or joins a room, through a [`Connector`]:
//! [`SpacetimeConnector`], Sceneuron's own client of SpacetimeDB's documented WebSocket API, or a
//! fake in tests. No SpacetimeDB code is linked in; only `room/module`, which SpacetimeDB runs, is
//! built with SpacetimeDB's crates.

mod actions;
mod backend;
mod client;
mod follow;
mod local;
mod practice;
mod session;
mod spacetime;
mod state;
mod summary;
mod target;

pub use backend::{BackendEvent, Call, CallError, Connector, Done, Events, Member, RoomBackend, RoomView, Shown};
pub use client::{RoomClient, Timeouts};
pub use local::{COVER_TEXT, cover_for};
pub use spacetime::SpacetimeConnector;
pub use summary::summary;
pub use target::{DEFAULT_DATABASE, DEFAULT_SERVER, Target};
