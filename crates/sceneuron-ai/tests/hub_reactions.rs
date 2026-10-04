// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the tongue-out reaction hosted in a hub, from on-device facts to the overlay: an open
//! mouth brings a look, the scripted vision model sees a tongue, the engine floods the overlay and
//! logs why, and the reaction turns off when the mouth closes. Switched off, the reaction is never
//! asked about.

#![cfg(feature = "hub")]

mod common;

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use common::sink::wait_until;
use common::{TEST_KEY, gray_frame, temp_dir};
use sceneuron_ai::{AiService, Assistant, Credentials, HubSink, MockProvider, ModelRequest, follow_hub};
use sceneuron_core::{Flood, Project, TONGUE_OUT, default_project};
use sceneuron_server::{Assistant as _, Hub, HubConfig};
use serde_json::json;

/// A model that sees a tongue whenever it is asked about one, and nothing else.
fn tongue_model() -> Arc<MockProvider> {
    Arc::new(MockProvider::new(|request: &ModelRequest| {
        let verdicts = if request.text.contains(TONGUE_OUT) {
            json!([{"condition": TONGUE_OUT, "reason": "The tongue sticks out past the lips.",
                    "holds": true, "confidence": 0.95, "detail": ""}])
        } else {
            json!([])
        };
        Ok(json!({"caption": "A person at a desk.", "verdicts": verdicts}))
    }))
}

fn hosted(project: Project, mock: &Arc<MockProvider>) -> (Arc<Hub>, Arc<Assistant>) {
    let hub = Hub::start(project, HubConfig::default()).unwrap();
    let sink = Arc::new(HubSink::new(Arc::clone(&hub)));
    let path = temp_dir("hub-reactions").join("ai.json");
    let (assistant, warning) = Assistant::new(path, mock.connector(), sink, Credentials::default());
    assert_eq!(warning, None);
    let assistant = Arc::new(assistant);
    follow_hub(&hub, &assistant);
    let service = AiService::new(Arc::clone(&assistant));
    let update = json!({"api_key": TEST_KEY, "consent": true, "enabled": true});
    service.update_settings(update).unwrap();
    (hub, assistant)
}

/// Reports the mouth as the camera would, five times a second for `ms`.
fn show_mouth(hub: &Hub, assistant: &Assistant, mouth: &str, ms: u64) {
    for _ in 0..=ms / 200 {
        let facts = vec!["face in view (0.97)".to_owned(), mouth.to_owned()];
        assistant.director().hint(facts, false, hub.now_ms());
        thread::sleep(Duration::from_millis(200));
    }
}

fn events(hub: &Hub) -> Vec<String> {
    hub.snapshot().events.into_iter().map(|event| event.detail).collect()
}

#[test]
fn sticking_the_tongue_out_floods_the_overlay_and_closing_the_mouth_ends_the_reaction() {
    let mock = tongue_model();
    let mut project = default_project();
    project.conditions.clear();
    project
        .rules
        .retain(|rule| matches!(rule.when, sceneuron_core::Trigger::Presence(_)));
    let (hub, assistant) = hosted(project, &mock);
    assert!(wait_until(|| hub.snapshot().ai.message.contains("mouth open")));
    show_mouth(&hub, &assistant, "mouth closed (0.01)", 400);
    assert_eq!(mock.calls(), 0, "a look while the mouth was closed");
    show_mouth(&hub, &assistant, "mouth open (0.26)", 600);
    assert!(wait_until(|| assistant.director().wants_frame(hub.now_ms())));
    assistant.director().offer_frame(gray_frame(64, 36, 120, hub.now_ms()));
    assert!(wait_until(|| hub.snapshot().flood.is_some()), "no flood");
    assert_eq!(hub.snapshot().flood.unwrap().flood, Flood::Tongue);
    assert!(hub.read(|engine| engine.condition_on(TONGUE_OUT)));
    let log = events(&hub);
    assert!(
        log.contains(&r#"ran "Tongue-out flood" because Tongue out turned on"#.to_owned()),
        "{log:?}"
    );
    assert!(log.contains(&"Tongue out turned on: The tongue sticks out past the lips.".to_owned()));
    show_mouth(&hub, &assistant, "mouth closed (0.02)", 200);
    assert!(wait_until(|| !hub.read(|engine| engine.condition_on(TONGUE_OUT))));
    assert!(events(&hub).contains(&"Tongue out turned off: on-device analysis no longer sees mouth open".to_owned()));
    assert_eq!(mock.calls(), 1);
    assistant.shutdown();
    hub.shutdown();
}

#[test]
fn switched_off_the_reaction_is_never_asked_about() {
    let mock = tongue_model();
    let mut project = default_project();
    project.reactions.enabled = false;
    let (hub, assistant) = hosted(project, &mock);
    show_mouth(&hub, &assistant, "mouth open (0.3)", 600);
    assistant.director().offer_frame(gray_frame(64, 36, 120, hub.now_ms()));
    assert!(
        wait_until(|| mock.calls() >= 1),
        "the project's conditions are still judged"
    );
    assert!(mock.requests().iter().all(|request| !request.text.contains(TONGUE_OUT)));
    assert_eq!(hub.snapshot().flood, None);
    assistant.shutdown();
    hub.shutdown();
}
