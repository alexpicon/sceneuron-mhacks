// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The JSON schema of an edit plan, mirroring the serde shape of `sceneuron_core::EditPlan`.
//!
//! The schema stays within what structured output supports: every object closes with
//! `additionalProperties: false`, there is no recursion, and value ranges live in descriptions
//! because numeric and length constraints are not supported. Validation of the applied plan
//! enforces the ranges.

mod decks;

use sceneuron_core::{ConditionEdge, Effect, ElementKind, EnterAnimation, IdleAnimation, PresenceTrigger, TextAlign};
use serde::Serialize;
use serde_json::{Map, Value, json};

/// Schema for `{summary, operations}`, with operations as one of the twelve `op` variants.
pub fn plan_schema() -> Value {
    let mut schema = object(
        json!({
            "summary": string("One to three plain sentences for the streamer about what the plan changes."),
            "operations": {
                "type": "array",
                "description": "At most 40 operations, applied in order.",
                "items": {"anyOf": operations()},
            },
        }),
        &["summary", "operations"],
    );
    schema["$defs"] = plan_defs();
    schema
}

/// The twelve `op` variants, whose references [`plan_defs`] resolves.
pub(crate) fn operations() -> Vec<Value> {
    vec![
        op("add_element", json!({"element": reference("element")}), &["element"]),
        op(
            "update_element",
            json!({"id": string("Id of the element to change."), "name": string("New name."), "props": reference("props")}),
            &["id"],
        ),
        op(
            "remove_element",
            json!({"id": string("Id of the element to remove.")}),
            &["id"],
        ),
        op(
            "add_condition",
            json!({"condition": reference("condition")}),
            &["condition"],
        ),
        op(
            "update_condition",
            json!({"condition": reference("condition")}),
            &["condition"],
        ),
        op(
            "remove_condition",
            json!({"id": string("Id of the condition to remove.")}),
            &["id"],
        ),
        op("add_rule", json!({"rule": reference("rule")}), &["rule"]),
        op("update_rule", json!({"rule": reference("rule")}), &["rule"]),
        op(
            "remove_rule",
            json!({"id": string("Id of the rule to remove.")}),
            &["id"],
        ),
        op("add_deck", json!({"deck": reference("deck")}), &["deck"]),
        op("update_deck", json!({"deck": reference("deck")}), &["deck"]),
        op(
            "remove_deck",
            json!({"id": string("Id of the deck to remove.")}),
            &["id"],
        ),
    ]
}

/// The definitions the operations refer to.
pub(crate) fn plan_defs() -> Value {
    json!({
        "props": props(),
        "element": element(),
        "condition": condition(),
        "trigger": trigger(),
        "action": action(),
        "rule": rule(),
        "deck": decks::deck(),
        "card": decks::card(),
        "card_props": decks::card_props(),
        "signal": decks::signal(),
    })
}

fn props() -> Value {
    object(
        json!({
            "x": number("Left edge in canvas pixels, -10000 to 10000."),
            "y": number("Top edge in canvas pixels, -10000 to 10000."),
            "width": number("Width in pixels, 1 to 10000."),
            "height": number("Height in pixels, 1 to 10000."),
            "visible": {"type": "boolean"},
            "text": string("Text of a text element, at most 1000 characters; may use {{scene}} and {{condition-id}}."),
            "font_size": number("Text size in pixels, 1 to 1000."),
            "bold": {"type": "boolean"},
            "color": color("Text color."),
            "background": color("Fill color of the panel."),
            "border_color": color("Border color."),
            "border_width": number("Border thickness in pixels, 0 to 200."),
            "corner_radius": number("Corner radius in pixels, 0 to 5000."),
            "opacity": number("Opacity of the whole element, 0 to 1."),
            "align": names(&[TextAlign::Left, TextAlign::Center, TextAlign::Right]),
            "idle": names(&[IdleAnimation::None, IdleAnimation::Pulse, IdleAnimation::Glow, IdleAnimation::Float]),
            "enter": names(&[EnterAnimation::None, EnterAnimation::Fade, EnterAnimation::SlideUp, EnterAnimation::Pop]),
        }),
        &[],
    )
}

fn element() -> Value {
    object(
        json!({
            "id": string("Unique element id: 1-40 of a-z, 0-9, - and _."),
            "name": string("Name shown in the editor, 1-80 characters."),
            "kind": names(&[ElementKind::Text, ElementKind::Box, ElementKind::Frame]),
            "props": reference("props"),
        }),
        &["id", "name", "kind", "props"],
    )
}

fn condition() -> Value {
    object(
        json!({
            "id": string("Unique condition id: 1-40 of a-z, 0-9, - and _; not \"scene\"."),
            "name": string("Name shown in the editor, 1-80 characters."),
            "prompt": string("What must be visible in one camera frame, 1-300 characters on one line."),
            "detail": string("What to report while it holds, at most 120 characters; empty for nothing."),
            "enabled": {"type": "boolean"},
            "min_confidence": number("Minimum model confidence, 0 to 1; default 0.6."),
            "confirm": integer("Frames that must hold before it turns on, 1 to 10; default 1."),
            "release": integer("Frames that must not hold before it turns off, 1 to 10; default 2."),
        }),
        &["id", "name", "prompt"],
    )
}

fn trigger() -> Value {
    let condition_edge = object(
        json!({
            "condition": string("Id of the condition."),
            "state": names(&[ConditionEdge::On, ConditionEdge::Off]),
        }),
        &["condition", "state"],
    );
    json!({"anyOf": [names(&[PresenceTrigger::Away, PresenceTrigger::Present]), condition_edge]})
}

fn action() -> Value {
    let set_props = object(
        json!({"type": constant("set_props"), "element": string("Id of the element."), "props": reference("props")}),
        &["type", "element", "props"],
    );
    let effect = object(
        json!({
            "type": constant("effect"),
            "element": string("Id of the element."),
            "effect": names(&[Effect::Shake, Effect::Pop, Effect::Flash, Effect::Bounce]),
        }),
        &["type", "element", "effect"],
    );
    json!({"anyOf": [set_props, effect]})
}

fn rule() -> Value {
    object(
        json!({
            "id": string("Unique rule id: 1-40 of a-z, 0-9, - and _."),
            "name": string("Name shown in the editor, 1-80 characters."),
            "enabled": {"type": "boolean"},
            "when": reference("trigger"),
            "actions": {"type": "array", "description": "1 to 16 actions.", "items": reference("action")},
        }),
        &["id", "name", "when", "actions"],
    )
}

fn op(name: &str, properties: Value, required: &[&str]) -> Value {
    let mut all = Map::new();
    all.insert("op".to_owned(), constant(name));
    if let Value::Object(properties) = properties {
        all.extend(properties);
    }
    let required: Vec<&str> = std::iter::once("op").chain(required.iter().copied()).collect();
    object(Value::Object(all), &required)
}

pub(crate) fn object(properties: Value, required: &[&str]) -> Value {
    let mut schema = json!({"type": "object", "properties": properties, "additionalProperties": false});
    if !required.is_empty() {
        schema["required"] = json!(required);
    }
    schema
}

fn reference(name: &str) -> Value {
    json!({"$ref": format!("#/$defs/{name}")})
}

pub(crate) fn constant(value: &str) -> Value {
    json!({"type": "string", "const": value})
}

pub(crate) fn string(description: &str) -> Value {
    json!({"type": "string", "description": description})
}

fn number(description: &str) -> Value {
    json!({"type": "number", "description": description})
}

fn integer(description: &str) -> Value {
    json!({"type": "integer", "description": description})
}

fn color(description: &str) -> Value {
    string(&format!("{description} Hex color: #rrggbb or #rrggbbaa."))
}

/// A string enum of the serde names of `variants`, so the schema cannot drift from the model.
pub(crate) fn names<T: Serialize>(variants: &[T]) -> Value {
    let names: Vec<Value> = variants
        .iter()
        .filter_map(|variant| serde_json::to_value(variant).ok())
        .collect();
    json!({"type": "string", "enum": names})
}
