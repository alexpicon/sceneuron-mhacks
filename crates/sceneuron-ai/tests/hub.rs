// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises hosting the AI in a hub: verdicts reach the engine and the overlay text, project and
//! presence changes reach the director, and the server's AI endpoints map settings, plans, and
//! provider errors.

mod common;

use std::sync::Arc;

use common::director::connector;
use common::sink::wait_until;
use common::{TEST_KEY, gray_frame, temp_dir};
use sceneuron_ai::{AiError, AiService, Assistant, Credentials, HubSink, MockProvider, ModelRequest, follow_hub};
use sceneuron_core::{Presence, Project, default_project};
use sceneuron_server::{Assistant as _, Hub, HubConfig, PlanMode, PlanRequest};
use serde_json::{Value, json};

fn holding_mug(request: &ModelRequest) -> Result<Value, AiError> {
    if request.schema_name.contains("plan") {
        return Ok(json!({"summary": "Hide the label.", "operations": [
            {"op": "update_element", "id": "name-label", "props": {"visible": false}}
        ]}));
    }
    Ok(json!({"caption": "A person holds up a mug.", "verdicts": [
        {"condition": "showing-item", "reason": "A mug is held toward the camera.", "holds": true,
         "confidence": 0.95, "detail": "coffee mug"},
        {"condition": "pet-in-view", "reason": "No animal is visible.", "holds": false,
         "confidence": 0.9, "detail": ""}
    ]}))
}

fn eager(mut project: Project) -> Project {
    project.ai.min_interval_ms = 1_000;
    project.ai.refresh_ms = 1_000;
    project
}

fn hosted(mock: &Arc<MockProvider>) -> (Arc<Hub>, Arc<Assistant>) {
    let hub = Hub::start(eager(default_project()), HubConfig::default()).unwrap();
    let sink = Arc::new(HubSink::new(Arc::clone(&hub)));
    let path = temp_dir("hub").join("ai.json");
    let (assistant, warning) = Assistant::new(path, connector(mock), sink, Credentials::default());
    assert_eq!(warning, None);
    let assistant = Arc::new(assistant);
    follow_hub(&hub, &assistant);
    (hub, assistant)
}

fn enable(service: &AiService) {
    let update = json!({"api_key": TEST_KEY, "consent": true, "enabled": true});
    let view = service.update_settings(update).unwrap();
    assert_eq!(
        (view["enabled"].as_bool(), view["has_key"].as_bool()),
        (Some(true), Some(true))
    );
}

#[test]
fn verdicts_turn_conditions_on_and_fill_overlay_text() {
    let mock = Arc::new(MockProvider::new(holding_mug));
    let (hub, assistant) = hosted(&mock);
    enable(&AiService::new(Arc::clone(&assistant)));
    assistant.director().offer_frame(gray_frame(64, 36, 120, hub.now_ms()));
    assert!(wait_until(|| hub.read(|engine| engine.condition_on("showing-item"))));
    let text = hub.read(|engine| engine.effective_props("now-showing").unwrap().text);
    assert_eq!(text, "Now showing: coffee mug");
    assert!(hub.read(|engine| engine.effective_props("now-showing").unwrap().visible));
    assert!(wait_until(|| hub.snapshot().ai.calls >= 1));
    assistant.shutdown();
    hub.shutdown();
}

#[test]
fn turning_scene_understanding_off_turns_conditions_off() {
    let mock = Arc::new(MockProvider::new(holding_mug));
    let (hub, assistant) = hosted(&mock);
    let service = AiService::new(Arc::clone(&assistant));
    enable(&service);
    assistant.director().offer_frame(gray_frame(64, 36, 120, hub.now_ms()));
    assert!(wait_until(|| hub.read(|engine| engine.condition_on("showing-item"))));
    service.update_settings(json!({"enabled": false})).unwrap();
    assert!(wait_until(|| !hub.read(|engine| engine.condition_on("showing-item"))));
    assert!(!hub.read(|engine| engine.effective_props("now-showing").unwrap().visible));
    assistant.shutdown();
    hub.shutdown();
}

#[test]
fn removing_every_condition_stops_the_calls() {
    let mock = Arc::new(MockProvider::new(holding_mug));
    let (hub, assistant) = hosted(&mock);
    enable(&AiService::new(Arc::clone(&assistant)));
    let mut project = eager(default_project());
    project.conditions.clear();
    project.reactions.enabled = false;
    project
        .rules
        .retain(|rule| matches!(rule.when, sceneuron_core::Trigger::Presence(_)));
    hub.try_update(|engine, now| engine.replace_project(project, now))
        .unwrap();
    assert!(wait_until(|| hub.snapshot().ai.state != "watching"));
    hub.update(|engine, now| engine.simulate_presence(Presence::Away, now));
    assistant.director().offer_frame(gray_frame(64, 36, 30, hub.now_ms()));
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert_eq!(mock.calls(), 0);
    assistant.shutdown();
    hub.shutdown();
}

#[test]
fn the_service_maps_settings_plans_and_errors() {
    let mock = Arc::new(MockProvider::new(holding_mug));
    let (hub, assistant) = hosted(&mock);
    let service = AiService::new(Arc::clone(&assistant));
    let settings = service.settings();
    assert_eq!(settings["available"], true);
    assert!(settings.get("api_key").is_none());
    assert_eq!(service.update_settings(json!({"nope": 1})).unwrap_err().status, 400);
    let retired = service.update_settings(json!({"provider": "anthropic"})).unwrap_err();
    assert_eq!(
        (retired.status, retired.message.as_str()),
        (
            400,
            "invalid AI settings: Anthropic is no longer offered; use gemini or openai-compatible"
        )
    );
    assert_eq!(service.analyze_now().unwrap_err().status, 400);
    enable(&service);
    assert!(service.analyze_now().is_ok());
    let request = PlanRequest {
        request: "hide my name".to_owned(),
        mode: PlanMode::Edit,
        use_camera: false,
    };
    let project = hub.read(|engine| engine.project().clone());
    let proposed = service.plan(&project, &request).unwrap();
    assert!(!proposed.project.element("name-label").unwrap().props.visible);
    assistant.shutdown();
    hub.shutdown();
}

#[test]
fn provider_failures_become_gateway_errors() {
    let mock = Arc::new(MockProvider::new(|_: &ModelRequest| {
        Err(AiError::Auth(
            "API key not valid. Please pass a valid API key.".to_owned(),
        ))
    }));
    let (hub, assistant) = hosted(&mock);
    let service = AiService::new(Arc::clone(&assistant));
    enable(&service);
    let request = PlanRequest {
        request: "anything".to_owned(),
        mode: PlanMode::Setup,
        use_camera: false,
    };
    let project = hub.read(|engine| engine.project().clone());
    let error = service.plan(&project, &request).unwrap_err();
    assert_eq!(error.status, 502);
    assert!(error.message.contains("rejected the API key"));
    let blank = PlanRequest {
        request: " ".to_owned(),
        ..request
    };
    assert_eq!(service.plan(&project, &blank).unwrap_err().status, 400);
    assistant.shutdown();
    hub.shutdown();
}

#[test]
fn quick_judgments_turn_conditions_on_through_the_hub() {
    let jev = common::server::MockServer::start(vec![common::server::Reply::json(
        200,
        json!({"answers": {"q0": {"type": "noul", "noul": 0.97}, "q1": {"type": "noul", "noul": 0.02}},
               "usage": {"input_tokens": 120, "output_tokens": 0}}),
    )]);
    let mock = Arc::new(MockProvider::new(holding_mug));
    let (hub, assistant) = hosted(&mock);
    let service = AiService::new(Arc::clone(&assistant));
    enable(&service);
    let update = json!({"jev_api_key": "jev-test-key-0123456789", "jev_base_url": jev.url()});
    let view = service.update_settings(update).unwrap();
    assert_eq!(
        (view["has_jev_key"].as_bool(), view["jev_key_source"].as_str()),
        (Some(true), Some("editor"))
    );
    assistant
        .director()
        .hint(vec!["holding a mug toward the camera".to_owned()], false, hub.now_ms());
    assert!(wait_until(|| hub.read(|engine| engine.condition_on("showing-item"))));
    let events = hub.snapshot().events;
    assert!(
        events
            .iter()
            .any(|e| e.detail.contains("quick judgment from on-device facts"))
    );
    assistant.shutdown();
    hub.shutdown();
}
