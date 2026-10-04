// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises camera signal values: their JSON shapes on cards, and which frame summaries each
//! signal matches.

use sceneuron_core::{Card, CardSignal, HandSign, SignalSummary};
use serde_json::json;

const SIGNS: [(HandSign, &str); 6] = [
    (HandSign::ThumbsUp, "thumbs_up"),
    (HandSign::ThumbsDown, "thumbs_down"),
    (HandSign::OpenPalm, "open_palm"),
    (HandSign::Peace, "peace"),
    (HandSign::Pointing, "pointing"),
    (HandSign::Fist, "fist"),
];

#[test]
fn signals_use_the_documented_json_shapes() {
    let shapes = [
        (CardSignal::HandRaised, json!("hand_raised")),
        (CardSignal::Fingers(3), json!({"fingers": 3})),
        (CardSignal::Sign(HandSign::ThumbsUp), json!({"sign": "thumbs_up"})),
    ];
    for (signal, shape) in shapes {
        assert_eq!(serde_json::to_value(signal).unwrap(), shape);
        assert_eq!(serde_json::from_value::<CardSignal>(shape).unwrap(), signal);
    }
    for (sign, word) in SIGNS {
        assert_eq!(serde_json::to_value(sign).unwrap(), word);
        let signal = serde_json::from_value::<CardSignal>(json!({"sign": word})).unwrap();
        assert_eq!(signal, CardSignal::Sign(sign));
    }
    for wrong in [
        json!("wave"),
        json!({"sign": "rock"}),
        json!({"fingers": -1}),
        json!({"fingers": 256}),
    ] {
        assert!(serde_json::from_value::<CardSignal>(wrong.clone()).is_err(), "{wrong}");
    }
}

#[test]
fn cards_carry_a_signal_only_when_they_have_one() {
    let card: Card = serde_json::from_value(json!({
        "id": "agree", "name": "Agree", "signal": {"sign": "thumbs_up"}
    }))
    .unwrap();
    assert_eq!(card.signal, Some(CardSignal::Sign(HandSign::ThumbsUp)));
    let json = serde_json::to_value(&card).unwrap();
    assert_eq!(json["signal"], json!({"sign": "thumbs_up"}));
    let plain = Card { signal: None, ..card };
    assert!(serde_json::to_value(&plain).unwrap().get("signal").is_none());
}

fn summary(edit: impl FnOnce(&mut SignalSummary)) -> SignalSummary {
    let mut summary = SignalSummary::default();
    edit(&mut summary);
    summary
}

#[test]
fn a_raised_hand_matches_any_raised_hand_or_arm() {
    let raised = CardSignal::HandRaised;
    assert!(!raised.matches(&SignalSummary::default()));
    assert!(raised.matches(&summary(|s| s.hands_raised = 1)));
    assert!(raised.matches(&summary(|s| s.hands_raised = 2)));
    assert!(raised.matches(&summary(|s| s.arms_up = true)));
    assert!(!raised.matches(&summary(|s| s.signs = vec![HandSign::OpenPalm])));
}

#[test]
fn fingers_match_the_exact_known_count() {
    for count in 0..=10 {
        let shown = summary(|s| s.fingers = Some(count));
        let matching: Vec<u8> = (0..=10).filter(|n| CardSignal::Fingers(*n).matches(&shown)).collect();
        assert_eq!(matching, [count]);
    }
    assert!(
        !CardSignal::Fingers(0).matches(&SignalSummary::default()),
        "an unknown count is not zero"
    );
}

#[test]
fn a_sign_matches_when_any_hand_shows_it() {
    let both = summary(|s| s.signs = vec![HandSign::Peace, HandSign::ThumbsDown]);
    for (sign, _) in SIGNS {
        let shown = matches!(sign, HandSign::Peace | HandSign::ThumbsDown);
        assert_eq!(CardSignal::Sign(sign).matches(&both), shown, "{sign:?}");
    }
}
