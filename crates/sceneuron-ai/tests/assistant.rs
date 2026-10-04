// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the assistant: settings that persist and reach the director, camera use that needs
//! consent and a frame, and request limits. Plans and their corrective retry are in
//! assistant_plans.rs.

mod common;

use std::sync::Arc;

use common::assistant::{answering, hide_label, open, with_key};
use common::director::connector;
use common::sink::{RecordingSink, wait_until};
use common::{TEST_KEY, gray_frame, temp_dir};
use sceneuron_ai::{AiError, AiSettingsUpdate, Assistant, Credentials, PlanMode};
use sceneuron_core::default_project;
use serde_json::json;

#[test]
fn settings_are_saved_reloaded_and_never_shown_with_the_key() {
    let path = temp_dir("assistant-settings").join("ai.json");
    let mock = answering(vec![hide_label()]);
    let (assistant, sink) = open(&path, &mock);
    let update: AiSettingsUpdate =
        serde_json::from_value(json!({"enabled": true, "consent": true, "api_key": TEST_KEY, "vision_effort": "high"}))
            .unwrap();
    let view = assistant.update_settings(update).unwrap();
    assert!(view.enabled && view.has_key && view.key_hint == "...abcd");
    assert_eq!(assistant.settings_view(), view);
    assert!(std::fs::read_to_string(&path).unwrap().contains(TEST_KEY));
    // The director picks the settings up; it has no conditions yet, so it idles.
    assert!(wait_until(|| sink
        .last_status()
        .is_some_and(|s| s.message.contains("add a condition"))));
    assistant.shutdown();

    let (reloaded, _) = open(&path, &mock);
    assert_eq!(reloaded.settings_view(), view);
    reloaded.shutdown();
}

#[test]
fn invalid_updates_change_nothing() {
    let path = temp_dir("assistant-invalid").join("ai.json");
    let mock = answering(vec![hide_label()]);
    let (assistant, _) = open(&path, &mock);
    let before = assistant.settings_view();
    let enable: AiSettingsUpdate = serde_json::from_value(json!({"enabled": true})).unwrap();
    let error = assistant.update_settings(enable).unwrap_err();
    assert_eq!(
        error,
        AiError::Config("confirm that camera images may be sent to the provider first".to_owned())
    );
    assert_eq!(assistant.settings_view(), before);
    assert!(!path.exists());
    assistant.shutdown();
}

#[test]
fn the_camera_needs_consent_and_a_frame() {
    let path = temp_dir("assistant-camera").join("ai.json");
    let mock = answering(vec![hide_label()]);
    let (assistant, _) = open(&path, &mock);
    assistant.update_settings(with_key()).unwrap();
    let project = default_project();
    let error = assistant
        .plan(&project, "Move alerts", PlanMode::Edit, true)
        .unwrap_err();
    assert_eq!(
        error,
        AiError::Config("confirm that camera images may be sent to the provider first".to_owned())
    );
    let consent: AiSettingsUpdate = serde_json::from_value(json!({"consent": true})).unwrap();
    assistant.update_settings(consent).unwrap();
    let error = assistant
        .plan(&project, "Move alerts", PlanMode::Edit, true)
        .unwrap_err();
    assert_eq!(
        error,
        AiError::Config("no camera image yet; add Sceneuron Camera Analysis to a camera source".to_owned())
    );
    assert_eq!(mock.calls(), 0);
    assistant.director().offer_frame(gray_frame(1920, 1080, 90, 5));
    assistant.plan(&project, "Move alerts", PlanMode::Edit, true).unwrap();
    let request = &mock.requests()[0];
    assert_eq!(request.images.len(), 1);
    let mut decoder = jpeg_decoder::Decoder::new(&request.images[0][..]);
    decoder.decode().unwrap();
    assert_eq!(decoder.info().unwrap().width, 768);
    assistant.shutdown();
}

#[test]
fn requests_must_be_one_to_two_thousand_characters() {
    let path = temp_dir("assistant-length").join("ai.json");
    let mock = answering(vec![hide_label()]);
    let (assistant, _) = open(&path, &mock);
    assistant.update_settings(with_key()).unwrap();
    let project = default_project();
    for request in ["", "   \n ", &"x".repeat(2_001)] {
        let error = assistant.plan(&project, request, PlanMode::Edit, false).unwrap_err();
        assert!(
            matches!(error, AiError::Config(ref m) if m.contains("2000")),
            "{error:?}"
        );
    }
    assert_eq!(mock.calls(), 0);
    let padded = format!("  {}  ", "y".repeat(2_000));
    assistant.plan(&project, &padded, PlanMode::Setup, false).unwrap();
    assert!(mock.requests()[0].text.starts_with("Mode: setup."));
    assistant.shutdown();
}

#[test]
fn planning_without_a_key_is_a_setup_problem() {
    let path = temp_dir("assistant-nokey").join("ai.json");
    let mock = answering(vec![hide_label()]);
    let (assistant, _) = open(&path, &mock);
    let error = assistant
        .plan(&default_project(), "Hide my name", PlanMode::Edit, false)
        .unwrap_err();
    assert!(
        matches!(error, AiError::Config(ref m) if m.contains("API key")),
        "{error:?}"
    );
    assistant.shutdown();
}

#[test]
fn a_bad_settings_file_gives_a_warning() {
    let dir = temp_dir("assistant-bad");
    let path = dir.join("ai.json");
    std::fs::write(&path, "[1, 2").unwrap();
    let mock = answering(vec![hide_label()]);
    let sink = Arc::new(RecordingSink::default());
    let (assistant, warning) = Assistant::new(path, connector(&mock), sink as _, Credentials::default());
    assert!(warning.unwrap().contains("using the defaults"));
    assert!(!assistant.settings_view().enabled);
    assistant.shutdown();
}
