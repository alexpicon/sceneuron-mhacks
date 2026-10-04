// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the assistant's plans: a valid plan comes back applied with its cost, and an answer
//! that cannot be read as a plan or fails validation is sent back once with the reason.

mod common;

use common::assistant::{answering, hide_label, open, with_key};
use common::sink::wait_until;
use common::temp_dir;
use sceneuron_ai::{AiError, PlanMode};
use sceneuron_core::{EditOp, Project, default_project, pack_for_canvas};
use serde_json::{Value, json};

fn hide_ghost() -> Value {
    json!({"summary": "Hides the ghost.", "operations": [
        {"op": "update_element", "id": "ghost", "props": {"visible": false}}
    ]})
}

/// Renames the meeting deck, giving every deck field except `left_out`.
fn rename_meeting(project: &Project, left_out: Option<&str>) -> Value {
    let mut deck = serde_json::to_value(project.deck("meeting").unwrap()).unwrap();
    deck["name"] = json!("Team signals");
    if let Some(field) = left_out {
        deck.as_object_mut().unwrap().remove(field);
    }
    json!({"summary": "Renames the meeting deck.", "operations": [{"op": "update_deck", "deck": deck}]})
}

#[test]
fn a_valid_plan_comes_back_applied_with_its_cost() {
    let path = temp_dir("assistant-plan").join("ai.json");
    let mock = answering(vec![hide_label()]);
    let (assistant, sink) = open(&path, &mock);
    assistant.update_settings(with_key()).unwrap();
    let project = default_project();
    let outcome = assistant.plan(&project, "Hide my name", PlanMode::Edit, false).unwrap();
    assert_eq!(outcome.plan.summary, "Hides the name label.");
    assert!(!outcome.project.element("name-label").unwrap().props.visible);
    assert_eq!(outcome.model, "gemini-3.1-pro-preview");
    // 10k input tokens at $2 and 1k output tokens at $12 per million.
    assert!((outcome.cost_usd - 0.032).abs() < 1e-9, "{}", outcome.cost_usd);
    let requests = mock.requests();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].text.contains("<request>\nHide my name\n</request>") && requests[0].images.is_empty());
    assert!(wait_until(|| sink.last_status().is_some_and(|s| s.calls == 1)));
    assert!((sink.last_status().unwrap().hour_cost_usd - 0.032).abs() < 1e-9);
    assistant.shutdown();
}

#[test]
fn a_plan_that_fails_validation_is_retried_once_with_the_error() {
    let path = temp_dir("assistant-retry").join("ai.json");
    let mock = answering(vec![hide_ghost(), hide_label()]);
    let (assistant, sink) = open(&path, &mock);
    assistant.update_settings(with_key()).unwrap();
    let outcome = assistant
        .plan(&default_project(), "Hide the ghost", PlanMode::Edit, false)
        .unwrap();
    assert!(matches!(&outcome.plan.operations[0], EditOp::UpdateElement { id, .. } if id == "name-label"));
    assert!(
        (outcome.cost_usd - 0.064).abs() < 1e-9,
        "both calls count: {}",
        outcome.cost_usd
    );
    let requests = mock.requests();
    assert_eq!(requests.len(), 2);
    assert!(!requests[0].text.contains("failed validation"));
    let retry = &requests[1].text;
    assert!(
        retry.contains("Validation error: operation 1: unknown element \"ghost\""),
        "{retry}"
    );
    assert!(
        retry.contains("\"id\":\"ghost\"") && retry.contains("corrected, complete plan"),
        "{retry}"
    );
    assert!(wait_until(|| sink.last_status().is_some_and(|s| s.calls == 2)));
    assistant.shutdown();
}

#[test]
fn a_plan_that_fails_twice_is_invalid() {
    let path = temp_dir("assistant-twice").join("ai.json");
    let mock = answering(vec![hide_ghost()]);
    let (assistant, _) = open(&path, &mock);
    assistant.update_settings(with_key()).unwrap();
    let error = assistant
        .plan(&default_project(), "Hide the ghost", PlanMode::Edit, false)
        .unwrap_err();
    let AiError::Invalid(message) = &error else {
        panic!("expected an invalid plan, got {error:?}");
    };
    assert!(message.contains("unknown element \"ghost\""), "{message}");
    assert_eq!(mock.calls(), 2);
    assistant.shutdown();
}

#[test]
fn a_deck_answer_that_leaves_out_a_field_is_retried_with_the_reason() {
    let path = temp_dir("assistant-deck-field").join("ai.json");
    let base = default_project();
    let project = base
        .add_pack(&pack_for_canvas("meeting", &base.canvas).unwrap())
        .unwrap();
    let refused = rename_meeting(&project, Some("camera"));
    let mock = answering(vec![refused.clone(), rename_meeting(&project, None)]);
    let (assistant, _) = open(&path, &mock);
    assistant.update_settings(with_key()).unwrap();
    let outcome = assistant
        .plan(
            &project,
            "Rename the meeting deck to Team signals",
            PlanMode::Edit,
            false,
        )
        .unwrap();
    let deck = outcome.project.deck("meeting").unwrap();
    assert_eq!((deck.name.as_str(), deck.auto_hide_ms), ("Team signals", 15_000));
    let requests = mock.requests();
    assert_eq!(requests.len(), 2, "the refused answer is asked for again");
    let retry = &requests[1].text;
    let reason = "Validation error: operation 1: update_deck \"meeting\" must give camera; anything left out is reset";
    assert!(retry.contains(reason), "{retry}");
    assert!(
        retry.contains(&format!("Rejected plan:\n{refused}\n")),
        "the answer goes back as the model gave it: {retry}"
    );
    assistant.shutdown();
}

#[test]
fn an_answer_of_the_wrong_shape_twice_is_invalid() {
    let path = temp_dir("assistant-shape").join("ai.json");
    let mock = answering(vec![json!({"summary": "No operations."})]);
    let (assistant, _) = open(&path, &mock);
    assistant.update_settings(with_key()).unwrap();
    let error = assistant
        .plan(&default_project(), "Hide my name", PlanMode::Edit, false)
        .unwrap_err();
    let AiError::Invalid(message) = &error else {
        panic!("expected an invalid answer, got {error:?}");
    };
    assert!(message.starts_with("the plan has the wrong shape:"), "{message}");
    assert_eq!(mock.calls(), 2);
    let retry = &mock.requests()[1].text;
    assert!(
        retry.contains("Validation error: the plan has the wrong shape:"),
        "{retry}"
    );
    assistant.shutdown();
}
