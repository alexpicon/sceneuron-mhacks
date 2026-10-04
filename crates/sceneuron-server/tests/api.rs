// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the JSON API over real loopback connections: state and project reads,
//! project replacement, overrides, releases, effects, automation, simulated presence,
//! and the host-backed source and overlay routes, with their success and error codes.

mod common;

use std::sync::Arc;

use common::{FAKE_SOURCE, Reply, TestServer, element};
use sceneuron_core::{Project, default_project};
use sceneuron_server::{Hub, HubConfig, NullAssistant, NullHost, NullRoom, Server, ServerConfig, generate_token};
use serde_json::{Value, json};

fn assert_json(reply: &Reply, status: u16) -> Value {
    assert_eq!(reply.status, status, "{}", reply.text());
    assert_eq!(reply.header("Content-Type"), Some("application/json"));
    assert_eq!(reply.header("Cache-Control"), Some("no-store"));
    assert_eq!(reply.header("X-Content-Type-Options"), Some("nosniff"));
    reply.json()
}

#[test]
fn state_returns_the_current_snapshot() {
    let server = TestServer::start();
    let reply = server.api("GET", "/api/state").send(server.port);
    let snapshot = assert_json(&reply, 200);
    assert_eq!(snapshot["revision"], server.hub.snapshot().revision);
    assert_eq!(snapshot["presence"]["state"], "unknown");
    assert_eq!(
        snapshot["elements"].as_array().unwrap().len(),
        default_project().elements.len()
    );
}

#[test]
fn project_round_trips_through_put() {
    let server = TestServer::start();
    let project = server.get_json("/api/project");
    assert_eq!(project, serde_json::to_value(default_project()).unwrap());
    let mut changed = project.clone();
    changed["elements"][1]["name"] = json!("Streamer name");
    changed["canvas"]["width"] = json!(1280);
    let reply = server
        .api("PUT", "/api/project")
        .body(changed.to_string())
        .send(server.port);
    let snapshot = assert_json(&reply, 200);
    assert_eq!(snapshot["canvas"]["width"], 1280);
    assert_eq!(element(&snapshot, "name-label")["name"], "Streamer name");
    assert_eq!(server.get_json("/api/project"), changed);
}

#[test]
fn an_invalid_project_is_rejected_and_nothing_changes() {
    let server = TestServer::start();
    let before = server.get_json("/api/project");
    let revision = server.hub.snapshot().revision;
    let mut duplicate = before.clone();
    duplicate["elements"][1]["id"] = json!("brb");
    let mut newer = before.clone();
    newer["version"] = json!(sceneuron_core::SCHEMA_VERSION + 1);
    let mut dangling = before.clone();
    dangling["rules"][0]["actions"][0]["element"] = json!("missing");
    let mut bad_color = before.clone();
    bad_color["elements"][0]["props"]["color"] = json!("red");
    let bodies = [
        duplicate.to_string(),
        newer.to_string(),
        dangling.to_string(),
        bad_color.to_string(),
        "{not json".to_owned(),
        String::new(),
        r#"{"elements": 5}"#.to_owned(),
    ];
    for body in bodies {
        let reply = server.api("PUT", "/api/project").body(body.clone()).send(server.port);
        assert_json(&reply, 400);
        assert!(!reply.error().is_empty(), "{body}");
    }
    assert_eq!(server.get_json("/api/project"), before);
    assert_eq!(server.hub.snapshot().revision, revision);
}

#[test]
fn override_holds_props_and_reports_errors() {
    let server = TestServer::start();
    let body = json!({"element": "brb", "props": {"visible": true, "text": "Lunch"}});
    let snapshot = assert_json(&server.post_json("/api/override", &body.to_string()), 200);
    let brb = element(&snapshot, "brb");
    assert_eq!(brb["props"]["visible"], true);
    assert_eq!(brb["props"]["text"], "Lunch");
    assert_eq!(brb["overridden"], json!(["visible", "text"]));

    let unknown = json!({"element": "missing", "props": {"visible": true}});
    let reply = server.post_json("/api/override", &unknown.to_string());
    assert_json(&reply, 404);
    assert!(reply.error().contains("missing"));
    let bad_requests = [
        json!({"element": "brb", "props": {"opacity": 5}}).to_string(),
        json!({"element": "brb", "props": {"sparkle": true}}).to_string(),
        json!({"element": "brb", "props": {}}).to_string(),
        json!({"element": "brb"}).to_string(),
        json!({"element": "brb", "props": {"visible": true}, "extra": 1}).to_string(),
        "[]".to_owned(),
    ];
    for body in bad_requests {
        assert_json(&server.post_json("/api/override", &body), 400);
    }
    let unchanged = server.get_json("/api/state");
    assert_eq!(element(&unchanged, "brb")["props"]["text"], "Lunch");
}

#[test]
fn release_frees_one_element_or_all() {
    let server = TestServer::start();
    for id in ["brb", "name-label"] {
        let body = json!({"element": id, "props": {"visible": false}});
        assert_json(&server.post_json("/api/override", &body.to_string()), 200);
    }
    let snapshot = assert_json(&server.post_json("/api/release", r#"{"element":"brb"}"#), 200);
    assert_eq!(element(&snapshot, "brb")["overridden"], json!([]));
    assert_eq!(element(&snapshot, "name-label")["overridden"], json!(["visible"]));
    assert_json(&server.post_json("/api/release", r#"{"element":"missing"}"#), 404);
    let snapshot = assert_json(&server.post_json("/api/release", r#"{"element":null}"#), 200);
    assert_eq!(element(&snapshot, "name-label")["overridden"], json!([]));
    assert_json(&server.post_json("/api/release", "{}"), 200);
    assert_json(&server.post_json("/api/release", r#"{"element":3}"#), 400);
}

#[test]
fn effect_plays_with_a_new_nonce() {
    let server = TestServer::start();
    let body = r#"{"element":"cam-frame","effect":"shake"}"#;
    let first = assert_json(&server.post_json("/api/effect", body), 200);
    let effect = &element(&first, "cam-frame")["effect"];
    assert_eq!(effect["effect"], "shake");
    let second = assert_json(&server.post_json("/api/effect", body), 200);
    assert_ne!(element(&second, "cam-frame")["effect"]["nonce"], effect["nonce"]);
    assert_json(
        &server.post_json("/api/effect", r#"{"element":"missing","effect":"pop"}"#),
        404,
    );
    assert_json(
        &server.post_json("/api/effect", r#"{"element":"brb","effect":"explode"}"#),
        400,
    );
}

#[test]
fn automation_pauses_and_resumes() {
    let server = TestServer::start();
    let paused = assert_json(&server.post_json("/api/automation", r#"{"enabled":false}"#), 200);
    assert_eq!(paused["automation_enabled"], false);
    assert!(!server.hub.read(|engine| engine.project().automation_enabled));
    let resumed = assert_json(&server.post_json("/api/automation", r#"{"enabled":true}"#), 200);
    assert_eq!(resumed["automation_enabled"], true);
    assert_json(&server.post_json("/api/automation", r#"{"enabled":"yes"}"#), 400);
}

#[test]
fn simulated_presence_runs_the_rules() {
    let server = TestServer::start();
    let away = assert_json(&server.post_json("/api/simulate", r#"{"presence":"away"}"#), 200);
    assert_eq!(away["presence"]["state"], "away");
    assert_eq!(element(&away, "brb")["props"]["visible"], true);
    let present = assert_json(&server.post_json("/api/simulate", r#"{"presence":"present"}"#), 200);
    assert_eq!(element(&present, "brb")["props"]["visible"], false);
    let unknown = assert_json(&server.post_json("/api/simulate", r#"{"presence":"unknown"}"#), 200);
    assert_eq!(unknown["presence"]["state"], "unknown");
    assert_json(&server.post_json("/api/simulate", r#"{"presence":"asleep"}"#), 400);
}

#[test]
fn sources_attach_and_overlay_use_the_host() {
    let server = TestServer::start();
    let sources = server.get_json("/api/sources");
    let expected = json!([{"uuid": FAKE_SOURCE, "name": "Camera", "kind": "av_capture_input", "analyzed": false}]);
    assert_eq!(sources, expected);
    let attach = json!({"uuid": FAKE_SOURCE}).to_string();
    let reply = server.post_json("/api/sources/attach", &attach);
    assert_eq!(assert_json(&reply, 200), json!({"ok": true}));
    assert_eq!(*server.host.attached.lock().unwrap(), vec![FAKE_SOURCE.to_owned()]);
    assert_eq!(server.get_json("/api/sources")[0]["analyzed"], true);
    let reply = server.post_json("/api/sources/attach", r#"{"uuid":"other"}"#);
    assert_json(&reply, 400);
    assert_eq!(reply.error(), "no source other");
    assert_json(&server.post_json("/api/sources/attach", "{}"), 400);
    let reply = server.api("POST", "/api/overlay").send(server.port);
    assert_eq!(assert_json(&reply, 200), json!({"name": "Sceneuron overlay"}));
}

#[test]
fn the_null_host_lists_nothing_and_refuses_scene_changes() {
    let hub = Hub::start(default_project(), HubConfig::default()).unwrap();
    let token = generate_token();
    let config = ServerConfig {
        port: 0,
        token: token.clone(),
        ..ServerConfig::default()
    };
    let server = Server::start(
        hub,
        Arc::new(NullHost),
        Arc::new(NullAssistant),
        Arc::new(NullRoom),
        config,
    )
    .unwrap();
    let request = |method: &str, path: &str| common::Req::new(method, path).header("X-Sceneuron-Token", &token);
    let reply = request("GET", "/api/sources").send(server.port());
    assert_eq!(assert_json(&reply, 200), json!([]));
    let reply = request("POST", "/api/sources/attach")
        .body(r#"{"uuid":"x"}"#)
        .send(server.port());
    assert_json(&reply, 400);
    assert_eq!(reply.error(), "Sceneuron is not running inside OBS");
    let reply = request("POST", "/api/overlay").send(server.port());
    assert_json(&reply, 400);
    assert_eq!(reply.error(), "Sceneuron is not running inside OBS");
}

#[test]
fn saving_a_project_keeps_the_live_automation_switch() {
    let server = TestServer::start();
    let mut project: Project = default_project();
    project.automation_enabled = false;
    project.elements[1].name = "Streamer name".to_owned();
    let body = serde_json::to_string(&project).unwrap();
    assert_json(&server.api("PUT", "/api/project").body(body).send(server.port), 200);
    let saved = server.hub.read(|engine| engine.project().clone());
    assert_eq!(saved.elements[1].name, "Streamer name");
    assert!(saved.automation_enabled && server.hub.snapshot().automation_enabled);
}
