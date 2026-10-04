// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises AI edit plans: every operation, cascading removals, refusals, validation of the
//! result, and the JSON shape the planner model produces.

mod common;

use sceneuron_core::{
    Action, ConditionEdge, EditOp, EditPlan, ElementKind, MAX_PLAN_OPERATIONS, Project, PropsPatch, Rule, Trigger,
    default_project,
};

fn plan(operations: Vec<EditOp>) -> EditPlan {
    EditPlan {
        summary: "test".to_owned(),
        operations,
    }
}

fn from_json(json: &str) -> EditPlan {
    serde_json::from_str(json).unwrap()
}

#[test]
fn planner_json_adds_a_condition_element_and_rule() {
    let plan = from_json(
        r##"{
        "summary": "Celebrate when I raise both arms",
        "operations": [
            {"op": "add_condition", "condition": {"id": "arms-up", "name": "Arms up",
                "prompt": "The person raises both arms above their head."}},
            {"op": "add_element", "element": {"id": "hype", "name": "Hype", "kind": "text",
                "props": {"text": "LET'S GO!", "visible": false}}},
            {"op": "add_rule", "rule": {"id": "hype-on", "name": "Hype on arms up",
                "when": {"condition": "arms-up", "state": "on"},
                "actions": [{"type": "set_props", "element": "hype", "props": {"visible": true}},
                            {"type": "effect", "element": "hype", "effect": "shake"}]}}
        ]}"##,
    );
    let next = plan.apply(&default_project()).unwrap();
    let condition = next.condition("arms-up").unwrap();
    assert_eq!((condition.confirm, condition.release, condition.enabled), (1, 2, true));
    assert_eq!(next.element("hype").unwrap().kind, ElementKind::Text);
    let rule = next.rules.iter().find(|r| r.id == "hype-on").unwrap();
    assert_eq!(rule.when, Trigger::condition("arms-up", ConditionEdge::On));
}

#[test]
fn update_element_patches_only_named_fields() {
    let base = default_project();
    let props = PropsPatch {
        text: Some("Alexander".to_owned()),
        ..PropsPatch::default()
    };
    let next = plan(vec![EditOp::UpdateElement {
        id: "name-label".to_owned(),
        name: Some("Streamer name".to_owned()),
        props,
    }])
    .apply(&base)
    .unwrap();
    let label = next.element("name-label").unwrap();
    assert_eq!(
        (label.name.as_str(), label.props.text.as_str()),
        ("Streamer name", "Alexander")
    );
    assert_eq!(label.props.x, base.element("name-label").unwrap().props.x);
}

#[test]
fn removing_an_element_drops_its_actions_and_emptied_rules() {
    let next = plan(vec![EditOp::RemoveElement { id: "brb".to_owned() }])
        .apply(&default_project())
        .unwrap();
    assert!(next.rules.iter().all(|r| r.id != "away-show-brb"));
    let present = next.rules.iter().find(|r| r.id == "present-hide-brb").unwrap();
    assert!(present.actions.iter().all(|a| a.element() != Some("brb")));
}

#[test]
fn removing_an_element_drops_the_deck_it_shows() {
    let base = common::deck_project();
    let next = plan(vec![EditOp::RemoveElement { id: "slot".to_owned() }])
        .apply(&base)
        .unwrap();
    assert!(next.decks.is_empty() && next.element("slot").is_none());
    let kept = plan(vec![EditOp::RemoveElement { id: "brb".to_owned() }])
        .apply(&base)
        .unwrap();
    assert_eq!(kept.decks, base.decks);
}

#[test]
fn removing_a_condition_drops_the_rules_it_triggers() {
    let next = plan(vec![EditOp::RemoveCondition {
        id: "pet-in-view".to_owned(),
    }])
    .apply(&default_project())
    .unwrap();
    assert!(next.condition("pet-in-view").is_none());
    assert!(next.rules.iter().all(|r| !r.id.starts_with("pet-")));
    assert!(
        next.element("pet-alert").is_some(),
        "elements stay for the streamer to reuse"
    );
}

#[test]
fn updates_replace_conditions_and_rules_by_id() {
    let base = default_project();
    let mut condition = base.condition("pet-in-view").unwrap().clone();
    condition.prompt = "A dog is visible.".to_owned();
    let mut rule: Rule = base.rules[0].clone();
    rule.enabled = false;
    let next = plan(vec![
        EditOp::UpdateCondition { condition },
        EditOp::UpdateRule { rule: rule.clone() },
        EditOp::RemoveRule {
            id: "pet-hide".to_owned(),
        },
    ])
    .apply(&base)
    .unwrap();
    assert_eq!(next.condition("pet-in-view").unwrap().prompt, "A dog is visible.");
    assert_eq!(next.rules[0], rule);
    assert!(next.rules.iter().all(|r| r.id != "pet-hide"));
}

#[test]
fn refuses_unknown_targets_duplicates_and_invalid_results_without_touching_the_input() {
    let base = default_project();
    let unknown = plan(vec![EditOp::RemoveRule { id: "nope".to_owned() }]).apply(&base);
    assert!(unknown.unwrap_err().0.contains("operation 1: unknown rule"));
    let element = base.elements[0].clone();
    let duplicate = plan(vec![EditOp::AddElement { element }]).apply(&base);
    assert!(duplicate.unwrap_err().0.contains("already exists"));
    let rule = Rule {
        id: "dangling".to_owned(),
        name: "Dangling".to_owned(),
        enabled: true,
        when: Trigger::condition("missing", ConditionEdge::On),
        actions: vec![Action::Effect {
            element: "brb".to_owned(),
            effect: sceneuron_core::Effect::Pop,
        }],
    };
    let invalid = plan(vec![EditOp::AddRule { rule }]).apply(&base);
    assert!(invalid.unwrap_err().0.contains("unknown condition"));
    assert_eq!(base, default_project());
}

#[test]
fn refuses_oversized_plans() {
    let operations = (0..=MAX_PLAN_OPERATIONS)
        .map(|_| EditOp::RemoveRule { id: "x".to_owned() })
        .collect();
    let error = plan(operations).apply(&Project::default()).unwrap_err();
    assert!(error.0.contains("at most"));
}

#[test]
fn rejects_unknown_operations_and_property_names() {
    assert!(serde_json::from_str::<EditPlan>(r#"{"summary": "", "operations": [{"op": "drop_all"}]}"#).is_err());
    let typo = r#"{"summary": "", "operations": [{"op": "update_element", "id": "brb", "props": {"visble": true}}]}"#;
    assert!(serde_json::from_str::<EditPlan>(typo).is_err());
}
