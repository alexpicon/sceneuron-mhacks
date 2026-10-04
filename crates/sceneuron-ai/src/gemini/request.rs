// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The `generateContent` request: the model id that goes into the URL path, the body, thinking,
//! and the schema rewritten into the JSON Schema subset Gemini documents.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Map, Value, json};

use crate::json_text::with_schema;
use crate::provider::{AiError, ModelRequest};

/// Keywords whose value is one schema or a list of schemas.
const SUBSCHEMAS: [&str; 7] = [
    "items",
    "prefixItems",
    "additionalProperties",
    "anyOf",
    "oneOf",
    "allOf",
    "not",
];
/// Keywords whose value maps names to schemas.
const NAMED_SUBSCHEMAS: [&str; 4] = ["properties", "$defs", "definitions", "patternProperties"];

/// Where a request's schema goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SchemaPlace {
    /// `generationConfig.responseFormat`, the current structured output field.
    ResponseFormat,
    /// `generationConfig.responseMimeType` with `responseJsonSchema`, the older one.
    ResponseJsonSchema,
    /// The system instruction, where nothing holds the model to it.
    Instructions,
}

/// The model id as it goes into the URL path: without one optional `models/` prefix, and only
/// when every character is a letter, digit, dot, hyphen, or underscore, so a model name can
/// never change the path or add a query.
pub(super) fn model_id(model: &str) -> Result<&str, AiError> {
    let id = model.strip_prefix("models/").unwrap_or(model);
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_');
    if !id.is_empty() && id.chars().all(allowed) {
        Ok(id)
    } else {
        let message =
            format!("the Gemini model {model:?} must be a name of letters, digits, dots, hyphens, and underscores");
        Err(AiError::Config(message))
    }
}

/// The `generateContent` body. The images come before the text, as Gemini's image guide
/// recommends for a single image; sampling settings such as temperature are left to the model.
pub(super) fn request_body(request: &ModelRequest, model: &str, schema: SchemaPlace) -> Value {
    let mut parts: Vec<Value> = request
        .images
        .iter()
        .map(|jpeg| json!({"inlineData": {"mimeType": "image/jpeg", "data": STANDARD.encode(jpeg)}}))
        .collect();
    parts.push(json!({"text": request.text}));
    let mut config = json!({"maxOutputTokens": request.max_tokens});
    if let Some(thinking) = thinking_config(model, &request.effort) {
        config["thinkingConfig"] = thinking;
    }
    let system = match schema {
        SchemaPlace::ResponseFormat => {
            let format = json!({"mimeType": "APPLICATION_JSON", "schema": gemini_schema(&request.schema)});
            config["responseFormat"] = json!({"text": format});
            request.system.clone()
        }
        SchemaPlace::ResponseJsonSchema => {
            config["responseMimeType"] = json!("application/json");
            config["responseJsonSchema"] = gemini_schema(&request.schema);
            request.system.clone()
        }
        SchemaPlace::Instructions => with_schema(&request.system, &request.schema),
    };
    json!({
        "systemInstruction": {"parts": [{"text": system}]},
        "contents": [{"role": "user", "parts": parts}],
        "generationConfig": config,
    })
}

/// Thinking for an effort of `low`, `medium`, or `high`: a level on Gemini 3 and later, and a
/// token budget on Gemini 2, which refuses levels. Any other effort, such as the empty one,
/// leaves thinking at the model's default.
fn thinking_config(model: &str, effort: &str) -> Option<Value> {
    let (level, budget) = match effort {
        "low" => ("LOW", 1_024),
        "medium" => ("MEDIUM", 8_192),
        "high" => ("HIGH", 24_576),
        _ => return None,
    };
    if model.starts_with("gemini-2.") {
        Some(json!({"thinkingBudget": budget}))
    } else {
        Some(json!({"thinkingLevel": level}))
    }
}

/// `schema` in the JSON Schema subset Gemini documents, which has `enum` but not `const`: every
/// `{"const": value}` becomes `{"enum": [value]}`, at any depth. Property names and values such
/// as `enum` lists are left alone.
pub(super) fn gemini_schema(schema: &Value) -> Value {
    let Value::Object(keywords) = schema else {
        return schema.clone();
    };
    let mut rewritten: Map<String, Value> = keywords
        .iter()
        .map(|(keyword, value)| {
            let value = if SUBSCHEMAS.contains(&keyword.as_str()) {
                subschemas(value)
            } else if NAMED_SUBSCHEMAS.contains(&keyword.as_str()) {
                named_subschemas(value)
            } else {
                value.clone()
            };
            (keyword.clone(), value)
        })
        .collect();
    // A constant is narrower than any enum beside it, so it replaces one.
    if let Some(constant) = rewritten.remove("const") {
        rewritten.insert("enum".to_owned(), json!([constant]));
    }
    Value::Object(rewritten)
}

fn subschemas(value: &Value) -> Value {
    match value {
        Value::Array(schemas) => schemas.iter().map(gemini_schema).collect(),
        schema => gemini_schema(schema),
    }
}

fn named_subschemas(value: &Value) -> Value {
    match value {
        Value::Object(schemas) => Value::Object(
            schemas
                .iter()
                .map(|(name, schema)| (name.clone(), gemini_schema(schema)))
                .collect(),
        ),
        other => other.clone(),
    }
}
