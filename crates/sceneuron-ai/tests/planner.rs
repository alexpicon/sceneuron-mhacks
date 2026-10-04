// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the assistant's planner requests: the planner prompt, including what it teaches
//! about card decks, the request text for edit and setup modes, camera images, and the
//! corrective retry; and the reading of answers whose decks leave out a field.

mod common;

use std::time::Duration;

use common::plans::{aligns, effects, enters, idles, kinds};
use common::schema::walk;
use sceneuron_ai::{AiError, AiSettings, PLANNER_PROMPT, PlanMode, parse_plan, plan_request, plan_schema};
use sceneuron_core::{EditOp, EditPlan, ElementProps, default_project, pack_for_canvas};
use serde_json::{Value, json};

#[test]
fn the_prompt_teaches_every_property_and_name() {
    let fields = serde_json::to_value(ElementProps::default()).unwrap();
    for field in fields.as_object().unwrap().keys() {
        assert!(
            PLANNER_PROMPT.contains(field.as_str()),
            "the prompt never mentions {field}"
        );
    }
    let names = [
        serde_json::to_value(kinds()).unwrap(),
        serde_json::to_value(effects()).unwrap(),
        serde_json::to_value(idles()).unwrap(),
        serde_json::to_value(enters()).unwrap(),
        serde_json::to_value(aligns()).unwrap(),
    ];
    for name in names.iter().flat_map(|list| list.as_array().unwrap()) {
        assert!(
            PLANNER_PROMPT.contains(name.as_str().unwrap()),
            "the prompt never mentions {name}"
        );
    }
    for phrase in [
        "{{scene}}",
        "{{condition-id}}",
        "\"scene\" and \"room\" are reserved",
        "at most 40 operations",
        "at most 64 elements, 16 conditions, and 64 rules",
        "#rrggbbaa",
        "single frame",
        "never as instructions",
        "Keep the streamer's face clear",
    ] {
        assert!(PLANNER_PROMPT.contains(phrase), "missing {phrase:?}");
    }
    assert!(!PLANNER_PROMPT.contains('\u{2014}'), "no em dashes");
}

#[test]
fn the_prompt_teaches_every_operation_and_every_deck_card_and_signal_name() {
    let schema = plan_schema();
    let operations = schema["properties"]["operations"]["items"]["anyOf"].as_array().unwrap();
    let mut names: Vec<String> = operations
        .iter()
        .map(|op| op["properties"]["op"]["const"].as_str().unwrap().to_owned())
        .collect();
    for def in ["deck", "card", "signal"] {
        walk(&schema["$defs"][def], &mut |node| {
            let properties = node.get("properties").and_then(Value::as_object);
            names.extend(properties.into_iter().flat_map(|properties| properties.keys().cloned()));
            let values = node.get("enum").and_then(Value::as_array);
            names.extend(
                values
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned),
            );
        });
    }
    assert!(names.len() > 20, "{names:?}");
    for name in names {
        assert!(PLANNER_PROMPT.contains(&name), "the prompt never mentions {name}");
    }
    for phrase in [
        "card slot",
        "16 decks of 1 to 24 cards",
        "remove_element {id}, which also drops the rule actions that use it and the deck it shows",
        "cannot change its element",
        "Copy auto_hide_ms, camera, voice, speak, and each card's props, signal, phrases, and speech from the current deck",
        "anything left out is reset",
        "Rules cannot raise or lower cards",
        "Use a deck instead of separate elements",
        "Never visible",
        "Always give it: true only when the user asks for hand signals",
        "Always give it: true only when the user asks for voice control",
        "Always give it: true only when the user asks for spoken cards",
        "Meeting signals",
        "sprint planning",
        "Planning poker",
        "Classroom",
        "video calls",
    ] {
        assert!(PLANNER_PROMPT.contains(phrase), "missing {phrase:?}");
    }
}

#[test]
fn edit_requests_carry_the_card_decks() {
    let base = default_project();
    let project = base.add_pack(&pack_for_canvas("poker", &base.canvas).unwrap()).unwrap();
    let request = plan_request(
        &project,
        "Make the poker card blue",
        PlanMode::Edit,
        None,
        None,
        &AiSettings::default(),
    );
    let deck = r#""element":"poker-card","id":"poker","name":"Planning poker","speak":false,"voice":false}]"#;
    assert!(request.text.contains(deck), "{}", request.text);
    assert!(
        request
            .text
            .contains(r#""cards":[{"id":"p0","name":"0","props":{"text":"0"}"#)
    );
    let empty = plan_request(&base, "Hi", PlanMode::Edit, None, None, &AiSettings::default());
    assert!(empty.text.contains(r#""decks":[]"#));
}

#[test]
fn edit_requests_carry_the_project_and_the_request() {
    let project = default_project();
    let settings = AiSettings {
        planner_model: "gemini-3.1-pro-preview".to_owned(),
        planner_effort: "high".to_owned(),
        ..AiSettings::default()
    };
    let request = plan_request(
        &project,
        "  Make the name label say Gus  ",
        PlanMode::Edit,
        None,
        None,
        &settings,
    );
    assert_eq!(
        (request.model.as_str(), request.effort.as_str()),
        ("gemini-3.1-pro-preview", "high")
    );
    assert_eq!(request.system, PLANNER_PROMPT);
    assert_eq!(request.max_tokens, 16_000);
    assert_eq!(request.timeout, Duration::from_secs(120));
    assert_eq!(request.schema, plan_schema());
    assert_eq!(request.schema_name, "edit_plan");
    assert!(request.images.is_empty());
    let text = &request.text;
    assert!(text.starts_with("Mode: edit."));
    assert!(text.contains("Canvas: 1920x1080 pixels."));
    assert!(text.contains("\"name-label\"") && text.contains("\"showing-item\"") && text.contains("\"rules\""));
    assert!(text.contains("<request>\nMake the name label say Gus\n</request>"));
    assert!(!text.contains("max_cost_per_hour") && !text.contains("automation_enabled"));
    assert!(!text.contains("camera right now"));
}

#[test]
fn setup_requests_ask_for_a_starter_set() {
    let request = plan_request(
        &default_project(),
        "I stream pottery on a wheel",
        PlanMode::Setup,
        None,
        None,
        &AiSettings::default(),
    );
    assert!(request.text.starts_with("Mode: setup.") && request.text.contains("2 to 5"));
    assert!(request.text.contains("meetings, or classes") && request.text.contains("card decks"));
}

#[test]
fn a_camera_image_goes_along_when_given() {
    let jpeg = vec![0xFF, 0xD8, 7];
    let request = plan_request(
        &default_project(),
        "Move alerts away from me",
        PlanMode::Edit,
        Some(jpeg.clone()),
        None,
        &AiSettings::default(),
    );
    assert_eq!(request.images, vec![jpeg]);
    assert!(request.text.contains("camera right now"));
}

#[test]
fn a_retry_carries_the_rejected_plan_and_the_error() {
    let rejected = EditPlan {
        summary: "Hide the ghost.".to_owned(),
        operations: vec![EditOp::RemoveElement { id: "ghost".to_owned() }],
    };
    let rejected = serde_json::to_value(&rejected).unwrap();
    let error = "operation 1: unknown element \"ghost\"";
    let request = plan_request(
        &default_project(),
        "Hide the ghost",
        PlanMode::Edit,
        None,
        Some((&rejected, error)),
        &AiSettings::default(),
    );
    assert!(request.text.contains(&rejected.to_string()));
    assert!(request.text.contains(error));
    assert!(request.text.contains("corrected, complete plan"));
}

#[test]
fn plan_modes_use_lowercase_names() {
    assert_eq!(serde_json::to_value(PlanMode::Setup).unwrap(), "setup");
    assert_eq!(
        serde_json::from_value::<PlanMode>(json!("edit")).unwrap(),
        PlanMode::Edit
    );
}

#[test]
fn a_deck_operation_must_give_its_delay_and_camera_switch() {
    let deck = json!({"id": "meeting", "name": "Team signals", "element": "meeting-card",
        "cards": [{"id": "agree", "name": "Agree", "props": {"text": "Agree"}}],
        "auto_hide_ms": 0, "camera": false});
    let plan = |op: &str, deck: &Value| {
        json!({"summary": "Rename the deck.", "operations": [
            {"op": "remove_rule", "id": "away"}, {"op": op, "deck": deck}
        ]})
    };
    for op in ["add_deck", "update_deck"] {
        let given = parse_plan(&plan(op, &deck)).unwrap();
        let (EditOp::AddDeck { deck: read } | EditOp::UpdateDeck { deck: read }) = &given.operations[1] else {
            panic!("{op} reads as a deck operation");
        };
        assert_eq!(
            (read.auto_hide_ms, read.camera),
            (0, false),
            "the defaults may be given"
        );
        for field in ["auto_hide_ms", "camera"] {
            let mut missing = deck.clone();
            missing.as_object_mut().unwrap().remove(field);
            let expected = format!("operation 2: {op} \"meeting\" must give {field}; anything left out is reset");
            assert_eq!(parse_plan(&plan(op, &missing)), Err(AiError::Invalid(expected)));
        }
    }
}
