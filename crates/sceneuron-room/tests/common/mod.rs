// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Shared helpers: a Sceneuron with the planning poker pack and a `{{room}}` element, its room
//! client on the fake rooms, and readers for what its overlay shows.

#![allow(dead_code)]

pub mod fake;
mod world;

use std::sync::Arc;
use std::time::Duration;

use sceneuron_core::{CardAction, Element, ElementKind, ElementProps, default_project, pack_for_canvas};
use sceneuron_room::{RoomClient, Timeouts};
use sceneuron_server::{Hub, HubConfig, Room, RoomAction, RoomError, RoomRequest};
use serde_json::Value;

pub use fake::FakeRooms;

/// One participant: a hub with the poker pack and its room client.
pub struct Person {
    pub hub: Arc<Hub>,
    pub client: Arc<RoomClient>,
}

impl Drop for Person {
    fn drop(&mut self) {
        self.client.shutdown();
        self.hub.shutdown();
    }
}

pub const SHORT: Timeouts = Timeouts {
    connect: Duration::from_millis(300),
    call: Duration::from_millis(300),
    view: Duration::from_millis(300),
};

pub fn person(rooms: &Arc<FakeRooms>) -> Person {
    let mut project = default_project();
    let pack = pack_for_canvas("poker", &project.canvas).unwrap();
    project = project.add_pack(&pack).unwrap();
    project.elements.push(Element {
        id: "room-summary".to_owned(),
        name: "Room summary".to_owned(),
        kind: ElementKind::Text,
        props: ElementProps {
            text: "{{room}}".to_owned(),
            ..ElementProps::default()
        },
    });
    let hub = Hub::start(project, HubConfig::default()).unwrap();
    let client = RoomClient::with_connector(Arc::clone(&hub), rooms.connector(), SHORT);
    Person { hub, client }
}

pub fn request(action: RoomAction) -> RoomRequest {
    RoomRequest {
        action,
        code: None,
        name: None,
        deck: None,
        server: None,
        database: None,
    }
}

impl Person {
    /// Creates a room as `name` and returns its code.
    pub fn create(&self, name: &str) -> String {
        let state = self.act(RoomRequest {
            name: Some(name.to_owned()),
            ..request(RoomAction::Create)
        });
        state["code"].as_str().unwrap().to_owned()
    }

    pub fn join(&self, code: &str, name: &str) -> Value {
        self.act(RoomRequest {
            code: Some(code.to_owned()),
            name: Some(name.to_owned()),
            ..request(RoomAction::Join)
        })
    }

    pub fn act(&self, request: RoomRequest) -> Value {
        self.client.act(request).unwrap_or_else(|error| panic!("{error:?}"))
    }

    pub fn try_act(&self, request: RoomRequest) -> Result<Value, RoomError> {
        self.client.act(request)
    }

    pub fn state(&self) -> Value {
        self.client.state()
    }

    pub fn raise(&self, card: &str) {
        self.hub
            .try_update(|engine, now| engine.card("poker", Some(card), CardAction::Raise, now))
            .unwrap();
    }

    pub fn lower(&self) {
        self.hub
            .try_update(|engine, now| engine.card("poker", None, CardAction::Lower, now))
            .unwrap();
    }

    /// The text the poker card slot shows, and whether it shows.
    pub fn slot(&self) -> (String, bool) {
        let props = self.hub.read(|engine| engine.effective_props("poker-card").unwrap());
        (props.text, props.visible)
    }

    pub fn summary(&self) -> String {
        self.hub
            .read(|engine| engine.effective_props("room-summary").unwrap().text)
    }

    pub fn raised(&self) -> Option<String> {
        self.hub.read(|engine| engine.raised_card("poker").map(str::to_owned))
    }

    pub fn covered(&self) -> bool {
        self.hub.read(|engine| engine.cover("poker").is_some())
    }
}

/// Waits up to 5 s for `done`, for what other threads finish, and fails the test if it never is.
#[allow(dead_code)]
pub fn wait_until(mut done: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !done() {
        assert!(std::time::Instant::now() < deadline, "gave up waiting after 5 s");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}
