// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Vendor responses travel through `obs_data`, which cannot hold lists of plain values and
//! turns nulls into empty objects. Responses therefore carry the exact snapshot as a JSON
//! string plus a view that survives `obs_data` unchanged.

use serde_json::{Map, Value};

/// The snapshot reshaped for `obs_data`: null fields are left out and non-empty lists of plain
/// values become comma-separated strings, such as `"overridden": "visible,text"`. Empty lists
/// stay lists, which `obs_data` can hold.
pub fn obs_data_view(value: &Value) -> Value {
    match value {
        Value::Object(fields) => {
            let kept: Map<String, Value> = fields
                .iter()
                .filter(|(_, field)| !field.is_null())
                .map(|(name, field)| (name.clone(), obs_data_view(field)))
                .collect();
            Value::Object(kept)
        }
        Value::Array(items) if items.iter().all(Value::is_object) => {
            Value::Array(items.iter().map(obs_data_view).collect())
        }
        Value::Array(items) => {
            let words: Vec<String> = items
                .iter()
                .filter(|item| !item.is_null())
                .map(|item| match item {
                    Value::String(text) => text.clone(),
                    other => other.to_string(),
                })
                .collect();
            Value::String(words.join(","))
        }
        other => other.clone(),
    }
}
