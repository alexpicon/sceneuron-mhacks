// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! JSON answers without structured output: the schema in the instructions, and finding the
//! answer in text that may wrap it in prose or markdown code fences.

use serde_json::Value;

use crate::provider::AiError;

/// Instructions for a server that cannot enforce the schema itself.
pub(crate) fn with_schema(system: &str, schema: &Value) -> String {
    format!("{system}\n\nAnswer with only one JSON object, without markdown, that matches this JSON schema:\n{schema}")
}

/// Parses `text` as one JSON object, or else the first complete JSON object inside it.
pub(crate) fn extract_object(text: &str) -> Result<Value, AiError> {
    let trimmed = text.trim();
    if let Ok(value @ Value::Object(_)) = serde_json::from_str::<Value>(trimmed) {
        return Ok(value);
    }
    for (start, _) in trimmed.match_indices('{') {
        let mut values = serde_json::Deserializer::from_str(&trimmed[start..]).into_iter::<Value>();
        if let Some(Ok(value @ Value::Object(_))) = values.next() {
            return Ok(value);
        }
    }
    let message = if trimmed.is_empty() {
        "the answer was empty".to_owned()
    } else {
        "the answer did not contain a JSON object".to_owned()
    };
    Err(AiError::Invalid(message))
}
