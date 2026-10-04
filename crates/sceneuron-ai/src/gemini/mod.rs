// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Google's Gemini API (`generateContent`) over raw HTTPS: schema-constrained JSON answers,
//! thinking levels, and Gemini's own refusals, usage, and errors. The key travels in a header,
//! never in the URL.
//!
//! Gemini documents no numeric limits for schemas, so every schema, the plan's included, goes
//! as structured output until the endpoint refuses it; which way a request's schema went is
//! logged at debug level.

mod request;
mod response;

use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

use crate::cost::gemini_prices;
use crate::http::{Failure, Http, mentions, with_retries};
use crate::provider::{AiError, ModelReply, ModelRequest, Provider};
use crate::settings::AiSettings;
use request::{SchemaPlace, model_id, request_body};
use response::{Answer, parse_response, status_error};

/// Calls `{base_url}/v1beta/models/{model}:generateContent`.
pub(crate) struct GeminiProvider {
    http: Http,
    base_url: String,
    api_key: String,
    /// Cleared for good when the endpoint refuses `responseFormat` itself by name; every schema
    /// then goes as `responseMimeType` with `responseJsonSchema`.
    response_format: AtomicBool,
    /// Schemas the endpoint refused inside `responseFormat`; those requests use the older fields.
    older_fields: Mutex<HashSet<String>>,
    /// Schemas the endpoint would not take as structured output; those requests carry the
    /// schema in the instructions instead.
    unstructured: Mutex<HashSet<String>>,
}

impl GeminiProvider {
    pub fn new(settings: &AiSettings) -> Result<GeminiProvider, AiError> {
        if settings.api_key.is_empty() {
            let message = "add a Gemini API key in the AI settings".to_owned();
            return Err(AiError::Config(message));
        }
        Ok(GeminiProvider {
            http: Http::new(&settings.base_url, &settings.api_key),
            base_url: settings.base_url.trim_end_matches('/').to_owned(),
            api_key: settings.api_key.clone(),
            response_format: AtomicBool::new(true),
            older_fields: Mutex::new(HashSet::new()),
            unstructured: Mutex::new(HashSet::new()),
        })
    }

    /// Where the request's schema goes: in the instructions once the endpoint refused it,
    /// otherwise as structured output in whichever field the endpoint still takes for it.
    fn schema_place(&self, request: &ModelRequest) -> SchemaPlace {
        let name = &request.schema_name;
        let (place, mode) = if contains(&self.unstructured, name) {
            (
                SchemaPlace::Instructions,
                "in the instructions, because the endpoint refused it as structured output",
            )
        } else if self.response_format.load(Ordering::Relaxed) && !contains(&self.older_fields, name) {
            (SchemaPlace::ResponseFormat, "as structured output")
        } else {
            (
                SchemaPlace::ResponseJsonSchema,
                "as structured output in responseJsonSchema",
            )
        };
        log::debug!("the {name} schema goes {mode}");
        place
    }

    fn attempt(&self, url: &str, body: &[u8], remaining: Duration) -> Result<Answer, Failure> {
        let headers = [("x-goog-api-key", self.api_key.as_str())];
        let reply = self.http.post(url, &headers, body, remaining)?;
        if !(200..300).contains(&reply.status) {
            return Err(status_error(&self.http, &reply));
        }
        parse_response(&reply.body).map_err(Failure::from)
    }
}

impl Provider for GeminiProvider {
    fn complete(&self, request: &ModelRequest) -> Result<ModelReply, AiError> {
        let started = Instant::now();
        let deadline = started + request.timeout;
        let model = model_id(&request.model)?;
        let url = format!("{}/v1beta/models/{model}:generateContent", self.base_url);
        // Each fallback below moves this schema to a later place for good, so this loop runs at
        // most 3 times.
        loop {
            let place = self.schema_place(request);
            let body = request_body(request, model, place).to_string().into_bytes();
            match with_retries(deadline, |remaining| self.attempt(&url, &body, remaining)) {
                Ok(answer) => {
                    return Ok(ModelReply {
                        json: answer.json,
                        usage: answer.usage,
                        model: reply_model(answer.model, &request.model),
                        latency_ms: started.elapsed().as_secs_f64() * 1000.0,
                    });
                }
                // A refusal that names a schema inside `responseFormat` is about that schema, so
                // only it moves to the older fields; one that names no schema is about the field.
                Err(error @ AiError::BadRequest(_))
                    if place == SchemaPlace::ResponseFormat
                        && mentions(&error, &["response_format", "responseformat"]) =>
                {
                    if mentions(&error, &["schema"]) {
                        insert(&self.older_fields, &request.schema_name);
                    } else {
                        self.response_format.store(false, Ordering::Relaxed);
                    }
                }
                Err(error @ AiError::BadRequest(_))
                    if place != SchemaPlace::Instructions && mentions(&error, &["schema"]) =>
                {
                    insert(&self.unstructured, &request.schema_name);
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn kind(&self) -> &'static str {
        "gemini"
    }
}

/// The model a reply names: the version Gemini reports, unless only the model asked for has a
/// known price, so a version id the price table lacks still counts toward the hourly budget.
fn reply_model(version: Option<String>, asked: &str) -> String {
    let now = SystemTime::now();
    match version {
        Some(version) if gemini_prices(&version, now).is_some() || gemini_prices(asked, now).is_none() => version,
        _ => asked.to_owned(),
    }
}

fn contains(names: &Mutex<HashSet<String>>, name: &str) -> bool {
    names
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .contains(name)
}

fn insert(names: &Mutex<HashSet<String>>, name: &str) {
    names
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(name.to_owned());
}
