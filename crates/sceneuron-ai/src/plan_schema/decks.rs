// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The card deck parts of the edit plan schema: decks, their cards, what a card shows, the
//! camera signals and trigger phrases that may raise it, and what it says when spoken.

use serde_json::{Value, json};

use super::{integer, object, props, reference, string};

/// The serde names of the core's hand signs, as in `{"sign": "thumbs_up"}`.
const HAND_SIGNS: [&str; 6] = ["thumbs_up", "thumbs_down", "open_palm", "peace", "pointing", "fist"];

/// Every field is required: `update_deck` replaces the whole deck, so a delay or a camera,
/// voice, or speech switch the model leaves out would silently reset.
pub(super) fn deck() -> Value {
    object(
        json!({
            "id": string("Unique deck id: 1-40 of a-z, 0-9, - and _."),
            "name": string("Name shown in the editor and on controls, 1-80 characters."),
            "element": string("Id of the text element, the card slot, that shows the raised card; it shows no other deck."),
            "cards": {"type": "array", "description": "1 to 24 cards, in the order controls step through them.", "items": reference("card")},
            "auto_hide_ms": integer("Milliseconds a raised card stays up: 0 until it is lowered, else 1000 to 3600000."),
            "camera": {"type": "boolean", "description": "Whether the cards' camera signals may raise them."},
            "voice": {"type": "boolean", "description": "Whether spoken commands and trigger phrases may raise the cards."},
            "speak": {"type": "boolean", "description": "Whether a card is spoken aloud when it is raised."},
        }),
        &[
            "id",
            "name",
            "element",
            "cards",
            "auto_hide_ms",
            "camera",
            "voice",
            "speak",
        ],
    )
}

pub(super) fn card() -> Value {
    object(
        json!({
            "id": string("Card id, unique in its deck: 1-40 of a-z, 0-9, - and _."),
            "name": string("Name shown on controls, 1-80 characters."),
            "props": reference("card_props"),
            "signal": reference("signal"),
            "phrases": {
                "type": "array",
                "description": "0 to 4 trigger phrases that raise the card when said, each 2-60 characters of letters, digits, spaces, apostrophes, and basic punctuation.",
                "items": {"type": "string"},
            },
            "speech": string("What to say when the card is raised on a deck that speaks, 1-200 characters; leave it out to say the card's text."),
        }),
        &["id", "name", "props"],
    )
}

/// The element props without `visible`: raising and lowering a card decide visibility.
pub(super) fn card_props() -> Value {
    let mut schema = props();
    if let Some(properties) = schema["properties"].as_object_mut() {
        properties.remove("visible");
    }
    schema
}

/// `"hand_raised"`, `{"fingers": n}`, or `{"sign": name}`.
pub(super) fn signal() -> Value {
    let fingers = object(
        json!({"fingers": integer("Fingers held up, summed over both hands, 0 to 10.")}),
        &["fingers"],
    );
    let sign = object(json!({"sign": {"type": "string", "enum": HAND_SIGNS}}), &["sign"]);
    json!({"anyOf": [{"type": "string", "enum": ["hand_raised"]}, fingers, sign]})
}
