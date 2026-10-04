// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Fixtures for the Gemini client tests: settings pointing at a scripted server, a request with
//! one image, answers in the shape of `generateContent`, and Google's error bodies, those that
//! refuse a key included.

use std::time::Duration;

use sceneuron_ai::{AiSettings, ModelRequest, ProviderKind};
use serde_json::{Value, json};

use super::TEST_KEY;
use super::server::{MockServer, Reply};

pub const IMAGE: [u8; 6] = [0xFF, 0xD8, 0xFF, 0xE0, 3, 4];
/// Where the default request goes.
pub const PATH: &str = "/v1beta/models/gemini-3.8-flash:generateContent";

pub fn settings(server: &MockServer) -> AiSettings {
    AiSettings {
        provider: ProviderKind::Gemini,
        base_url: server.url(),
        api_key: TEST_KEY.to_owned(),
        ..AiSettings::default()
    }
}

pub fn request() -> ModelRequest {
    ModelRequest {
        model: "gemini-3.8-flash".to_owned(),
        effort: "low".to_owned(),
        system: "You judge camera frames.".to_owned(),
        text: "Is a cat visible?".to_owned(),
        images: vec![IMAGE.to_vec()],
        schema: json!({"type": "object", "properties": {"answer": {"type": "integer"}},
            "required": ["answer"], "additionalProperties": false}),
        schema_name: "answer".to_owned(),
        max_tokens: 4000,
        timeout: Duration::from_secs(20),
    }
}

/// A successful response with `candidate` as its only candidate.
pub fn answer(candidate: Value) -> Reply {
    Reply::json(
        200,
        json!({
            "candidates": [candidate],
            "usageMetadata": {"promptTokenCount": 1300, "cachedContentTokenCount": 1000,
                "candidatesTokenCount": 7, "thoughtsTokenCount": 110, "totalTokenCount": 1417},
            "modelVersion": "gemini-3.8-flash",
            "responseId": "test-response",
        }),
    )
}

/// `{"answer": 42}`, split over two parts, after a thought summary that drafts another answer,
/// finished normally.
pub fn success() -> Reply {
    answer(json!({
        "content": {"role": "model", "parts": [
            {"text": "Draft: {\"answer\": 1}", "thought": true},
            {"text": "{\"answer\": ", "thoughtSignature": "c2lnbmF0dXJl"},
            {"text": "42}"}
        ]},
        "finishReason": "STOP",
        "index": 0
    }))
}

/// A candidate that stopped for `reason`, with `message` as its finish message when given.
pub fn finished(reason: &str, message: Option<&str>) -> Reply {
    let mut candidate = json!({
        "content": {"role": "model", "parts": [{"text": "{\"answer\": "}]},
        "finishReason": reason,
        "index": 0
    });
    if let Some(message) = message {
        candidate["finishMessage"] = json!(message);
    }
    answer(candidate)
}

/// Google's error body: the HTTP status as `code`, the gRPC status name, and a message.
pub fn error(code: u16, status: &str, message: &str) -> Reply {
    Reply::json(
        code,
        json!({"error": {"code": code, "message": message, "status": status}}),
    )
}

/// How `generateContent` answers a key it does not know.
pub fn invalid_key() -> Reply {
    Reply::json(
        400,
        json!({"error": {"code": 400, "message": "API key not valid. Please pass a valid API key.",
            "status": "INVALID_ARGUMENT", "details": [{"@type": "type.googleapis.com/google.rpc.ErrorInfo",
            "reason": "API_KEY_INVALID", "domain": "googleapis.com",
            "metadata": {"service": "generativelanguage.googleapis.com"}}]}}),
    )
}

/// Google's body for a key it refuses, with `reason` in its `google.rpc.ErrorInfo` detail.
pub fn key_refused(code: u16, status: &str, reason: &str, message: &str) -> Reply {
    Reply::json(
        code,
        json!({"error": {"code": code, "message": message, "status": status, "details": [
            {"@type": "type.googleapis.com/google.rpc.ErrorInfo", "reason": reason, "domain": "googleapis.com"}
        ]}}),
    )
}

/// Every object key anywhere in `value`.
pub fn keys(value: &Value) -> Vec<String> {
    match value {
        Value::Object(fields) => fields
            .iter()
            .flat_map(|(key, value)| std::iter::once(key.clone()).chain(keys(value)))
            .collect(),
        Value::Array(items) => items.iter().flat_map(keys).collect(),
        _ => Vec::new(),
    }
}
