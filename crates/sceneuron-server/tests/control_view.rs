// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the control view: its shape, what it leaves out for privacy, and which requests
//! answer a control token with it instead of the snapshot.

mod common;

use std::collections::BTreeSet;

use common::{TestServer, control_post, deck, element, keys, project_with_packs, server_with_decks, view_after};
use sceneuron_core::{PropsPatch, SceneObservation};
use sceneuron_server::HubConfig;
use serde_json::{Value, json};

fn set(names: &[&'static str]) -> BTreeSet<&'static str> {
    names.iter().copied().collect()
}

#[test]
fn the_control_view_names_what_shows_and_nothing_else() {
    let server = server_with_decks();
    let view = server.control("GET", "/api/control/state").send(server.port).json();
    assert_eq!(
        keys(&view),
        set(&["revision", "automation_enabled", "decks", "elements"])
    );
    assert_eq!(view["revision"], server.hub.snapshot().revision);
    assert_eq!(view["automation_enabled"], true);
    let meeting = deck(&view, "meeting");
    assert_eq!(
        keys(meeting),
        set(&["id", "name", "raised", "raised_by", "hide_at_ms", "covered", "cards"])
    );
    assert_eq!(
        (
            &meeting["name"],
            &meeting["raised"],
            &meeting["raised_by"],
            &meeting["hide_at_ms"],
            &meeting["covered"]
        ),
        (
            &json!("Meeting signals"),
            &Value::Null,
            &Value::Null,
            &Value::Null,
            &json!(false)
        )
    );
    assert_eq!(meeting["cards"].as_array().unwrap().len(), 8);
    let wrap_up = &meeting["cards"][0];
    assert_eq!(keys(wrap_up), set(&["id", "name", "text", "color", "background"]));
    assert_eq!(
        wrap_up,
        &json!({"id": "wrap-up", "name": "Wrap it up", "text": "⏱️ Wrap it up",
                "color": "#111827ff", "background": "#f59e0bff"})
    );
    let five = &deck(&view, "poker")["cards"][4];
    assert_eq!(
        five,
        &json!({"id": "p5", "name": "5", "text": "5", "color": "#111827ff", "background": "#f8fafcff"}),
        "a card that sets only its text shows the slot's colors"
    );
    let ids: Vec<&str> = view["elements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();
    let project_ids: Vec<String> = server
        .hub
        .read(|engine| engine.project().elements.iter().map(|e| e.id.clone()).collect());
    assert_eq!(ids, project_ids);
    let brb = element(&view, "brb");
    assert_eq!(keys(brb), set(&["id", "name", "visible", "held"]));
    assert_eq!(brb["held"], false);
}

#[test]
fn card_text_is_shown_as_written_so_captions_stay_private() {
    let mut project = project_with_packs(&["poker"]);
    project.decks[0].cards[0].props.text = Some("Now: {{scene}}".to_owned());
    let server = TestServer::with(project, HubConfig::default());
    let caption = "Alex's whiteboard with the quarterly numbers";
    let observation = SceneObservation {
        caption: caption.to_owned(),
        verdicts: Vec::new(),
    };
    server.hub.update(|engine, now| engine.observe_scene(observation, now));
    let raised = view_after(
        &server,
        "/api/card",
        json!({"deck": "poker", "card": "p0", "action": "raise"}),
    );
    assert_eq!(deck(&raised, "poker")["cards"][0]["text"], "Now: {{scene}}");
    assert!(!raised.to_string().contains("whiteboard"), "{raised}");
    let snapshot = server.hub.snapshot();
    assert!(
        snapshot.elements.iter().any(|e| e.props.text.contains("whiteboard")),
        "the overlay shows it"
    );
}

#[test]
fn a_covered_card_is_marked_and_keeps_its_face() {
    let server = server_with_decks();
    let raised = view_after(
        &server,
        "/api/card",
        json!({"deck": "poker", "card": "p5", "action": "raise"}),
    );
    let cover = PropsPatch {
        text: Some("Voted".to_owned()),
        ..PropsPatch::default()
    };
    server
        .hub
        .try_update(|engine, now| engine.cover_card("poker", Some(cover), now))
        .unwrap();
    let covered = server.control("GET", "/api/control/state").send(server.port).json();
    let poker = deck(&covered, "poker");
    assert_eq!((&poker["raised"], &poker["covered"]), (&json!("p5"), &json!(true)));
    assert_eq!(
        poker["cards"],
        deck(&raised, "poker")["cards"],
        "a key still shows its card"
    );
    assert_eq!(deck(&covered, "meeting")["covered"], false);
}

#[test]
fn a_card_request_answers_each_token_with_its_view() {
    let server = server_with_decks();
    let body = json!({"deck": "poker", "card": "p5", "action": "raise"}).to_string();
    let full = server.post_json("/api/card", &body);
    assert_eq!(full.status, 200, "{}", full.text());
    let snapshot = full.json();
    assert!(snapshot.get("events").is_some() && snapshot.get("presence").is_some());
    assert_eq!(snapshot["decks"][1]["raised"], "p5");
    let control = control_post(&server, "/api/card", json!({"deck": "poker", "action": "lower"}));
    let view = control.json();
    assert_eq!(
        keys(&view),
        set(&["revision", "automation_enabled", "decks", "elements"])
    );
    assert_eq!(deck(&view, "poker")["raised"], Value::Null);
}

#[test]
fn automation_release_and_effects_answer_a_control_token_with_its_view() {
    let server = TestServer::start();
    let paused = view_after(&server, "/api/automation", json!({"enabled": false}));
    assert_eq!(paused["automation_enabled"], false);
    assert!(paused.get("events").is_none());
    let effect = view_after(&server, "/api/effect", json!({"element": "brb", "effect": "shake"}));
    assert_eq!(
        keys(&effect),
        set(&["revision", "automation_enabled", "decks", "elements"])
    );
    view_after(
        &server,
        "/api/control/element",
        json!({"element": "brb", "action": "hide"}),
    );
    let released = view_after(&server, "/api/release", json!({}));
    assert_eq!(element(&released, "brb")["held"], false);
    let full = server.post_json("/api/automation", r#"{"enabled":true}"#).json();
    assert!(full.get("events").is_some(), "the editor still gets the snapshot");
}
