// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises flood requests, which play reaction presets: the editor and a control surface play
//! every preset at once, each with a new nonce that the event streams publish, a control surface
//! gets its control view back with the reaction playing, the overlay token and no token cannot
//! play one, and malformed requests and unknown presets are refused.

mod common;

use std::time::Duration;

use common::{EventStream, Req, TestServer, control_post};
use sceneuron_core::presets;
use serde_json::{Value, json};

const TONGUE: &str = r#"{"flood":"tongue"}"#;

fn nonce(snapshot: &Value) -> u64 {
    snapshot["flood"]["nonce"].as_u64().expect("a flood with a nonce")
}

fn preset_ids() -> Vec<Value> {
    presets()
        .iter()
        .map(|preset| serde_json::to_value(preset.flood).unwrap())
        .collect()
}

#[test]
fn the_editor_floods_the_overlay_with_a_new_nonce_each_time() {
    let server = TestServer::start();
    assert_eq!(server.get_json("/api/state")["flood"], json!(null));
    let first = server.post_json("/api/flood", TONGUE);
    assert_eq!(first.status, 200, "{}", first.text());
    let first = first.json();
    assert_eq!(first["flood"]["flood"], "tongue");
    let second = server.post_json("/api/flood", TONGUE).json();
    assert!(
        nonce(&second) > nonce(&first),
        "a flood within the cooldown still plays"
    );
    let events = second["events"].as_array().unwrap();
    assert_eq!(events[0]["source"], "manual");
    assert_eq!(events[0]["detail"], "flooded the overlay with 😜 😛 🤪 😝");
}

#[test]
fn the_event_stream_publishes_the_flood() {
    let server = TestServer::start();
    let mut events: EventStream = server.events();
    let before = events.next_state(Duration::from_secs(3)).expect("the first snapshot");
    assert_eq!(before["flood"], json!(null));
    assert_eq!(server.post_json("/api/flood", TONGUE).status, 200);
    let published = events.next_state(Duration::from_secs(3)).expect("the flood's snapshot");
    assert_eq!(published["flood"]["flood"], "tongue");
}

#[test]
fn a_control_surface_floods_and_gets_its_view_back_with_the_reaction_playing() {
    let server = TestServer::start();
    let reply = control_post(&server, "/api/flood", json!({"flood": "tongue"}));
    assert_eq!(reply.status, 200, "{}", reply.text());
    let view = reply.json();
    assert!(view.get("decks").is_some() && view.get("events").is_none());
    let flood = server.hub.read(|engine| engine.flood()).expect("a flood");
    assert_eq!(view["flood"]["flood"], "tongue");
    assert_eq!(view["flood"]["nonce"], flood.nonce);
    let left_ms = view["flood"]["left_ms"].as_u64().unwrap();
    assert!(left_ms > 2_000 && left_ms <= 3_000, "{left_ms}");
}

#[test]
fn every_preset_plays_from_the_editor_and_from_a_control_surface() {
    let server = TestServer::start();
    let mut last = 0;
    for id in preset_ids() {
        let snapshot = server.post_json("/api/flood", &json!({"flood": id}).to_string()).json();
        assert_eq!(snapshot["flood"]["flood"], id);
        assert!(nonce(&snapshot) > last, "{id}");
        last = nonce(&snapshot);
        let view = control_post(&server, "/api/flood", json!({"flood": id})).json();
        assert_eq!(view["flood"]["flood"], id);
        assert!(nonce(&view) > last, "{id}");
        last = nonce(&view);
    }
    let events = server.get_json("/api/state")["events"].clone();
    assert_eq!(
        events[0]["detail"],
        r#"flooded the overlay with 🤖 🦾 ⚙️ 🔧 and "My clanker will do it!""#
    );
}

#[test]
fn the_control_event_stream_sends_a_view_for_each_new_reaction() {
    let server = TestServer::start();
    let mut events = server.control_events(&server.control_token);
    let first = events.next_control(Duration::from_secs(3)).expect("the first view");
    assert!(first.get("flood").is_none(), "no reaction plays");
    for id in ["email", "email", "ship-it"] {
        assert_eq!(control_post(&server, "/api/flood", json!({"flood": id})).status, 200);
        let view = events
            .next_control(Duration::from_secs(3))
            .expect("a view for the reaction");
        let playing = server.hub.read(|engine| engine.flood()).unwrap();
        assert_eq!(view["flood"]["flood"], id);
        assert_eq!(nonce(&view), playing.nonce, "{id}");
    }
}

#[test]
fn the_overlay_token_and_no_token_cannot_flood() {
    let server = TestServer::start();
    let url = server.server.as_ref().unwrap().overlay_url();
    let token = url.split("token=").nth(1).unwrap().to_owned();
    for id in preset_ids() {
        let body = json!({"flood": id}).to_string();
        let overlay = Req::new("POST", "/api/flood")
            .header("X-Sceneuron-Token", &token)
            .body(body.as_str())
            .send(server.port);
        assert_eq!(overlay.status, 403, "{}", overlay.text());
        let tokenless = Req::new("POST", "/api/flood").body(body.as_str()).send(server.port);
        assert_eq!(tokenless.status, 401, "{}", tokenless.text());
    }
    assert!(server.hub.read(|engine| engine.flood()).is_none());
}

#[test]
fn malformed_floods_and_unknown_presets_are_refused() {
    let server = TestServer::start();
    for body in [
        r#"{"flood":"confetti"}"#,
        r#"{"flood":"mind_blown"}"#,
        r#"{"flood":"Email"}"#,
        "{}",
        r#"{"flood":"tongue","extra":1}"#,
        "nope",
    ] {
        let reply = server.post_json("/api/flood", body);
        assert_eq!(reply.status, 400, "{body}: {}", reply.text());
        let control = server.control("POST", "/api/flood").body(body).send(server.port);
        assert_eq!(control.status, 400, "{body}: {}", control.text());
    }
    let get = server.api("GET", "/api/flood").send(server.port);
    assert_eq!(get.status, 405);
    assert_eq!(get.header("Allow"), Some("POST"));
    assert!(server.hub.read(|engine| engine.flood()).is_none());
}
