// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises card hand signals through the project API: every kind the editor offers is saved
//! and read back as sent, and a finger count above ten, an unknown sign, or a malformed signal is
//! refused without changing the project.

mod common;

use common::{TestServer, project_with_packs};
use sceneuron_server::HubConfig;
use serde_json::{Value, json};

fn meeting_server() -> TestServer {
    TestServer::with(project_with_packs(&["meeting"]), HubConfig::default())
}

/// The saved project with `signal` on the meeting deck's `card`; `None` removes it.
fn with_signal(server: &TestServer, card: &str, signal: Option<Value>) -> Value {
    let mut project = server.get_json("/api/project");
    let cards = project["decks"][0]["cards"].as_array_mut().unwrap();
    let found = cards.iter_mut().find(|found| found["id"] == card).unwrap();
    let fields = found.as_object_mut().unwrap();
    match signal {
        Some(signal) => fields.insert("signal".to_owned(), signal),
        None => fields.remove("signal"),
    };
    project
}

fn put(server: &TestServer, project: &Value) -> common::Reply {
    server
        .api("PUT", "/api/project")
        .body(project.to_string())
        .send(server.port)
}

#[test]
fn every_signal_the_editor_offers_is_saved_as_sent() {
    let server = meeting_server();
    let mut signals = vec![json!("hand_raised")];
    signals.extend((0..=10).map(|fingers| json!({ "fingers": fingers })));
    for sign in ["thumbs_up", "thumbs_down", "open_palm", "peace", "pointing", "fist"] {
        signals.push(json!({ "sign": sign }));
    }
    for signal in signals {
        let project = with_signal(&server, "cant-hear", Some(signal.clone()));
        let reply = put(&server, &project);
        assert_eq!(reply.status, 200, "{signal}: {}", reply.text());
        assert_eq!(server.get_json("/api/project"), project, "{signal}");
    }
    let project = with_signal(&server, "cant-hear", None);
    assert_eq!(put(&server, &project).status, 200);
    let saved = server.get_json("/api/project");
    assert!(
        saved["decks"][0]["cards"][5].get("signal").is_none(),
        "no signal is no key"
    );
    assert_eq!(saved, project);
}

#[test]
fn a_finger_count_above_ten_or_an_unknown_sign_is_refused_and_nothing_changes() {
    let server = meeting_server();
    let before = server.get_json("/api/project");
    let revision = server.hub.snapshot().revision;
    let eleven = put(
        &server,
        &with_signal(&server, "cant-hear", Some(json!({"fingers": 11}))),
    );
    assert_eq!(
        (eleven.status, eleven.error()),
        (400, "meeting: cant-hear: signal fingers must be 0 to 10".to_owned())
    );
    let refused = [
        json!({"sign": "rock_on"}),
        json!({"sign": "Thumbs up"}),
        json!({"fingers": -1}),
        json!({"fingers": 2.5}),
        json!({"fingers": 256}),
        json!({"fingers": 2, "sign": "peace"}),
        json!("raised_hand"),
        json!(3),
    ];
    for signal in refused {
        let reply = put(&server, &with_signal(&server, "cant-hear", Some(signal.clone())));
        assert_eq!(reply.status, 400, "{signal}: {}", reply.text());
        assert!(
            reply.error().starts_with("invalid request body: "),
            "{signal}: {}",
            reply.error()
        );
    }
    assert_eq!(server.get_json("/api/project"), before);
    assert_eq!(server.hub.snapshot().revision, revision);
}
