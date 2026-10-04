// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises overlay-wide emoji floods as rule actions and manual requests: their JSON, a
//! project rule that floods on any trigger, a manual flood that ignores the cooldown and restarts
//! it, and floods surviving the removal of elements.

use sceneuron_core::{
    Action, EditOp, EditPlan, Engine, Flood, Presence, Project, Rule, TONGUE_OUT, Trigger, Verdict, default_project,
};

fn flooding_rule(when: Trigger) -> Rule {
    Rule {
        id: "flood-on-leave".to_owned(),
        name: "Flood when I leave".to_owned(),
        enabled: true,
        when,
        actions: vec![Action::Flood { flood: Flood::Tongue }],
    }
}

fn with_rule(rule: Rule) -> Project {
    let mut project = default_project();
    project.rules.push(rule);
    project
}

fn tongue_out(engine: &mut Engine, at_ms: u64) {
    let verdict = Verdict {
        condition: TONGUE_OUT.to_owned(),
        holds: true,
        confidence: 0.95,
        detail: String::new(),
        reason: "The tongue is out.".to_owned(),
    };
    engine.observe_verdicts(vec![verdict], at_ms);
}

#[test]
fn a_flood_action_names_no_element() {
    let action: Action = serde_json::from_str(r#"{"type": "flood", "flood": "tongue"}"#).unwrap();
    assert_eq!(action, Action::Flood { flood: Flood::Tongue });
    assert_eq!(action.element(), None);
    assert!(serde_json::from_str::<Action>(r#"{"type": "flood", "flood": "confetti"}"#).is_err());
    assert!(with_rule(flooding_rule(Trigger::away())).validate().is_ok());
}

#[test]
fn a_project_rule_floods_on_its_trigger_whatever_the_switch() {
    let mut project = with_rule(flooding_rule(Trigger::away()));
    project.reactions.enabled = false;
    let mut engine = Engine::new(project, 0).unwrap();
    engine.simulate_presence(Presence::Present, 1_000);
    assert_eq!(engine.flood(), None);
    engine.simulate_presence(Presence::Away, 2_000);
    let flood = engine.flood().expect("the rule's flood");
    assert_eq!((flood.flood, flood.at_ms), (Flood::Tongue, 2_000));
    let log = engine.snapshot(2_000).events;
    assert_eq!(log[0].detail, r#"ran "Flood when I leave""#);
    let published = serde_json::to_value(engine.snapshot(2_000)).unwrap();
    assert_eq!(
        published["flood"],
        serde_json::json!({"flood": "tongue", "nonce": flood.nonce, "at_ms": 2_000})
    );
}

#[test]
fn rules_do_not_flood_while_automation_is_paused() {
    let mut engine = Engine::new(with_rule(flooding_rule(Trigger::away())), 0).unwrap();
    engine.set_automation(false, 500);
    engine.simulate_presence(Presence::Away, 1_000);
    assert_eq!(engine.flood(), None);
}

#[test]
fn a_manual_flood_ignores_the_cooldown_and_restarts_it() {
    let mut project = default_project();
    project.reactions.cooldown_ms = 20_000;
    let mut engine = Engine::new(project, 0).unwrap();
    tongue_out(&mut engine, 1_000);
    let first = engine.flood().unwrap().nonce;
    let outcome = engine.trigger_flood(Flood::Tongue, 5_000);
    assert!(outcome.changed && !outcome.project_dirty);
    let manual = engine.flood().unwrap();
    assert!(manual.nonce > first);
    assert_eq!(manual.at_ms, 5_000);
    let event = &engine.snapshot(5_000).events[0];
    assert_eq!(
        (event.source.as_str(), event.detail.as_str()),
        ("manual", "flooded the overlay with 😜 😛 🤪 😝")
    );
    let mut later = engine.clone();
    later.reset_conditions("test", 6_000);
    tongue_out(&mut later, 22_000);
    assert_eq!(
        later.flood().unwrap().at_ms,
        5_000,
        "the manual flood restarted the cooldown"
    );
    later.reset_conditions("test", 23_000);
    tongue_out(&mut later, 25_000);
    assert_eq!(later.flood().unwrap().at_ms, 25_000);
}

#[test]
fn removing_an_element_keeps_flood_actions() {
    let mut rule = flooding_rule(Trigger::away());
    rule.actions.push(Action::Effect {
        element: "brb".to_owned(),
        effect: sceneuron_core::Effect::Shake,
    });
    let plan = EditPlan {
        summary: "Remove the BRB card.".to_owned(),
        operations: vec![EditOp::RemoveElement { id: "brb".to_owned() }],
    };
    let next = plan.apply(&with_rule(rule)).unwrap();
    let kept = next.rules.iter().find(|rule| rule.id == "flood-on-leave").unwrap();
    assert_eq!(kept.actions, [Action::Flood { flood: Flood::Tongue }]);
}
