// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! AI settings as a Sceneuron that offered Anthropic saved them, written to temporary files, and
//! the warning shown when such a file moves to Gemini.

use std::fs;
use std::path::{Path, PathBuf};

use sceneuron_ai::AiSettings;
use serde_json::{Value, json};

use super::temp_dir;

/// An Anthropic-shaped key; never a real credential.
pub const ANTHROPIC_KEY: &str = "sk-ant-saved-in-the-editor-9876";

/// `settings` as an older Sceneuron saved them on Anthropic, with every field written out.
pub fn saved_on_anthropic(settings: &AiSettings) -> Value {
    let mut file = serde_json::to_value(settings).unwrap();
    for (field, value) in [
        ("provider", json!("anthropic")),
        ("base_url", json!("https://api.anthropic.com")),
        ("vision_model", json!("claude-opus-5-5")),
        ("planner_model", json!("claude-opus-5-5")),
        ("api_key", json!(ANTHROPIC_KEY)),
        ("input_price_per_mtok", json!(4.0)),
        ("output_price_per_mtok", json!(20.0)),
    ] {
        file[field] = value;
    }
    file["subtitles"]["translation_model"] = json!("claude-haiku-4-5-20251001");
    file
}

/// Writes `file` as `ai.json` in a new temporary folder named after `name`.
pub fn written(name: &str, file: &Value) -> PathBuf {
    let path = temp_dir(name).join("ai.json");
    fs::write(&path, file.to_string()).unwrap();
    path
}

/// The warning for the file at `path` moving off Anthropic, naming what may no longer be sent.
pub fn warning(path: &Path, consent: Option<&str>) -> String {
    let moved = format!(
        "Anthropic is no longer offered, so the AI settings in {} now use Gemini with its default models, \
         without the Anthropic key or price overrides",
        path.display()
    );
    match consent {
        Some(sent) => {
            format!("{moved}; consent to send {sent} was for Anthropic, so confirm it again for Google in the editor")
        }
        None => moved,
    }
}
