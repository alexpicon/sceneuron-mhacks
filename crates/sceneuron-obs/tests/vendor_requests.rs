// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises vendor request checking: each request's data becomes one typed request, unknown
//! fields and values of the wrong type are refused, and card requests raise, lower, and step
//! decks as one engine change each.

use sceneuron::vendor_requests::{Change, REQUESTS, Request, parse};
use sceneuron_core::{CardAction, Effect, Engine, EngineError, Outcome, Presence, default_project, pack_for_canvas};
use serde_json::Value;

fn engine_with(packs: &[&str]) -> Engine {
    let mut project = default_project();
    for id in packs {
        let pack = pack_for_canvas(id, &project.canvas).unwrap();
        project = project.add_pack(&pack).unwrap();
    }
    Engine::new(project, 0).unwrap()
}

fn change(name: &str, data: &str) -> Change {
    match parse(name, data) {
        Ok(Request::Change(change)) => change,
        other => panic!("{name} {data}: {other:?}"),
    }
}

fn refusal(name: &str, data: &str) -> String {
    parse(name, data).expect_err(&format!("{name} {data} should be refused"))
}

fn card(deck: &str, card: Option<&str>, action: CardAction) -> Change {
    Change::Card {
        deck: deck.to_owned(),
        card: card.map(str::to_owned),
        action,
    }
}

/// Data each registered request accepts.
const VALID: [(&str, &str); 11] = [
    ("GetState", "{}"),
    ("SetOverride", r#"{"element":"brb","props":{"visible":true}}"#),
    ("ReleaseOverrides", r#"{"element":"brb"}"#),
    ("TriggerEffect", r#"{"element":"brb","effect":"pop"}"#),
    ("SetAutomation", r#"{"enabled":false}"#),
    ("SimulatePresence", r#"{"presence":"away"}"#),
    ("AnalyzeNow", "{}"),
    ("RaiseCard", r#"{"deck":"poker","card":"p5"}"#),
    ("ToggleCard", r#"{"deck":"poker","card":"p5"}"#),
    ("LowerCard", r#"{"deck":"poker"}"#),
    ("StepCard", r#"{"deck":"poker","step":1}"#),
];

// Checking request data

#[test]
fn every_registered_request_is_understood() {
    let registered: Vec<_> = REQUESTS.iter().map(|name| name.to_str().unwrap()).collect();
    let covered: Vec<_> = VALID.iter().map(|(name, _)| *name).collect();
    assert_eq!(registered, covered);
    for (name, data) in VALID {
        assert!(parse(name, data).is_ok(), "{name} {data}");
    }
    assert_eq!(refusal("Explode", "{}"), "unknown request Explode");
}

#[test]
fn every_request_refuses_unknown_fields() {
    for (name, data) in VALID {
        let mut extra: Value = serde_json::from_str(data).unwrap();
        extra["colour"] = Value::from("red");
        let error = refusal(name, &extra.to_string());
        assert!(error.contains("unknown field `colour`"), "{name}: {error}");
    }
    let nested = r#"{"element":"brb","props":{"visible":true,"colour":"red"}}"#;
    assert!(refusal("SetOverride", nested).contains("unknown field `colour`"));
}

#[test]
fn empty_data_counts_as_none() {
    assert_eq!(parse("GetState", ""), Ok(Request::GetState));
    assert_eq!(parse("AnalyzeNow", " "), Ok(Request::AnalyzeNow));
    assert!(refusal("SetAutomation", "").contains("missing field `enabled`"));
}

#[test]
fn release_overrides_refuses_an_element_that_is_not_text() {
    for data in [
        r#"{"element":5}"#,
        r#"{"element":true}"#,
        r#"{"element":{}}"#,
        r#"{"element":["brb"]}"#,
    ] {
        assert!(refusal("ReleaseOverrides", data).contains("invalid type"), "{data}");
    }
    let one = Change::ReleaseOverrides {
        element: Some("brb".to_owned()),
    };
    assert_eq!(change("ReleaseOverrides", r#"{"element":"brb"}"#), one);
    for every in ["{}", r#"{"element":null}"#] {
        assert_eq!(
            change("ReleaseOverrides", every),
            Change::ReleaseOverrides { element: None }
        );
    }
}

#[test]
fn element_and_automation_requests_keep_their_meaning() {
    let effect = Change::TriggerEffect {
        element: "brb".to_owned(),
        effect: Effect::Pop,
    };
    assert_eq!(change("TriggerEffect", r#"{"element":"brb","effect":"pop"}"#), effect);
    assert_eq!(
        change("SetAutomation", r#"{"enabled":true}"#),
        Change::SetAutomation { enabled: true }
    );
    let away = Change::SimulatePresence {
        presence: Presence::Away,
    };
    assert_eq!(change("SimulatePresence", r#"{"presence":"away"}"#), away);
    assert!(refusal("SetAutomation", r#"{"enabled":"yes"}"#).contains("invalid type"));
    assert!(refusal("SimulatePresence", r#"{"presence":"here"}"#).contains("unknown variant"));
    assert!(refusal("TriggerEffect", r#"{"element":"brb","effect":"spin"}"#).contains("unknown variant"));
}

#[test]
fn raise_and_toggle_name_a_card() {
    let data = r#"{"deck":"poker","card":"p5"}"#;
    assert_eq!(change("RaiseCard", data), card("poker", Some("p5"), CardAction::Raise));
    assert_eq!(
        change("ToggleCard", data),
        card("poker", Some("p5"), CardAction::Toggle)
    );
    assert!(refusal("RaiseCard", r#"{"deck":"poker"}"#).contains("missing field `card`"));
    assert!(refusal("ToggleCard", r#"{"card":"p5"}"#).contains("missing field `deck`"));
    assert!(refusal("RaiseCard", r#"{"deck":"poker","card":5}"#).contains("invalid type"));
}

#[test]
fn lower_card_lowers_one_deck_or_every_deck() {
    assert_eq!(
        change("LowerCard", r#"{"deck":"poker"}"#),
        card("poker", None, CardAction::Lower)
    );
    for every in ["", "{}", r#"{"deck":null}"#] {
        assert_eq!(change("LowerCard", every), Change::LowerAllCards, "{every:?}");
    }
    assert!(refusal("LowerCard", r#"{"deck":7}"#).contains("invalid type"));
}

#[test]
fn step_card_moves_one_card_forward_or_back() {
    assert_eq!(
        change("StepCard", r#"{"deck":"poker","step":1}"#),
        card("poker", None, CardAction::Next)
    );
    assert_eq!(
        change("StepCard", r#"{"deck":"poker","step":-1}"#),
        card("poker", None, CardAction::Previous)
    );
    assert!(refusal("StepCard", r#"{"deck":"poker","step":2}"#).contains("step must be 1 or -1, not 2"));
    for step in ["0", "-2", "1.5", r#""1""#, "true", "null"] {
        assert!(
            parse("StepCard", &format!(r#"{{"deck":"poker","step":{step}}}"#)).is_err(),
            "{step}"
        );
    }
    assert!(refusal("StepCard", r#"{"deck":"poker"}"#).contains("missing field `step`"));
}

// Applying card requests

/// Checks a request and applies it to `engine`.
fn send(engine: &mut Engine, name: &str, data: &str) -> Result<Outcome, EngineError> {
    change(name, data).apply(engine, 10)
}

/// An engine with a poker card and a meeting card raised.
fn two_raised() -> Engine {
    let mut engine = engine_with(&["poker", "meeting"]);
    send(&mut engine, "RaiseCard", r#"{"deck":"poker","card":"p3"}"#).unwrap();
    send(&mut engine, "RaiseCard", r#"{"deck":"meeting","card":"agree"}"#).unwrap();
    engine
}

fn raised(engine: &Engine) -> (Option<&str>, Option<&str>) {
    (engine.raised_card("poker"), engine.raised_card("meeting"))
}

#[test]
fn raise_card_raises_in_one_change() {
    let mut engine = engine_with(&["poker"]);
    let revision = engine.revision();
    let outcome = send(&mut engine, "RaiseCard", r#"{"deck":"poker","card":"p5"}"#).unwrap();
    assert!(outcome.changed && !outcome.project_dirty);
    assert_eq!(engine.revision(), revision + 1);
    assert_eq!(engine.raised_card("poker"), Some("p5"));
    let props = engine.effective_props("poker-card").unwrap();
    assert!(props.visible && props.text == "5");
}

#[test]
fn toggle_card_raises_another_card_or_lowers_the_raised_one() {
    let mut engine = engine_with(&["poker"]);
    send(&mut engine, "ToggleCard", r#"{"deck":"poker","card":"p5"}"#).unwrap();
    send(&mut engine, "ToggleCard", r#"{"deck":"poker","card":"p8"}"#).unwrap();
    assert_eq!(engine.raised_card("poker"), Some("p8"));
    send(&mut engine, "ToggleCard", r#"{"deck":"poker","card":"p8"}"#).unwrap();
    assert_eq!(engine.raised_card("poker"), None);
}

#[test]
fn lower_card_without_a_deck_lowers_every_deck_in_one_change() {
    let mut engine = two_raised();
    let revision = engine.revision();
    assert!(send(&mut engine, "LowerCard", "{}").unwrap().changed);
    assert_eq!(engine.revision(), revision + 1);
    assert_eq!(raised(&engine), (None, None));
    assert!(!send(&mut engine, "LowerCard", "{}").unwrap().changed);
}

#[test]
fn lower_card_with_a_deck_lowers_only_that_deck() {
    let mut engine = two_raised();
    send(&mut engine, "LowerCard", r#"{"deck":"meeting"}"#).unwrap();
    assert_eq!(raised(&engine), (Some("p3"), None));
}

#[test]
fn step_card_wraps_around_the_deck() {
    let mut engine = engine_with(&["poker"]);
    send(&mut engine, "StepCard", r#"{"deck":"poker","step":-1}"#).unwrap();
    assert_eq!(engine.raised_card("poker"), Some("break"));
    send(&mut engine, "StepCard", r#"{"deck":"poker","step":1}"#).unwrap();
    assert_eq!(engine.raised_card("poker"), Some("p0"));
}

#[test]
fn card_requests_for_unknown_decks_or_cards_change_nothing() {
    let mut engine = engine_with(&["poker"]);
    let revision = engine.revision();
    let votes = Err(EngineError::UnknownDeck("votes".to_owned()));
    assert_eq!(send(&mut engine, "RaiseCard", r#"{"deck":"votes","card":"p5"}"#), votes);
    assert_eq!(send(&mut engine, "StepCard", r#"{"deck":"votes","step":1}"#), votes);
    let unknown_card = send(&mut engine, "ToggleCard", r#"{"deck":"poker","card":"p4"}"#);
    assert!(matches!(unknown_card, Err(EngineError::UnknownCard { .. })));
    assert_eq!(engine.revision(), revision);
}
