// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The Sceneuron runtime shared by the OBS plugin and the development server: the [`Hub`]
//! that owns the overlay engine, and the local HTTP [`Server`] for the editor, the overlay
//! page, and the JSON API.

mod accept;
mod access;
mod api;
mod api_ai;
mod api_camera;
mod api_control;
mod api_room;
mod api_subtitles;
mod api_voice;
mod assets;
mod assistant;
mod body;
mod config;
mod connection;
mod control_access;
mod control_view;
mod date;
mod endpoints;
mod head;
mod hello;
mod host;
mod http;
mod hub;
mod links;
mod preview;
mod registry;
mod room;
mod routes;
mod server;
mod setup;
mod sse;
mod stream_deck;
mod token;

pub use assistant::{Assistant, AssistantError, NullAssistant, PlanMode, PlanRequest, ProposedPlan, Speech};
pub use config::{ServerConfig, ServerError};
pub use hello::proof;
pub use host::{AudioSourceInfo, Host, NullHost, SourceInfo};
pub use hub::{Hub, HubConfig, HubEvent};
pub use preview::{CameraAnalysis, FaceArea, PREVIEW_MAX_SIDE, PreviewGap, PreviewImage};
pub use room::{NullRoom, Room, RoomAction, RoomError, RoomRequest};
pub use server::Server;
pub use setup::{SetupOutcome, SetupReport, SetupStep, camera_to_analyze};
pub use stream_deck::stream_deck_pairing_link;
pub use token::{generate_token, tokens_match};

use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::Serialize;

/// Serializes to compact JSON.
///
/// The types sent here always serialize; the fallback only keeps a response well formed.
fn to_json<T: Serialize + ?Sized>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| r#"{"error":"cannot encode the response"}"#.to_owned())
}

/// Locks a mutex, recovering the data if another thread panicked while holding it.
///
/// Every critical section here leaves its data consistent between statements, and a
/// poisoned lock must not turn into a panic inside the host application.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
