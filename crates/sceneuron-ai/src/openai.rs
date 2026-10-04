// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! OpenAI-style chat completions, as served by NVIDIA NIM, Ollama, llama.cpp, vLLM, and others.
//!
//! These servers differ in what they accept, so the client degrades gracefully: when JSON schema
//! output is refused it asks for a plain JSON object instead, and it finds the answer even when
//! a model wraps it in prose or code fences.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use crate::http::{Failure, Http, mentions, with_retries};
use crate::json_text::{extract_object, with_schema};
use crate::provider::{AiError, ModelReply, ModelRequest, Provider, Usage};
use crate::settings::AiSettings;

/// Calls `{base_url}/chat/completions`.
pub(crate) struct OpenAiProvider {
    http: Http,
    url: String,
    authorization: Option<String>,
    /// Cleared for good when the server rejects JSON schema output.
    json_schema: AtomicBool,
}

impl OpenAiProvider {
    pub fn new(settings: &AiSettings) -> OpenAiProvider {
        let key = &settings.api_key;
        OpenAiProvider {
            http: Http::new(&settings.base_url, key),
            url: format!("{}/chat/completions", settings.base_url.trim_end_matches('/')),
            authorization: (!key.is_empty()).then(|| format!("Bearer {key}")),
            json_schema: AtomicBool::new(true),
        }
    }

    fn attempt(&self, body: &[u8], remaining: Duration) -> Result<Answer, Failure> {
        let headers: Vec<(&str, &str)> = self
            .authorization
            .iter()
            .map(|value| ("authorization", value.as_str()))
            .collect();
        let reply = self.http.post(&self.url, &headers, body, remaining)?;
        if !(200..300).contains(&reply.status) {
            return Err(self.http.status_error(&reply));
        }
        parse_response(&reply.body).map_err(Failure::from)
    }
}

impl Provider for OpenAiProvider {
    fn complete(&self, request: &ModelRequest) -> Result<ModelReply, AiError> {
        let started = Instant::now();
        let deadline = started + request.timeout;
        loop {
            let json_schema = self.json_schema.load(Ordering::Relaxed);
            let body = request_body(request, json_schema).to_string().into_bytes();
            match with_retries(deadline, |remaining| self.attempt(&body, remaining)) {
                Ok(answer) => {
                    return Ok(ModelReply {
                        json: answer.json,
                        usage: answer.usage,
                        model: answer.model.unwrap_or_else(|| request.model.clone()),
                        latency_ms: started.elapsed().as_secs_f64() * 1000.0,
                    });
                }
                // Switching off schema output happens once, so this retry happens at most once.
                Err(error @ AiError::BadRequest(_))
                    if json_schema && mentions(&error, &["response_format", "json_schema"]) =>
                {
                    self.json_schema.store(false, Ordering::Relaxed);
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn kind(&self) -> &'static str {
        "openai-compatible"
    }
}

/// The chat completions body. Without schema support the schema moves into the instructions.
pub(crate) fn request_body(request: &ModelRequest, json_schema: bool) -> Value {
    let (system, response_format) = if json_schema {
        let format = json!({
            "type": "json_schema",
            "json_schema": {"name": request.schema_name, "schema": request.schema, "strict": true},
        });
        (request.system.clone(), format)
    } else {
        (
            with_schema(&request.system, &request.schema),
            json!({"type": "json_object"}),
        )
    };
    // Plain text keeps text-only requests working on servers without multimodal content parts.
    let user = if request.images.is_empty() {
        json!(request.text)
    } else {
        let mut parts = vec![json!({"type": "text", "text": request.text})];
        parts.extend(request.images.iter().map(|jpeg| {
            let url = format!("data:image/jpeg;base64,{}", STANDARD.encode(jpeg));
            json!({"type": "image_url", "image_url": {"url": url}})
        }));
        Value::Array(parts)
    };
    json!({
        "model": request.model,
        "max_tokens": request.max_tokens,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "response_format": response_format,
    })
}

struct Answer {
    json: Value,
    usage: Usage,
    model: Option<String>,
}

fn parse_response(body: &str) -> Result<Answer, AiError> {
    let response: Value =
        serde_json::from_str(body).map_err(|_| AiError::Invalid("the server's response is not JSON".to_owned()))?;
    let choice = response
        .pointer("/choices/0")
        .ok_or_else(|| AiError::Invalid("the server's response has no answer".to_owned()))?;
    let refusal = choice.pointer("/message/refusal").and_then(Value::as_str);
    if let Some(refusal) = refusal.filter(|text| !text.trim().is_empty()) {
        return Err(AiError::Refused(refusal.to_owned()));
    }
    match choice.get("finish_reason").and_then(Value::as_str) {
        Some("length") => {
            let message = "the answer was cut off at the token limit".to_owned();
            return Err(AiError::Invalid(message));
        }
        Some("content_filter") => {
            let message = "the server's content filter blocked the answer".to_owned();
            return Err(AiError::Refused(message));
        }
        _ => {}
    }
    let tokens = |field: &str| {
        response
            .pointer(&format!("/usage/{field}"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    Ok(Answer {
        json: extract_object(&content_text(choice.pointer("/message/content")))?,
        usage: Usage {
            input_tokens: tokens("prompt_tokens"),
            output_tokens: tokens("completion_tokens"),
            ..Usage::default()
        },
        model: response.get("model").and_then(Value::as_str).map(str::to_owned),
    })
}

/// Message content is a string on most servers and a list of text parts on some.
fn content_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect(),
        _ => String::new(),
    }
}
