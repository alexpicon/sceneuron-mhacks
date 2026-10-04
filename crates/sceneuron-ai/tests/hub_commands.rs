// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises voice requests in a hub against a scripted planner: words no local command knows go
//! to the AI once its key is set, and its actions change the overlay and the saved project while
//! the heard line and the event log say what was understood and done; without a key the heard
//! line says how to set the AI up; a look is answered without a model call; and local commands
//! never reach the model.

mod common;

use std::sync::Arc;

use common::assistant::{answering, with_key};
use common::director::connector;
use common::sink::wait_until;
use common::temp_dir;
use sceneuron_ai::{Assistant, Credentials, HubSink, MockProvider, follow_hub};
use sceneuron_core::{Flood, default_project, pack_for_canvas};
use sceneuron_server::{Hub, HubConfig};
use serde_json::json;

fn hosted(mock: &Arc<MockProvider>) -> (Arc<Hub>, Arc<Assistant>) {
    let base = default_project();
    let mut project = base
        .add_pack(&pack_for_canvas("meeting", &base.canvas).unwrap())
        .unwrap();
    project.decks[0].voice = true;
    let hub = Hub::start(project, HubConfig::default()).unwrap();
    let sink = Arc::new(HubSink::new(Arc::clone(&hub)));
    let path = temp_dir("hub-commands").join("ai.json");
    let (assistant, warning) = Assistant::new(path, connector(mock), sink, Credentials::default());
    assert_eq!(warning, None);
    let assistant = Arc::new(assistant);
    follow_hub(&hub, &assistant);
    (hub, assistant)
}

fn result(hub: &Hub) -> String {
    hub.read(|engine| engine.voice_view().heard.map(|heard| heard.result).unwrap_or_default())
}

fn say(hub: &Hub, words: &str) {
    hub.update(|engine, now| engine.type_command(words, now));
}

#[test]
fn words_for_the_ai_change_the_overlay_and_the_project_and_say_so() {
    let answer = json!({"understood": "celebrate", "actions": [
        {"action": "react", "reaction": "ship-it"},
        {"action": "raise_card", "deck": "meeting", "card": "agree"},
        {"action": "edit_project", "summary": "Hides the name label.", "operations": [
            {"op": "update_element", "id": "name-label", "props": {"visible": false}}
        ]}
    ]});
    let mock = answering(vec![answer]);
    let (hub, assistant) = hosted(&mock);
    say(&hub, "Sceneuron, celebrate");
    let unset = "no command matched, and the AI that takes any other is not set up: add a Gemini API key in the AI \
                 settings";
    assert!(wait_until(|| result(&hub) == unset), "{}", result(&hub));
    assert_eq!(mock.calls(), 0);
    assistant.update_settings(with_key()).unwrap();
    say(&hub, "Sceneuron, celebrate");
    let expected = "the AI took it as \"celebrate\": played 🚀 Ship it; raised Meeting signals: Agree; changed the \
                    project: Hides the name label.";
    assert!(wait_until(|| result(&hub) == expected), "{}", result(&hub));
    let (flood, raised, label) = hub.read(|engine| {
        let label = engine.project().element("name-label").unwrap().props.visible;
        (
            engine.flood().map(|flood| flood.flood),
            engine.raised_card("meeting").map(str::to_owned),
            label,
        )
    });
    assert_eq!(
        (flood, raised.as_deref(), label),
        (Some(Flood::ShipIt), Some("agree"), false)
    );
    let logged = hub.read(|engine| engine.snapshot(0).events.iter().any(|event| event.detail == expected));
    assert!(logged, "the event log says it too");
    assert!(hub.read(|engine| engine.voice_requests().is_empty()));
    assert_eq!(mock.calls(), 1);
    assistant.shutdown();
    hub.shutdown();
}

#[test]
fn a_look_and_local_commands_make_no_model_call() {
    let mock = answering(vec![json!({"understood": "x", "actions": []})]);
    let (hub, assistant) = hosted(&mock);
    assistant.update_settings(with_key()).unwrap();
    say(&hub, "look now");
    let cannot = "the vision model cannot look: turn on scene understanding in the AI tab";
    assert!(wait_until(|| result(&hub) == cannot), "{}", result(&hub));
    say(&hub, "mind blown");
    assert_eq!(result(&hub), "played 🤯 Mind blown");
    say(&hub, "vaporwave theme");
    assert_eq!(result(&hub), "switched the overlay to the Vaporwave theme");
    say(&hub, "wrap it up");
    assert_eq!(result(&hub), "raised Meeting signals: Wrap it up");
    assert_eq!(mock.calls(), 0);
    assistant.shutdown();
    hub.shutdown();
}
