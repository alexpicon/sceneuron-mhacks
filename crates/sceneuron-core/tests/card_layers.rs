// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises where raised cards sit among the overlay's layers (above rules, below manual
//! holds), auto-hide through `tick`, and the decks in the snapshot.

mod common;

use common::{auto_hide_project, deck_project, engine, last_event, text, visible};
use sceneuron_core::{Action, CardAction, Engine, Presence, Project, PropsPatch, Rule, SceneObservation, Trigger};
use serde_json::json;

/// [`deck_project`] plus a rule that hides the slot, colors it, and changes its text when the
/// streamer leaves.
fn ruled_project() -> Project {
    let mut project = deck_project();
    project.rules.push(Rule {
        id: "away-slot".to_owned(),
        name: "Slot when away".to_owned(),
        enabled: true,
        when: Trigger::away(),
        actions: vec![Action::SetProps {
            element: "slot".to_owned(),
            props: PropsPatch {
                visible: Some(false),
                text: Some("Away".to_owned()),
                color: Some("#ffcc00".to_owned()),
                ..PropsPatch::default()
            },
        }],
    });
    project
}

fn raise(engine: &mut Engine, card: &str, now_ms: u64) {
    engine.card("votes", Some(card), CardAction::Raise, now_ms).unwrap();
}

#[test]
fn a_raised_card_wins_over_rules_and_lowering_restores_them() {
    let mut engine = engine(ruled_project());
    engine.simulate_presence(Presence::Away, 10);
    raise(&mut engine, "yes", 20);
    let props = engine.effective_props("slot").unwrap();
    assert!(props.visible, "a rule must not hide a raised card");
    assert_eq!(props.text, "Yes");
    assert_eq!(
        props.color, "#ffcc00",
        "fields the card leaves alone keep the rule's value"
    );
    engine.card("votes", None, CardAction::Lower, 30).unwrap();
    assert!(!visible(&engine, "slot"));
    assert_eq!(text(&engine, "slot"), "Away");
}

#[test]
fn rules_that_run_while_a_card_is_up_stay_underneath() {
    let mut engine = engine(ruled_project());
    raise(&mut engine, "no", 10);
    engine.simulate_presence(Presence::Away, 20);
    assert!(visible(&engine, "slot"));
    assert_eq!(text(&engine, "slot"), "No");
}

#[test]
fn manual_holds_win_over_a_raised_card() {
    let mut engine = engine(deck_project());
    let hide = PropsPatch {
        visible: Some(false),
        text: Some("Held".to_owned()),
        ..PropsPatch::default()
    };
    engine.set_override("slot", &hide, 10).unwrap();
    raise(&mut engine, "yes", 20);
    assert!(!visible(&engine, "slot"), "a held visibility outranks the card");
    assert_eq!(engine.raised_card("votes"), Some("yes"));
    engine.release_overrides(Some("slot"), 30).unwrap();
    assert!(visible(&engine, "slot"));
    assert_eq!(text(&engine, "slot"), "Yes");
}

#[test]
fn releasing_overrides_and_pausing_automation_leave_cards_up() {
    let mut engine = engine(deck_project());
    raise(&mut engine, "maybe", 10);
    engine.release_overrides(None, 20).unwrap();
    engine.set_automation(false, 30);
    raise(&mut engine, "no", 40);
    assert_eq!(engine.raised_card("votes"), Some("no"));
    assert!(visible(&engine, "slot"));
}

#[test]
fn card_text_renders_placeholders() {
    let mut project = deck_project();
    project.decks[0].cards[0].props.text = Some("Seen: {{scene}}".to_owned());
    let mut engine = engine(project);
    let observation = SceneObservation {
        caption: "a person waving".to_owned(),
        verdicts: Vec::new(),
    };
    engine.observe_scene(observation, 5);
    raise(&mut engine, "yes", 10);
    assert_eq!(text(&engine, "slot"), "Seen: a person waving");
}

#[test]
fn tick_lowers_a_card_when_its_auto_hide_delay_ends() {
    let mut engine = engine(auto_hide_project(15_000));
    raise(&mut engine, "yes", 1_000);
    assert!(!engine.tick(15_999).changed);
    assert_eq!(engine.raised_card("votes"), Some("yes"));
    let revision = engine.revision();
    assert!(engine.tick(16_000).changed);
    assert_eq!(engine.revision(), revision + 1);
    assert_eq!(engine.raised_card("votes"), None);
    assert!(!visible(&engine, "slot"));
    assert_eq!(
        last_event(&engine, 16_000),
        ("auto-hide".to_owned(), "card lowered: Votes".to_owned())
    );
    assert!(!engine.tick(17_000).changed);
}

#[test]
fn raising_again_or_stepping_restarts_the_auto_hide_delay() {
    let mut engine = engine(auto_hide_project(15_000));
    raise(&mut engine, "yes", 1_000);
    assert!(
        engine
            .card("votes", Some("yes"), CardAction::Raise, 10_000)
            .unwrap()
            .changed
    );
    assert!(!engine.tick(16_000).changed);
    engine.card("votes", None, CardAction::Next, 20_000).unwrap();
    assert!(!engine.tick(34_999).changed);
    assert_eq!(engine.raised_card("votes"), Some("no"));
    assert!(engine.tick(35_000).changed);
}

#[test]
fn cards_without_auto_hide_stay_up_and_lowered_decks_have_no_deadline() {
    let mut engine = engine(deck_project());
    raise(&mut engine, "yes", 1_000);
    assert!(!engine.tick(10_000_000).changed);
    assert_eq!(engine.raised_card("votes"), Some("yes"));
    let mut engine = common::engine(auto_hide_project(5_000));
    raise(&mut engine, "yes", 1_000);
    engine.card("votes", None, CardAction::Lower, 2_000).unwrap();
    assert!(!engine.tick(6_000).changed);
}

#[test]
fn snapshots_list_every_deck_with_its_raised_card() {
    let mut engine = engine(auto_hide_project(15_000));
    let lowered = serde_json::to_value(engine.snapshot(0)).unwrap();
    assert_eq!(
        lowered["decks"],
        json!([{"id": "votes", "name": "Votes", "element": "slot", "raised": null,
                "raised_by": null, "raised_at_ms": null, "hide_at_ms": null, "covered": false,
                "vote": null, "summary": null}])
    );
    raise(&mut engine, "maybe", 2_000);
    let raised = serde_json::to_value(engine.snapshot(2_000)).unwrap();
    assert_eq!(
        raised["decks"][0],
        json!({"id": "votes", "name": "Votes", "element": "slot", "raised": "maybe",
               "raised_by": "manual", "raised_at_ms": 2_000, "hide_at_ms": 17_000, "covered": false,
               "vote": null, "summary": null})
    );
    let slot = raised["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|element| element["id"] == "slot")
        .unwrap();
    assert_eq!(slot["props"]["visible"], true);
    assert_eq!(slot["props"]["text"], "Maybe");
}
