// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Scene understanding: the request that asks a vision model to judge the streamer's conditions
//! in one camera frame, and the reading of its answer.

use std::fmt::Write as _;
use std::time::Duration;

use sceneuron_core::{Condition, SceneObservation};
use serde_json::{Value, json};

use crate::provider::{AiError, ModelRequest};
use crate::settings::AiSettings;

/// Instructions shared by every judge call.
pub const JUDGE_PROMPT: &str = include_str!("prompts/judge.txt");
/// Room for the model's thinking plus a verdict per condition.
const MAX_TOKENS: u32 = 4000;
const TIMEOUT: Duration = Duration::from_secs(30);
const MAX_HINTS: usize = 16;
const MAX_HINT_CHARS: usize = 200;

/// Builds the request that judges every enabled condition in one JPEG camera frame.
///
/// `hints` are facts from local detectors, passed along as possibly wrong hints.
pub fn judge_request(conditions: &[Condition], hints: &[String], jpeg: Vec<u8>, settings: &AiSettings) -> ModelRequest {
    let enabled: Vec<&Condition> = conditions.iter().filter(|condition| condition.enabled).collect();
    ModelRequest {
        model: settings.vision_model.clone(),
        effort: settings.vision_effort.clone(),
        system: JUDGE_PROMPT.to_owned(),
        text: judge_text(&enabled, hints),
        images: vec![jpeg],
        schema: observation_schema(&enabled),
        schema_name: "scene_observation".to_owned(),
        max_tokens: MAX_TOKENS,
        timeout: TIMEOUT,
    }
}

/// Reads the model's answer into an observation with bounded, single-line text.
pub fn parse_observation(json: &Value) -> Result<SceneObservation, AiError> {
    let mut observation: SceneObservation = serde_json::from_value(json.clone())
        .map_err(|error| AiError::Invalid(format!("the scene description has the wrong shape: {error}")))?;
    for verdict in &mut observation.verdicts {
        // Some local models answer in percent despite the schema.
        if verdict.confidence > 1.0 && verdict.confidence <= 100.0 {
            verdict.confidence /= 100.0;
        }
        if !verdict.holds {
            verdict.detail.clear();
        }
    }
    Ok(observation.sanitized())
}

fn judge_text(conditions: &[&Condition], hints: &[String]) -> String {
    let mut text = String::new();
    if conditions.is_empty() {
        text.push_str("There are no conditions to judge this time: return an empty verdicts list and the caption.\n");
    } else {
        text.push_str("Judge each of these conditions in the attached camera frame.\n");
        for condition in conditions {
            let detail = if condition.detail.trim().is_empty() {
                "none (detail is \"\")".to_owned()
            } else {
                quote(&condition.detail)
            };
            let _ = write!(
                text,
                "\n- id: {}\n  name: {}\n  holds when: {}\n  detail to report while it holds: {detail}\n",
                quote(&condition.id),
                quote(&condition.name),
                quote(&condition.prompt),
            );
        }
    }
    let hints: Vec<String> = hints
        .iter()
        .map(|hint| one_line(hint, MAX_HINT_CHARS))
        .filter(|hint| !hint.is_empty())
        .take(MAX_HINTS)
        .collect();
    if !hints.is_empty() {
        text.push_str("\nLocal detector hints (they may be wrong):\n");
        for hint in hints {
            let _ = writeln!(text, "- {hint}");
        }
    }
    text
}

/// The answer's shape. Verdicts may only name the conditions asked about.
fn observation_schema(conditions: &[&Condition]) -> Value {
    let condition = if conditions.is_empty() {
        json!({"type": "string"})
    } else {
        let ids: Vec<&str> = conditions.iter().map(|condition| condition.id.as_str()).collect();
        json!({"type": "string", "enum": ids})
    };
    json!({
        "type": "object",
        "properties": {
            "caption": {
                "type": "string",
                "description": "One neutral sentence of at most 15 words describing the scene.",
            },
            "verdicts": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "condition": condition,
                        "reason": {
                            "type": "string",
                            "description": "One short sentence citing the visual evidence.",
                        },
                        "holds": {"type": "boolean"},
                        "confidence": {
                            "type": "number",
                            "description": "Probability from 0 to 1 that the holds value is correct.",
                        },
                        "detail": {
                            "type": "string",
                            "description": "The requested detail in at most 4 words when holds is true, else empty.",
                        },
                    },
                    "required": ["condition", "reason", "holds", "confidence", "detail"],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["caption", "verdicts"],
        "additionalProperties": false,
    })
}

/// A JSON string literal, so quotes and odd characters cannot blur where a value ends.
fn quote(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_default()
}

fn one_line(text: &str, max_chars: usize) -> String {
    let words: Vec<&str> = text
        .split(|c: char| c.is_whitespace() || c.is_control())
        .filter(|w| !w.is_empty())
        .collect();
    words.join(" ").chars().take(max_chars).collect()
}
