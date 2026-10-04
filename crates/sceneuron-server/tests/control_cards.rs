// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises card requests: raising, toggling, stepping, and lowering decks, and the 400 and
//! 404 answers for mistakes.

mod common;

use common::{control_post, deck, element, server_with_decks, view_after};
use serde_json::{Value, json};

#[test]
fn card_actions_raise_toggle_step_and_lower() {
    let server = server_with_decks();
    let act = |body: Value| view_after(&server, "/api/card", body);
    let raised = act(json!({"deck": "meeting", "card": "agree", "action": "raise"}));
    let meeting = deck(&raised, "meeting");
    assert_eq!(
        (&meeting["raised"], &meeting["raised_by"]),
        (&json!("agree"), &json!("manual"))
    );
    let now = server.hub.now_ms();
    let hide_at = meeting["hide_at_ms"].as_u64().unwrap();
    assert!(hide_at > now && hide_at <= now + 15_000, "{hide_at} vs {now}");
    assert_eq!(element(&raised, "meeting-card")["visible"], true);
    let lowered = act(json!({"deck": "meeting", "card": "agree", "action": "toggle"}));
    assert_eq!(deck(&lowered, "meeting")["raised"], Value::Null);
    assert_eq!(element(&lowered, "meeting-card")["visible"], false);
    let previous = act(json!({"deck": "poker", "action": "previous"}));
    assert_eq!(deck(&previous, "poker")["raised"], "break");
    assert_eq!(
        deck(&previous, "poker")["hide_at_ms"],
        Value::Null,
        "poker cards stay up"
    );
    let wrapped = act(json!({"deck": "poker", "card": null, "action": "next"}));
    assert_eq!(deck(&wrapped, "poker")["raised"], "p0");
    let toggled = act(json!({"deck": "poker", "card": "p8", "action": "toggle"}));
    assert_eq!(deck(&toggled, "poker")["raised"], "p8");
    let lowered = act(json!({"deck": "poker", "card": "ignored", "action": "lower"}));
    assert_eq!(deck(&lowered, "poker")["raised"], Value::Null);
    let again = act(json!({"deck": "poker", "action": "lower"}));
    assert_eq!(
        again["revision"], lowered["revision"],
        "lowering a lowered deck changes nothing"
    );
}

#[test]
fn card_mistakes_are_explained() {
    let server = server_with_decks();
    let cases = [
        (
            json!({"deck": "nope", "card": "p5", "action": "raise"}),
            404,
            "unknown deck \"nope\"",
        ),
        (
            json!({"deck": "poker", "card": "p4", "action": "raise"}),
            404,
            "deck \"poker\" has no card \"p4\"",
        ),
        (
            json!({"deck": "poker", "action": "toggle"}),
            400,
            "name the card to raise or toggle",
        ),
        (
            json!({"deck": "poker", "action": "shuffle"}),
            400,
            "invalid request body",
        ),
        (
            json!({"deck": "poker", "action": "next", "step": 2}),
            400,
            "invalid request body",
        ),
        (json!({"card": "p5", "action": "raise"}), 400, "invalid request body"),
    ];
    for (body, status, message) in cases {
        let reply = control_post(&server, "/api/card", body.clone());
        assert_eq!(reply.status, status, "{body}: {}", reply.text());
        assert!(reply.error().starts_with(message), "{body}: {}", reply.error());
    }
    assert!(server.hub.snapshot().decks.iter().all(|deck| deck.raised.is_none()));
}
