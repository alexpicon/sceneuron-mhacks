// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The JSON schema of a voice command's answer, mirroring the serde shape of
//! `sceneuron_core::VoiceAction`: what was understood, and the actions, one of twelve kinds tagged
//! by `action`, whose project edits take the edit plan's operations. It keeps to what structured
//! output supports, like the plan schema.

use sceneuron_core::{Theme, presets};
use serde_json::{Map, Value, json};

use super::MAX_ACTIONS;
use crate::plan_schema::{constant, names, object, operations, plan_defs, string};

/// Schema for `{understood, actions}`.
pub fn command_schema() -> Value {
    let floods: Vec<_> = presets().iter().map(|preset| preset.flood).collect();
    let id = |what: &str| string(&format!("Id of the {what}, from the project."));
    let flag = |what: &str| json!({"type": "boolean", "description": what});
    let language = string("BCP 47 code of the language, such as es, pt-BR, zh-Hans, tlh, or art-x-dothraki.");
    let actions = vec![
        action("react", json!({"reaction": names(&floods)})),
        action("theme", json!({"theme": names(&Theme::ALL)})),
        action(
            "show_element",
            json!({"element": id("element"), "visible": flag("true shows it, false hides it.")}),
        ),
        action(
            "set_text",
            json!({"element": id("text element"), "text": string("The text to show, at most 1000 characters.")}),
        ),
        action("release_element", json!({"element": id("element")})),
        action(
            "raise_card",
            json!({"deck": id("deck"), "card": id("card in that deck")}),
        ),
        action(
            "lower_cards",
            json!({"deck": string("Id of the deck, or \"\" for every deck whose voice is on.")}),
        ),
        action(
            "automation",
            json!({"enabled": flag("true resumes automation, false pauses it.")}),
        ),
        action(
            "subtitles",
            json!({"enabled": flag("true turns live subtitles on, false off.")}),
        ),
        action("subtitle_language", json!({ "language": language })),
        action("look", json!({})),
        action(
            "room",
            json!({"step": {"type": "string", "enum": ["reveal", "next"], "description": "reveal shows everyone's votes; next starts the next round."}}),
        ),
        action(
            "edit_project",
            json!({
                "summary": string("One short plain sentence about what the change does."),
                "operations": {
                    "type": "array",
                    "description": "At most 40 operations, applied in order.",
                    "items": {"anyOf": operations()},
                },
            }),
        ),
    ];
    let mut schema = object(
        json!({
            "understood": string(
                "A few plain words for what the streamer asked for, or, with no actions, why nothing fits.",
            ),
            "actions": {
                "type": "array",
                "description": format!("At most {MAX_ACTIONS} actions, carried out in order."),
                "items": {"anyOf": actions},
            },
        }),
        &["understood", "actions"],
    );
    schema["$defs"] = plan_defs();
    schema
}

/// One action kind: its `action` tag and the fields it needs, all required.
fn action(name: &str, fields: Value) -> Value {
    let mut all = Map::new();
    all.insert("action".to_owned(), constant(name));
    let mut required = vec!["action".to_owned()];
    if let Value::Object(fields) = fields {
        required.extend(fields.keys().cloned());
        all.extend(fields);
    }
    let required: Vec<&str> = required.iter().map(String::as_str).collect();
    object(Value::Object(all), &required)
}
