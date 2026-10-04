// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises voice commands through the API: `GET /api/voice/guide` lists what can be said for
//! the saved project, `POST /api/voice/command` carries out typed words as a command with voice
//! control off, what only the host can do waits as a request once a host answers and is refused
//! at once before, malformed bodies are refused, and
//! the reactions' voice switches save with the project and are checked like the camera's. voice.rs
//! covers who may use each endpoint.

mod common;

use common::{TestServer, server_with_decks};
use sceneuron_core::{Flood, VoiceAction, VoiceTask};
use serde_json::{Value, json};

fn titles(guide: &Value) -> Vec<String> {
    let groups = guide["groups"].as_array().unwrap();
    groups
        .iter()
        .map(|group| group["title"].as_str().unwrap().to_owned())
        .collect()
}

fn group<'g>(guide: &'g Value, title: &str) -> &'g Value {
    let groups = guide["groups"].as_array().unwrap();
    groups.iter().find(|group| group["title"] == title).unwrap()
}

fn command(server: &TestServer, text: &str) -> Value {
    let reply = server.post_json("/api/voice/command", &json!({ "text": text }).to_string());
    assert_eq!(reply.status, 200, "{text}: {}", reply.text());
    reply.json()
}

#[test]
fn the_guide_lists_what_can_be_said_for_the_saved_project() {
    let server = server_with_decks();
    let guide = server.get_json("/api/voice/guide");
    assert_eq!(
        (guide["wake_word"].clone(), guide["mode"].clone()),
        (json!("Sceneuron"), json!("push_to_talk"))
    );
    assert_eq!(titles(&guide)[0], "Reactions");
    let cards = &group(&guide, "Cards")["entries"];
    assert_eq!(cards[0]["say"], json!([]), "no deck answers voice yet");
    let reaction = &group(&guide, "Reactions")["entries"][2];
    assert_eq!(
        reaction,
        &json!({"say": ["mind blown", "play mind blown"], "does": "plays 🤯 Mind blown at once"})
    );
    server.hub.update(|engine, now| {
        let mut project = engine.project().clone();
        project.decks.iter_mut().for_each(|deck| deck.voice = true);
        engine.replace_project(project, now).unwrap()
    });
    let guide = server.get_json("/api/voice/guide");
    let cards = &group(&guide, "Cards")["entries"];
    assert_eq!(cards[0]["does"], "raises that card of Meeting signals");
    assert_eq!(group(&guide, "Trigger phrases")["addressed"], false);
}

#[test]
fn typed_words_run_as_a_command_with_voice_off() {
    let server = server_with_decks();
    let view = command(&server, "Sceneuron, mind blown");
    assert_eq!(view["enabled"], false);
    let heard = &view["heard"];
    assert_eq!(
        (heard["text"].clone(), heard["typed"].clone(), heard["result"].clone()),
        (
            json!("Sceneuron, mind blown"),
            json!(true),
            json!("played 🤯 Mind blown")
        )
    );
    assert_eq!(server.get_json("/api/state")["flood"]["flood"], "mind-blown");
    let revision = server.get_json("/api/state")["project_revision"].clone();
    command(&server, "hacker theme");
    assert_eq!(server.get_json("/api/project")["theme"], "hacker");
    assert_ne!(
        server.get_json("/api/state")["project_revision"],
        revision,
        "the editor catches up"
    );
    let view = command(&server, "order a pizza");
    assert_eq!(
        view["heard"]["result"], "no command matched, and Sceneuron's AI is not running to take others",
        "without the AI, nothing waits for it"
    );
    server.hub.update(|engine, _| engine.set_voice_host(true));
    let view = command(&server, "look now");
    assert_eq!(view["heard"]["result"], "asking the vision model to look");
    let waiting = server.hub.read(|engine| engine.voice_requests().to_vec());
    assert_eq!(waiting.len(), 1);
    assert_eq!(
        waiting[0].task,
        VoiceTask::Act {
            action: VoiceAction::Look
        }
    );
}

#[test]
fn malformed_commands_are_refused() {
    let server = TestServer::start();
    let long = json!({ "text": "a".repeat(201) }).to_string();
    for body in [
        "{}",
        r#"{"text":"   "}"#,
        r#"{"text":7}"#,
        r#"{"text":"lower","extra":1}"#,
        &long,
    ] {
        let reply = server.post_json("/api/voice/command", body);
        assert_eq!(reply.status, 400, "{body}: {}", reply.text());
    }
    assert_eq!(
        server.post_json("/api/voice/command", r#"{"text":""}"#).error(),
        "text must be 1 to 200 characters"
    );
    let wrong = server.api("GET", "/api/voice/command").send(server.port);
    assert_eq!((wrong.status, wrong.header("Allow")), (405, Some("POST")));
}

#[test]
fn the_reactions_voice_switches_save_with_the_project() {
    let server = TestServer::start();
    let mut project = server.get_json("/api/project");
    project["reactions"]["voice_off"] = json!(["email", "coffee"]);
    let saved = server
        .api("PUT", "/api/project")
        .body(project.to_string())
        .send(server.port);
    assert_eq!(saved.status, 200, "{}", saved.text());
    assert_eq!(
        server.get_json("/api/project")["reactions"]["voice_off"],
        json!(["email", "coffee"])
    );
    let off = server.hub.read(|engine| engine.project().reactions.voice_off.clone());
    assert_eq!(off, [Flood::Email, Flood::Coffee]);
    project["reactions"]["voice_off"] = json!(["tongue"]);
    let refused = server
        .api("PUT", "/api/project")
        .body(project.to_string())
        .send(server.port);
    assert_eq!(refused.status, 400);
    assert_eq!(
        refused.error(),
        r#"reactions.voice_off names "tongue", which has no trigger phrase"#
    );
}
