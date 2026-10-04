// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the subtitle endpoints: who may use each one (an explicit table for every token),
//! what `GET /api/subtitles` shows and that the snapshot carries the same, a simulated line and
//! the bodies it refuses, and the control view, which leaves subtitles out.

mod common;

use common::{Reply, Req, TestServer};
use sceneuron_core::{Heard, SubtitleStatus};

/// One request and the status each token gets: none, overlay, control, editor.
const ACCESS: &[(&str, &str, &str, [u16; 4])] = &[
    ("GET", "/api/subtitles", "", [401, 200, 403, 200]),
    (
        "POST",
        "/api/subtitles/simulate",
        r#"{"text":"Hello"}"#,
        [401, 403, 403, 200],
    ),
];

fn overlay_token(server: &TestServer) -> String {
    let url = server.server.as_ref().unwrap().overlay_url();
    url.split("token=").nth(1).unwrap().to_owned()
}

fn send(request: Req, body: &str, port: u16) -> Reply {
    if body.is_empty() { request } else { request.body(body) }.send(port)
}

#[test]
fn each_token_reaches_exactly_the_subtitle_endpoints_in_the_table() {
    let server = TestServer::start();
    let overlay = overlay_token(&server);
    for (method, target, body, expected) in ACCESS {
        let requests = [
            Req::new(method, target),
            server.with_token(&overlay, method, target),
            server.control(method, target),
            server.api(method, target),
        ];
        for ((request, expected), who) in requests
            .into_iter()
            .zip(expected)
            .zip(["none", "overlay", "control", "editor"])
        {
            let reply = send(request, body, server.port);
            assert_eq!(reply.status, *expected, "{who}: {method} {target}: {}", reply.text());
        }
    }
    let wrong = server.api("POST", "/api/subtitles").send(server.port);
    assert_eq!((wrong.status, wrong.header("Allow")), (405, Some("GET")));
    let wrong = server.api("GET", "/api/subtitles/simulate").send(server.port);
    assert_eq!((wrong.status, wrong.header("Allow")), (405, Some("POST")));
}

#[test]
fn the_subtitles_view_shows_the_lines_the_live_words_and_the_status() {
    let server = TestServer::start();
    let view = server.get_json("/api/subtitles");
    assert_eq!(view["lines"], serde_json::json!([]));
    assert_eq!(
        (view["status"]["state"].clone(), view["status"]["enabled"].clone()),
        ("off".into(), false.into())
    );
    server.hub.update(|engine, now| {
        let status = SubtitleStatus {
            enabled: true,
            state: "listening".to_owned(),
            targets: vec!["es".to_owned()],
            ..SubtitleStatus::default()
        };
        engine.set_subtitle_status(status, now);
        let heard = Heard {
            text: "Good morning".to_owned(),
            is_final: true,
            addressed: false,
        };
        engine.hear(&heard, now);
        let partial = Heard {
            text: "and welcome".to_owned(),
            is_final: false,
            addressed: false,
        };
        engine.hear(&partial, now)
    });
    let view = server.get_json("/api/subtitles");
    assert_eq!(view["lines"], serde_json::json!(["Good morning"]));
    assert_eq!(view["live"], "and welcome");
    assert_eq!(view["translations"], serde_json::json!({"es": []}));
    let state = server.get_json("/api/state");
    assert_eq!(state["subtitles"], view, "the snapshot carries the same");
}

#[test]
fn a_simulated_line_is_shown_and_bad_bodies_are_refused() {
    let server = TestServer::start();
    let reply = server.post_json("/api/subtitles/simulate", r#"{"text":"  Checking the bar  "}"#);
    assert_eq!(reply.status, 200, "{}", reply.text());
    assert_eq!(reply.json()["lines"], serde_json::json!(["Checking the bar"]));
    let events = server.get_json("/api/state")["events"].clone();
    assert_eq!(events[0]["detail"], "simulated a subtitle line", "never the words");
    let long = format!(r#"{{"text":"{}"}}"#, "a".repeat(301));
    for bad in [
        "{}",
        r#"{"text":""}"#,
        r#"{"text":"   "}"#,
        r#"{"text":5}"#,
        r#"{"text":"hi","lang":"es"}"#,
        &long,
    ] {
        assert_eq!(server.post_json("/api/subtitles/simulate", bad).status, 400, "{bad}");
    }
}

#[test]
fn the_control_view_leaves_subtitles_out() {
    let server = TestServer::start();
    server.post_json("/api/subtitles/simulate", r#"{"text":"Private enough"}"#);
    let reply = server.control("GET", "/api/control/state").send(server.port);
    assert_eq!(reply.status, 200, "{}", reply.text());
    assert!(reply.json().get("subtitles").is_none());
    assert!(!reply.text().contains("Private enough"));
}
