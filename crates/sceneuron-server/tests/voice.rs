// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the voice endpoints: who may use each one (an explicit table for every token),
//! what `GET /api/voice` shows, push-to-talk held and let go from the editor and a control
//! surface, and spoken card audio served once it exists and refused before. voice_commands.rs
//! covers the guide to what can be said and typed commands.

mod common;

use common::{Reply, Req, TestServer, server_with_decks};
use sceneuron_core::{Heard, VoiceMode, VoiceStatus};

const SPEECH_ID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// One request and the status each token gets: none, overlay, control, editor.
const ACCESS: &[(&str, &str, &str, [u16; 4])] = &[
    ("GET", "/api/voice", "", [401, 403, 403, 200]),
    ("POST", "/api/voice/talk", r#"{"down":false}"#, [401, 403, 200, 200]),
    ("GET", "/api/voice/guide", "", [401, 403, 403, 200]),
    (
        "POST",
        "/api/voice/command",
        r#"{"text":"lower"}"#,
        [401, 403, 403, 200],
    ),
    (
        "GET",
        "/api/speech/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "",
        [401, 200, 403, 200],
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
fn each_token_reaches_exactly_the_voice_endpoints_in_the_table() {
    let server = server_with_decks();
    server
        .assistant
        .speeches
        .lock()
        .unwrap()
        .push((SPEECH_ID.to_owned(), b"ID3audio".to_vec()));
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
    let wrong_method = server.api("POST", "/api/voice").send(server.port);
    assert_eq!((wrong_method.status, wrong_method.header("Allow")), (405, Some("GET")));
}

#[test]
fn the_voice_view_shows_the_status_and_what_was_heard() {
    let server = server_with_decks();
    let view = server.get_json("/api/voice");
    assert_eq!(view["enabled"], false);
    assert_eq!(view["mode"], "push_to_talk");
    assert_eq!(
        (view["listening"].clone(), view["talking"].clone()),
        (false.into(), false.into())
    );
    assert!(view["heard"].is_null());
    server.hub.update(|engine, now| {
        let mut project = engine.project().clone();
        project.decks[0].voice = true;
        engine.replace_project(project, now).unwrap();
        let status = VoiceStatus {
            enabled: true,
            mode: VoiceMode::AlwaysOn,
            ..VoiceStatus::default()
        };
        engine.set_voice_status(status, now);
        let heard = Heard {
            text: "Sceneuron, wrap it up".to_owned(),
            is_final: true,
            addressed: false,
        };
        engine.hear(&heard, now)
    });
    let view = server.get_json("/api/voice");
    assert_eq!(
        (view["enabled"].clone(), view["mode"].clone()),
        (true.into(), "always_on".into())
    );
    assert_eq!(view["heard"]["text"], "Sceneuron, wrap it up");
    assert_eq!(view["heard"]["result"], "raised Meeting signals: Wrap it up");
    assert_eq!(
        (view["heard"]["deck"].clone(), view["heard"]["card"].clone()),
        ("meeting".into(), "wrap-up".into())
    );
    let state = server.get_json("/api/state");
    assert_eq!(state["voice"], view);
}

#[test]
fn push_to_talk_is_held_and_let_go_by_the_editor_or_a_control_surface() {
    let server = server_with_decks();
    let held = server.post_json("/api/voice/talk", r#"{"down":true}"#);
    assert_eq!(held.status, 200, "{}", held.text());
    assert_eq!(held.json()["voice"]["talking"], true, "the editor gets the snapshot");
    assert!(server.hub.read(|engine| engine.talking()));
    let reply = server
        .control("POST", "/api/voice/talk")
        .body(r#"{"down":false}"#)
        .send(server.port);
    assert_eq!(reply.status, 200, "{}", reply.text());
    assert!(
        reply.json().get("voice").is_none(),
        "a control surface gets the control view"
    );
    assert!(!server.hub.read(|engine| engine.talking()));
    for bad in ["{}", r#"{"down":"yes"}"#, r#"{"down":true,"extra":1}"#] {
        assert_eq!(server.post_json("/api/voice/talk", bad).status, 400, "{bad}");
    }
}

#[test]
fn spoken_card_audio_is_served_once_it_exists() {
    let server = TestServer::start();
    let missing = server.api("GET", &format!("/api/speech/{SPEECH_ID}")).send(server.port);
    assert_eq!(missing.status, 404);
    assert_eq!(missing.error(), "this spoken card is not ready");
    server
        .assistant
        .speeches
        .lock()
        .unwrap()
        .push((SPEECH_ID.to_owned(), b"ID3audio".to_vec()));
    let overlay = overlay_token(&server);
    let target = format!("/api/speech/{SPEECH_ID}?token={overlay}");
    let found = Req::new("GET", &target).send(server.port);
    assert_eq!(found.status, 200);
    assert_eq!(found.header("Content-Type"), Some("audio/mpeg"));
    assert_eq!(found.body, b"ID3audio");
    for odd in [
        "/api/speech/",
        "/api/speech/../ai",
        "/api/speech/0123ABCDEF",
        &format!("/api/speech/{SPEECH_ID}0"),
    ] {
        assert_eq!(server.api("GET", odd).send(server.port).status, 404, "{odd}");
        assert_eq!(
            Req::new("GET", odd).send(server.port).status,
            401,
            "{odd} without a token"
        );
    }
}
