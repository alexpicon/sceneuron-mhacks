// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Starting a director against a mock provider, with a connector that needs a key the way the
//! real one does.

use std::sync::Arc;

use sceneuron_ai::{AiError, AiSettings, Connector, Director, MockProvider, Provider};
use sceneuron_core::{AiPolicy, Condition};
use serde_json::{Value, json};

use super::condition;
use super::sink::RecordingSink;

/// A policy that calls on every new frame: no minimum interval and an immediate refresh.
pub fn eager_policy() -> AiPolicy {
    AiPolicy {
        min_interval_ms: 0,
        refresh_ms: 0,
        ..AiPolicy::default()
    }
}

pub fn pet_condition() -> Vec<Condition> {
    vec![condition(
        "pet",
        "A cat or dog is clearly visible.",
        "the kind of animal",
    )]
}

/// The answer a vision model gives when it sees a cat.
pub fn cat_answer() -> Value {
    json!({"caption": "A cat sits on a desk.", "verdicts": [
        {"condition": "pet", "reason": "A cat sits on the desk.", "holds": true, "confidence": 0.9, "detail": "Cat"}
    ]})
}

/// Hands out `mock`, or refuses like the real connector when the key is missing.
pub fn connector(mock: &Arc<MockProvider>) -> Connector {
    let provider = Arc::clone(mock);
    Arc::new(move |settings: &AiSettings| {
        if settings.api_key.is_empty() {
            return Err(AiError::Config("add a Gemini API key in the AI settings".to_owned()));
        }
        Ok(Arc::clone(&provider) as Arc<dyn Provider>)
    })
}

/// A director with `settings` and a recording sink.
pub fn start(settings: AiSettings, mock: &Arc<MockProvider>) -> (Director, Arc<RecordingSink>) {
    let sink = Arc::new(RecordingSink::default());
    let director = Director::spawn(settings, connector(mock), Arc::clone(&sink) as _);
    (director, sink)
}

/// A director that watches the pet condition on every new frame and is ready.
pub fn start_watching(mock: &Arc<MockProvider>) -> (Director, Arc<RecordingSink>) {
    let (director, sink) = start(super::enabled_settings(), mock);
    director.set_project(pet_condition(), eager_policy(), false);
    sink.wait_for_state("watching");
    (director, sink)
}
