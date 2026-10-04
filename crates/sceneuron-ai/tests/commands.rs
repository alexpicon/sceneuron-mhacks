// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises voice commands for the AI: the answer schema suits structured output and matches
//! the serde shape of every action kind and enum value, the request carries the words, the
//! reactions, the themes, and the project to the planner model with little effort and the
//! planner's instructions, a retry carries the refused answer, and answers are read with the
//! plan's checks for project edits.

mod common;

use common::assistant::with_key;
use common::plans::every_operation;
use common::schema::{recursive_defs, structured_output_problems, validate};
use sceneuron_ai::{
    AiError, AiSettings, MAX_ACTIONS, PLANNER_PROMPT, VOICE_PROMPT, check_edits, command_request, command_schema,
    parse_commands,
};
use sceneuron_core::{EditOp, Flood, Theme, VoiceAction, default_project, presets};
use serde_json::{Value, json};

/// One action of every kind, every reaction and theme among them.
fn every_action() -> Vec<VoiceAction> {
    let mut actions: Vec<VoiceAction> = presets()
        .iter()
        .map(|preset| VoiceAction::React { reaction: preset.flood })
        .collect();
    actions.extend(Theme::ALL.iter().map(|theme| VoiceAction::Theme { theme: *theme }));
    let id = |text: &str| text.to_owned();
    actions.extend([
        VoiceAction::ShowElement {
            element: id("brb"),
            visible: true,
        },
        VoiceAction::SetText {
            element: id("name-label"),
            text: id("Back in five"),
        },
        VoiceAction::ReleaseElement { element: id("brb") },
        VoiceAction::RaiseCard {
            deck: id("meeting"),
            card: id("agree"),
        },
        VoiceAction::LowerCards { deck: String::new() },
        VoiceAction::Automation { enabled: false },
        VoiceAction::Room {
            step: sceneuron_core::RoomStep::Reveal,
        },
        VoiceAction::Room {
            step: sceneuron_core::RoomStep::Next,
        },
        VoiceAction::Subtitles { enabled: true },
        VoiceAction::SubtitleLanguage { language: id("pt-BR") },
        VoiceAction::Look,
    ]);
    let plan = every_operation();
    actions.push(VoiceAction::EditProject {
        summary: plan.summary,
        operations: plan.operations,
    });
    actions
}

fn answer(actions: &[VoiceAction]) -> Value {
    json!({"understood": "make it festive", "actions": actions})
}

#[test]
fn the_schema_suits_structured_output_and_offers_every_action_kind() {
    let schema = command_schema();
    assert_eq!(structured_output_problems(&schema), Vec::<String>::new());
    assert_eq!(recursive_defs(&schema), Vec::<String>::new());
    let kinds: Vec<&str> = schema["properties"]["actions"]["items"]["anyOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|kind| kind["properties"]["action"]["const"].as_str().unwrap())
        .collect();
    assert_eq!(
        kinds,
        [
            "react",
            "theme",
            "show_element",
            "set_text",
            "release_element",
            "raise_card",
            "lower_cards",
            "automation",
            "subtitles",
            "subtitle_language",
            "look",
            "room",
            "edit_project"
        ]
    );
    let served: Vec<String> = every_action()
        .iter()
        .map(|action| {
            serde_json::to_value(action).unwrap()["action"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert!(kinds.iter().all(|kind| served.iter().any(|known| known == kind)));
}

#[test]
fn every_action_matches_the_schema_and_round_trips() {
    let schema = command_schema();
    let actions = every_action();
    let value = answer(&actions);
    validate(&value, &schema, &schema).unwrap();
    for some in actions.chunks(MAX_ACTIONS) {
        let read = parse_commands(&answer(some)).unwrap();
        assert_eq!(
            (read.understood.as_str(), read.actions.as_slice()),
            ("make it festive", some)
        );
    }
    let mut misnamed = value.clone();
    misnamed["actions"][0]["reaction"] = json!("confetti");
    assert!(validate(&misnamed, &schema, &schema).is_err());
    let mut extra = value;
    extra["actions"][0]["volume"] = json!(11);
    assert!(validate(&extra, &schema, &schema).is_err());
}

#[test]
fn the_request_carries_the_words_and_the_project_to_the_planner_model() {
    let settings = AiSettings {
        planner_model: "gemini-3.1-pro-preview".to_owned(),
        planner_effort: "high".to_owned(),
        ..AiSettings::default()
    };
    let mut project = default_project();
    project.automation_enabled = false;
    let request = command_request(&project, "  Sceneuron, make it festive ", None, &settings);
    assert_eq!(
        (
            request.model.as_str(),
            request.effort.as_str(),
            request.schema_name.as_str()
        ),
        ("gemini-3.1-pro-preview", "low", "voice_command")
    );
    assert_eq!(request.system, format!("{PLANNER_PROMPT}\n\n{VOICE_PROMPT}"));
    assert!(request.images.is_empty(), "no camera image goes along");
    assert_eq!(request.schema, command_schema());
    let text = &request.text;
    assert!(text.starts_with("The streamer said, to Sceneuron:\n<words>\nSceneuron, make it festive\n</words>"));
    for part in [
        r#"{"banner":"Mind blown","id":"mind-blown","name":"Mind blown"}"#,
        "Themes: none, graphite, hacker, eighties, windows-95, classic-mac, vaporwave; now none.",
        "Automation is paused.",
        "Canvas: 1920x1080 pixels.",
        r#""id":"name-label""#,
    ] {
        assert!(text.contains(part), "{part} in {text}");
    }
    let rejected = json!({"understood": "x", "actions": []});
    let retry = command_request(
        &project,
        "make it festive",
        Some((&rejected, "unknown element \"x\"")),
        &settings,
    );
    assert!(
        retry
            .text
            .contains("Rejected answer:\n{\"actions\":[],\"understood\":\"x\"}")
    );
    assert!(retry.text.contains("Reason: unknown element \"x\""));
    assert!(with_key().planner_model.is_some());
}

#[test]
fn answers_are_read_with_the_plans_checks() {
    let spaced =
        json!({"understood": "  play   the\nship it ", "actions": [{"action": "react", "reaction": "ship-it"}]});
    let read = parse_commands(&spaced).unwrap();
    assert_eq!(read.understood, "play the ship it");
    assert_eq!(
        read.actions,
        [VoiceAction::React {
            reaction: Flood::ShipIt
        }]
    );
    let message = |value: Value| match parse_commands(&value) {
        Err(AiError::Invalid(message)) => message,
        other => panic!("expected a refusal, got {other:?}"),
    };
    assert_eq!(message(json!({"actions": []})), "the answer has no understood text");
    assert_eq!(message(json!({"understood": "x"})), "the answer has no list of actions");
    let many: Vec<Value> = (0..=MAX_ACTIONS).map(|_| json!({"action": "look"})).collect();
    assert_eq!(
        message(json!({"understood": "x", "actions": many})),
        "an answer may have at most 10 actions"
    );
    assert!(message(json!({"understood": "x", "actions": [{"action": "dance"}]})).starts_with("action 1: "));
    let deck = json!({"id": "d", "name": "D", "element": "brb", "cards": [{"id": "c", "name": "C"}]});
    let partial = json!({"understood": "x", "actions": [
        {"action": "look"},
        {"action": "edit_project", "summary": "Adds a deck.", "operations": [{"op": "add_deck", "deck": deck}]}
    ]});
    assert_eq!(
        message(partial),
        "action 2: operation 1: add_deck \"d\" must give auto_hide_ms; anything left out is reset"
    );
}

#[test]
fn project_edits_must_apply_in_turn() {
    let project = default_project();
    let edit = |operations: Vec<EditOp>| VoiceAction::EditProject {
        summary: "Edits.".to_owned(),
        operations,
    };
    let remove = || EditOp::RemoveElement { id: "brb".to_owned() };
    let mut answer = parse_commands(&json!({"understood": "x", "actions": []})).unwrap();
    answer.actions = vec![edit(vec![remove()])];
    assert_eq!(check_edits(&answer, &project), Ok(()));
    answer.actions.push(edit(vec![remove()]));
    assert_eq!(
        check_edits(&answer, &project),
        Err("operation 1: unknown element \"brb\"".to_owned()),
        "the second edit applies after the first"
    );
}
