// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the AI endpoints over real loopback connections: settings pass-through,
//! edit plans with their preview and base revision, saving a plan with If-Match and the
//! conflict when the project moved on, on-demand analysis, and a build without AI.

mod common;

use std::sync::Arc;
use std::sync::atomic::Ordering;

use common::{Req, TestServer, waving_plan};
use sceneuron_core::default_project;
use sceneuron_server::{Hub, HubConfig, NullAssistant, NullHost, NullRoom, PlanMode, Server, ServerConfig};
use serde_json::{Value, json};

fn plan(server: &TestServer, body: Value) -> common::Reply {
    server.post_json("/api/ai/plan", &body.to_string())
}

#[test]
fn settings_pass_through_the_assistant() {
    let server = TestServer::start();
    assert_eq!(server.get_json("/api/ai")["available"], true);
    let reply = server
        .api("PUT", "/api/ai")
        .body(json!({ "consent": true, "enabled": true }).to_string())
        .send(server.port);
    assert_eq!(reply.status, 200, "{}", reply.text());
    assert_eq!(reply.json()["enabled"], true);
    let refused = server
        .api("PUT", "/api/ai")
        .body(json!({ "enabled": true }).to_string())
        .send(server.port);
    assert_eq!(
        (refused.status, refused.error().as_str()),
        (400, "confirm consent first")
    );
    let not_object = server.api("PUT", "/api/ai").body("[1]").send(server.port);
    assert_eq!(not_object.status, 400);
}

#[test]
fn a_plan_comes_with_its_preview_and_base_revision() {
    let server = TestServer::start();
    let reply = plan(
        &server,
        json!({ "request": "Wave detection please", "mode": "setup", "use_camera": true }),
    );
    assert_eq!(reply.status, 200, "{}", reply.text());
    let body = reply.json();
    assert_eq!(body["summary"], "Watch for waving");
    assert_eq!(body["operations"][0]["op"], "add_condition");
    assert_eq!(
        body["base_revision"],
        server.hub.read(|engine| engine.project_revision())
    );
    let preview = &body["project"];
    assert!(
        preview["conditions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == "waving")
    );
    assert_eq!(
        (body["model"].as_str(), body["cost_usd"].as_f64()),
        (Some("fake-model"), Some(0.01))
    );
    let request = server.assistant.requests.lock().unwrap()[0].clone();
    assert_eq!((request.mode, request.use_camera), (PlanMode::Setup, true));
    assert!(server.hub.read(|engine| engine.project().condition("waving").is_none()));
}

#[test]
fn plan_requests_are_checked_and_assistant_errors_keep_their_status() {
    let server = TestServer::start();
    for request in [json!({ "request": "  " }), json!({ "request": "x".repeat(2001) })] {
        assert_eq!(plan(&server, request).status, 400);
    }
    assert_eq!(plan(&server, json!({ "request": "hi", "extra": 1 })).status, 400);
    let failed = plan(&server, json!({ "request": "fail" }));
    assert_eq!(
        (failed.status, failed.error().as_str()),
        (502, "the API key was rejected")
    );
}

#[test]
fn saving_a_plan_checks_the_base_revision() {
    let server = TestServer::start();
    let body = plan(&server, json!({ "request": "add waving" })).json();
    let base = body["base_revision"].as_u64().unwrap();
    let stale = waving_plan().apply(&default_project()).unwrap();
    let saved = server
        .api("PUT", "/api/project")
        .header("If-Match", &base.to_string())
        .body(body["project"].to_string())
        .send(server.port);
    assert_eq!(saved.status, 200, "{}", saved.text());
    assert_eq!(saved.json()["project_revision"], base + 1);
    let conflict = server
        .api("PUT", "/api/project")
        .header("If-Match", &format!("\"{base}\""))
        .body(serde_json::to_string(&stale).unwrap())
        .send(server.port);
    assert_eq!(conflict.status, 409, "{}", conflict.text());
    assert_eq!(conflict.json()["project_revision"], base + 1);
    let garbage = server
        .api("PUT", "/api/project")
        .header("If-Match", "soon")
        .body(serde_json::to_string(&stale).unwrap())
        .send(server.port);
    assert_eq!(garbage.status, 400);
}

#[test]
fn pausing_automation_does_not_conflict_with_an_open_editor() {
    let server = TestServer::start();
    let base = server.hub.read(|engine| engine.project_revision());
    server.post_json("/api/automation", r#"{"enabled": false}"#);
    let saved = server
        .api("PUT", "/api/project")
        .header("If-Match", &base.to_string())
        .body(serde_json::to_string(&default_project()).unwrap())
        .send(server.port);
    assert_eq!(saved.status, 200, "{}", saved.text());
}

#[test]
fn analyze_now_is_accepted() {
    let server = TestServer::start();
    let reply = server.post_json("/api/ai/analyze", "");
    assert_eq!(reply.status, 202, "{}", reply.text());
    assert_eq!(server.assistant.looks.load(Ordering::SeqCst), 1);
    assert_eq!(server.api("GET", "/api/ai/analyze").send(server.port).status, 405);
}

#[test]
fn a_build_without_ai_says_so() {
    let hub = Hub::start(default_project(), HubConfig::default()).unwrap();
    let config = ServerConfig::default();
    let token = config.token.clone();
    let server = Server::start(
        Arc::clone(&hub),
        Arc::new(NullHost),
        Arc::new(NullAssistant),
        Arc::new(NullRoom),
        config,
    )
    .unwrap();
    let get = |target: &str| {
        Req::new("GET", target)
            .header("X-Sceneuron-Token", &token)
            .send(server.port())
    };
    assert_eq!(get("/api/ai").json(), json!({ "available": false }));
    let reply = Req::new("POST", "/api/ai/analyze")
        .header("X-Sceneuron-Token", &token)
        .send(server.port());
    assert_eq!(reply.status, 400);
    server.stop();
    hub.shutdown();
}
