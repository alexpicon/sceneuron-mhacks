// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises `/api/room`: who may read the room and take each action (the editor token every
//! action, a control token only reveal and next, with answers that carry no names), how requests
//! reach the room, refusals, and a build without a room.

mod common;

use std::sync::Arc;

use common::{Req, TestServer};
use sceneuron_core::default_project;
use sceneuron_server::{Hub, HubConfig, NullAssistant, NullHost, NullRoom, RoomAction, Server, ServerConfig};
use serde_json::{Value, json};

const ACTIONS: [(&str, RoomAction); 5] = [
    ("create", RoomAction::Create),
    ("join", RoomAction::Join),
    ("leave", RoomAction::Leave),
    ("reveal", RoomAction::Reveal),
    ("next", RoomAction::Next),
];

fn body(action: &str) -> String {
    json!({ "action": action, "code": "K7M2QX", "name": "Ana" }).to_string()
}

fn overlay_token(server: &TestServer) -> String {
    let url = server.server.as_ref().unwrap().overlay_url();
    url.split("token=").nth(1).unwrap().to_owned()
}

#[test]
fn the_editor_token_reads_the_room_and_takes_every_action() {
    let server = TestServer::start();
    let state = server.get_json("/api/room");
    assert_eq!(state["code"], "K7M2QX");
    assert_eq!(state["participants"][0]["name"], "Ana");
    for (name, action) in ACTIONS {
        let reply = server.post_json("/api/room", &body(name));
        assert_eq!(reply.status, 200, "{name}: {}", reply.text());
        assert_eq!(reply.json()["after"], format!("{action:?}"));
    }
    assert_eq!(server.room.actions(), ACTIONS.map(|(_, action)| action));
    let first = server.room.requests.lock().unwrap()[0].clone();
    assert_eq!(
        (first.code.as_deref(), first.name.as_deref(), first.deck, first.server),
        (Some("K7M2QX"), Some("Ana"), None, None)
    );
}

#[test]
fn a_control_token_may_only_reveal_and_start_the_next_round() {
    let server = TestServer::start();
    let read = server.control("GET", "/api/room").send(server.port);
    assert_eq!(read.status, 403, "{}", read.text());
    assert_eq!(
        read.error(),
        "the control token can only reveal the votes and start the next round"
    );
    for (name, action) in ACTIONS {
        let reply = server.control("POST", "/api/room").body(body(name)).send(server.port);
        if matches!(action, RoomAction::Reveal | RoomAction::Next) {
            assert_eq!(reply.status, 200, "{name}: {}", reply.text());
            assert_eq!(reply.json(), json!({ "ok": true }), "{name} answers no names or votes");
        } else {
            assert_eq!(reply.status, 403, "{name}: {}", reply.text());
        }
    }
    assert_eq!(server.room.actions(), vec![RoomAction::Reveal, RoomAction::Next]);
}

#[test]
fn the_overlay_token_and_no_token_reach_nothing() {
    let server = TestServer::start();
    let token = overlay_token(&server);
    for (method, payload) in [("GET", String::new()), ("POST", body("reveal"))] {
        let overlay = Req::new(method, "/api/room")
            .header("X-Sceneuron-Token", &token)
            .body(payload.as_str());
        assert_eq!(overlay.send(server.port).status, 403, "{method} with the overlay token");
        let anonymous = Req::new(method, "/api/room").body(payload.as_str());
        assert_eq!(anonymous.send(server.port).status, 401, "{method} without a token");
    }
    assert!(server.room.actions().is_empty());
}

#[test]
fn malformed_requests_are_refused_before_they_reach_the_room() {
    let server = TestServer::start();
    for payload in [
        r#"{"action":"vote"}"#,
        r#"{"code":"K7M2QX"}"#,
        r#"{"action":"join","color":"red"}"#,
        r#"{"action":"join","code":5}"#,
        "not json",
    ] {
        let reply = server.post_json("/api/room", payload);
        assert_eq!(reply.status, 400, "{payload}: {}", reply.text());
    }
    let put = server.api("PUT", "/api/room").body("{}").send(server.port);
    assert_eq!(put.status, 405);
    assert_eq!(put.header("Allow"), Some("GET, POST"));
    assert!(server.room.actions().is_empty());
}

#[test]
fn the_rooms_refusals_keep_their_status_and_message() {
    let server = TestServer::start();
    let reply = server.post_json("/api/room", r#"{"action":"join","code":"WRONG1","name":"Ana"}"#);
    assert_eq!(reply.status, 400);
    assert_eq!(reply.error(), "no room has the code WRONG1");
}

#[test]
fn a_build_without_a_room_says_so() {
    let hub = Hub::start(default_project(), HubConfig::default()).unwrap();
    let config = ServerConfig::default();
    let token = config.token.clone();
    let room = Arc::new(NullRoom);
    let server = Server::start(
        Arc::clone(&hub),
        Arc::new(NullHost),
        Arc::new(NullAssistant),
        room,
        config,
    )
    .unwrap();
    let get = Req::new("GET", "/api/room")
        .header("X-Sceneuron-Token", &token)
        .send(server.port());
    assert_eq!(get.json(), json!({ "available": false }));
    let post = Req::new("POST", "/api/room")
        .header("X-Sceneuron-Token", &token)
        .body(r#"{"action":"create","name":"Ana"}"#)
        .send(server.port());
    assert_eq!(post.status, 400);
    assert_eq!(
        post.json()["error"],
        Value::from("the shared room is not available in this build")
    );
    server.stop();
    hub.shutdown();
}
