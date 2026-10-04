// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Fixtures for the assistant tests: an assistant over a scripted planner, the settings update
//! that gives it a key and Gemini 3.1 Pro to plan with, whose fixed list prices the tests count,
//! a plan answer that applies to the default project, and a voice command answer.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use sceneuron_ai::{AiSettingsUpdate, Assistant, Credentials, MockProvider, ModelRequest, Usage};
use serde_json::{Value, json};

use super::TEST_KEY;
use super::director::connector;
use super::sink::RecordingSink;

pub fn with_key() -> AiSettingsUpdate {
    serde_json::from_value(json!({"api_key": TEST_KEY, "planner_model": "gemini-3.1-pro-preview"})).unwrap()
}

pub fn open(path: &Path, mock: &Arc<MockProvider>) -> (Assistant, Arc<RecordingSink>) {
    let sink = Arc::new(RecordingSink::default());
    let (assistant, warning) = Assistant::new(
        path.to_path_buf(),
        connector(mock),
        Arc::clone(&sink) as _,
        Credentials::default(),
    );
    assert_eq!(warning, None);
    (assistant, sink)
}

pub fn hide_label() -> Value {
    json!({"summary": "Hides the name label.", "operations": [
        {"op": "update_element", "id": "name-label", "props": {"visible": false}}
    ]})
}

/// A voice command answer: Ship it, subtitles into Spanish, and the name label hidden.
pub fn festive() -> Value {
    json!({"understood": "celebrate in Spanish", "actions": [
        {"action": "react", "reaction": "ship-it"},
        {"action": "subtitle_language", "language": "es"},
        {"action": "edit_project", "summary": "Hides the name label.", "operations": [
            {"op": "update_element", "id": "name-label", "props": {"visible": false}}
        ]}
    ]})
}

/// A mock that answers with `answers` in turn, repeating the last one.
pub fn answering(answers: Vec<Value>) -> Arc<MockProvider> {
    let next = AtomicUsize::new(0);
    let usage = Usage {
        input_tokens: 10_000,
        output_tokens: 1_000,
        ..Usage::default()
    };
    Arc::new(
        MockProvider::new(move |_: &ModelRequest| {
            let index = next.fetch_add(1, Ordering::SeqCst).min(answers.len() - 1);
            Ok(answers[index].clone())
        })
        .with_usage(usage),
    )
}
