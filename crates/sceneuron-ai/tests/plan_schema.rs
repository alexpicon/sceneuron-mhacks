// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the edit plan schema: the structured-output rules, no recursion, agreement with
//! the serde shape of every operation and enum value, card decks and their signals, optional
//! fields, and the reading of plans into `EditPlan`.

mod common;

use common::plans::{HAND_SIGNS, every_operation};
use common::schema::{recursive_defs, structured_output_problems, validate};
use sceneuron_ai::{AiError, parse_plan, plan_schema};
use sceneuron_core::{EditOp, EditPlan, default_project, pack_for_canvas, packs};
use serde_json::{Value, json};

#[test]
fn the_schema_suits_structured_output() {
    let schema = plan_schema();
    assert_eq!(structured_output_problems(&schema), Vec::<String>::new());
    assert_eq!(recursive_defs(&schema), Vec::<String>::new());
    let variants = schema["properties"]["operations"]["items"]["anyOf"].as_array().unwrap();
    assert_eq!(variants.len(), 12);
}

#[test]
fn every_operation_round_trips_through_parse_plan() {
    let plan = every_operation();
    let value = serde_json::to_value(&plan).unwrap();
    assert_eq!(parse_plan(&value).unwrap(), plan);
}

#[test]
fn serialized_operations_match_the_schema() {
    let schema = plan_schema();
    let value = serde_json::to_value(every_operation()).unwrap();
    validate(&value, &schema, &schema).unwrap();
    let mut misnamed = value.clone();
    misnamed["operations"][0]["op"] = json!("insert_element");
    assert!(validate(&misnamed, &schema, &schema).is_err());
    let mut extra = value;
    extra["operations"][0]["element"]["props"]["shadow"] = json!(true);
    assert!(validate(&extra, &schema, &schema).is_err());
}

#[test]
fn optional_fields_may_be_left_out() {
    let schema = plan_schema();
    let answer = json!({"summary": "Hide the label.", "operations": [
        {"op": "update_element", "id": "name-label", "props": {"visible": false}},
        {"op": "add_condition", "condition": {"id": "mug", "name": "Mug", "prompt": "A mug is visible."}},
        {"op": "add_rule", "rule": {"id": "r", "name": "R", "when": "away",
            "actions": [{"type": "effect", "element": "name-label", "effect": "pop"}]}}
    ]});
    validate(&answer, &schema, &schema).unwrap();
    let plan = parse_plan(&answer).unwrap();
    plan.apply(&default_project()).unwrap();
    let EditOp::AddCondition { condition } = &plan.operations[1] else {
        panic!("second operation adds a condition");
    };
    assert_eq!((condition.confirm, condition.release, condition.enabled), (1, 2, true));
}

#[test]
fn plans_of_the_wrong_shape_are_invalid_and_summaries_are_flattened() {
    for answer in [
        json!({"summary": "x"}),
        json!({"summary": "x", "operations": [{"op": "explode"}]}),
        json!({"summary": "x", "operations": [{"op": "update_element", "id": "a", "props": {"glow": 1}}]}),
    ] {
        assert!(matches!(parse_plan(&answer), Err(AiError::Invalid(_))), "{answer}");
    }
    let plan = parse_plan(&json!({"summary": "  Two\nlines\there. ", "operations": []})).unwrap();
    assert_eq!(plan.summary, "Two lines here.");
}

#[test]
fn every_pack_as_a_plan_matches_the_schema_and_applies() {
    let schema = plan_schema();
    let base = default_project();
    for pack in packs() {
        let pack = pack_for_canvas(&pack.id, &base.canvas).unwrap();
        let mut operations: Vec<EditOp> = pack
            .elements
            .iter()
            .cloned()
            .map(|element| EditOp::AddElement { element })
            .collect();
        operations.extend(pack.decks.iter().cloned().map(|deck| EditOp::AddDeck { deck }));
        let plan = EditPlan {
            summary: format!("Add {}.", pack.name),
            operations,
        };
        let value = serde_json::to_value(&plan).unwrap();
        validate(&value, &schema, &schema).unwrap_or_else(|error| panic!("{}: {error}", pack.id));
        assert_eq!(parse_plan(&value).unwrap(), plan);
        assert_eq!(plan.apply(&base).unwrap(), base.add_pack(&pack).unwrap());
    }
}

#[test]
fn signals_take_three_shapes_and_every_hand_sign() {
    let schema = plan_schema();
    let signal = |signal: Value| validate(&signal, &schema["$defs"]["signal"], &schema);
    for good in [json!("hand_raised"), json!({"fingers": 0}), json!({"fingers": 10})] {
        signal(good.clone()).unwrap_or_else(|error| panic!("{good}: {error}"));
    }
    for sign in HAND_SIGNS {
        signal(json!({"sign": sign})).unwrap();
    }
    let bad = [
        json!("thumbs_up"),
        json!({"fingers": "three"}),
        json!({"fingers": 2.5}),
        json!({"sign": "wave"}),
        json!({"fingers": 2, "sign": "peace"}),
    ];
    for bad in bad {
        assert!(signal(bad.clone()).is_err(), "{bad}");
    }
}

#[test]
fn decks_need_their_fields_and_cards_never_set_visible() {
    let schema = plan_schema();
    let card = json!({"id": "yes", "name": "Yes", "props": {"text": "Yes", "background": "#16a34aff"}});
    let deck = json!({"id": "votes", "name": "Votes", "element": "slot", "cards": [card],
        "auto_hide_ms": 15000, "camera": false, "voice": false, "speak": false});
    let plan = |op: &str, deck: &Value| json!({"summary": "x", "operations": [{"op": op, "deck": deck}]});
    validate(&plan("add_deck", &deck), &schema, &schema).unwrap();
    for field in [
        "id",
        "name",
        "element",
        "cards",
        "auto_hide_ms",
        "camera",
        "voice",
        "speak",
    ] {
        let mut missing = deck.clone();
        missing.as_object_mut().unwrap().remove(field);
        for op in ["add_deck", "update_deck"] {
            assert!(
                validate(&plan(op, &missing), &schema, &schema).is_err(),
                "{op} without {field}"
            );
        }
    }
    let mut shown = deck.clone();
    shown["cards"][0]["props"]["visible"] = json!(true);
    assert!(validate(&plan("add_deck", &shown), &schema, &schema).is_err());
    let mut bare = deck.clone();
    bare["cards"][0].as_object_mut().unwrap().remove("props");
    assert!(validate(&plan("add_deck", &bare), &schema, &schema).is_err());
    let element_props = schema["$defs"]["props"]["properties"].as_object().unwrap();
    let card_props = schema["$defs"]["card_props"]["properties"].as_object().unwrap();
    let mut expected: Vec<&String> = element_props.keys().filter(|key| *key != "visible").collect();
    expected.sort();
    let mut actual: Vec<&String> = card_props.keys().collect();
    actual.sort();
    assert_eq!(actual, expected, "cards may set every other prop");
}
