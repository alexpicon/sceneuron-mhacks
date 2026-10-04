// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises saving a new project into a running engine: the automatic layer is recomputed,
//! the live automation switch is kept, raised cards survive only while their deck, card, and
//! element do and follow a changed auto-hide delay, and the project revision advances.

mod common;

use common::{auto_hide_project, deck_project};
use sceneuron_core::{CardAction, Engine, Presence, Project, SceneObservation, Verdict, default_project};

fn visible(engine: &Engine, id: &str) -> bool {
    engine.effective_props(id).unwrap().visible
}

fn pet_seen(engine: &mut Engine, at_ms: u64) {
    let observation = SceneObservation {
        caption: String::new(),
        verdicts: vec![Verdict {
            condition: "pet-in-view".to_owned(),
            holds: true,
            confidence: 0.9,
            detail: "Dog".to_owned(),
            reason: String::new(),
        }],
    };
    engine.observe_scene(observation, at_ms);
}

#[test]
fn removed_rules_stop_affecting_the_overlay() {
    let mut engine = Engine::new(default_project(), 0).unwrap();
    engine.simulate_presence(Presence::Away, 10);
    assert!(visible(&engine, "brb"));
    let mut project = default_project();
    project.rules.retain(|rule| rule.id != "away-show-brb");
    engine.replace_project(project, 20).unwrap();
    assert!(!visible(&engine, "brb"));
}

#[test]
fn new_rules_apply_to_the_current_state_without_replaying_effects() {
    let mut project = default_project();
    project.rules.retain(|rule| rule.id != "pet-show");
    let mut engine = Engine::new(project, 0).unwrap();
    pet_seen(&mut engine, 10);
    assert!(!visible(&engine, "pet-alert"));
    engine.replace_project(default_project(), 20).unwrap();
    assert!(visible(&engine, "pet-alert"));
    let frame = engine
        .snapshot(20)
        .elements
        .into_iter()
        .find(|e| e.id == "cam-frame")
        .unwrap();
    assert!(frame.effect.is_none());
}

#[test]
fn the_latest_state_wins_when_rules_overlap() {
    let mut project = default_project();
    let pet_hides_brb = sceneuron_core::Action::SetProps {
        element: "brb".to_owned(),
        props: sceneuron_core::PropsPatch {
            visible: Some(false),
            ..Default::default()
        },
    };
    let pet = project.rules.iter().position(|rule| rule.id == "pet-show").unwrap();
    project.rules[pet].actions.push(pet_hides_brb);
    let mut engine = Engine::new(project.clone(), 0).unwrap();
    engine.simulate_presence(Presence::Away, 10);
    pet_seen(&mut engine, 20);
    assert!(!visible(&engine, "brb"));
    engine.replace_project(project, 30).unwrap();
    assert!(
        !visible(&engine, "brb"),
        "the pet arrived after the streamer left, so it wins"
    );
}

#[test]
fn saving_keeps_the_live_automation_switch_and_advances_the_revision() {
    let mut engine = Engine::new(default_project(), 0).unwrap();
    engine.set_automation(false, 1);
    let revision = engine.project_revision();
    let mut project = default_project();
    project.automation_enabled = true;
    engine.replace_project(project, 2).unwrap();
    assert!(!engine.project().automation_enabled);
    assert_eq!(engine.project_revision(), revision + 1);
    assert_eq!(engine.snapshot(2).project_revision, revision + 1);
}

#[test]
fn paused_engines_keep_their_frozen_automatic_values() {
    let mut engine = Engine::new(default_project(), 0).unwrap();
    engine.simulate_presence(Presence::Away, 10);
    engine.set_automation(false, 20);
    engine.replace_project(default_project(), 30).unwrap();
    assert!(visible(&engine, "brb"));
}

#[test]
fn removed_conditions_lose_their_state() {
    let mut engine = Engine::new(default_project(), 0).unwrap();
    pet_seen(&mut engine, 10);
    let mut project = default_project();
    project.conditions.retain(|condition| condition.id != "pet-in-view");
    project.rules.retain(|rule| !rule.id.starts_with("pet-"));
    engine.replace_project(project, 20).unwrap();
    assert!(!engine.condition_on("pet-in-view"));
    assert!(engine.snapshot(20).conditions.iter().all(|c| c.id != "pet-in-view"));
    assert!(!visible(&engine, "pet-alert"));
}

fn raised_engine(project: Project) -> Engine {
    let mut engine = Engine::new(project, 0).unwrap();
    engine.card("votes", Some("no"), CardAction::Raise, 10).unwrap();
    engine
}

#[test]
fn raised_cards_survive_saves_that_keep_their_deck_card_and_element() {
    let mut engine = raised_engine(deck_project());
    let mut project = deck_project();
    project.decks[0].name = "Renamed".to_owned();
    project.decks[0].cards[1].props.text = Some("Nope".to_owned());
    project.decks[0].cards.swap(0, 2);
    engine.replace_project(project, 20).unwrap();
    assert_eq!(engine.raised_card("votes"), Some("no"));
    assert_eq!(
        engine.effective_props("slot").unwrap().text,
        "Nope",
        "the saved card shows"
    );
    assert!(visible(&engine, "slot"));
}

#[test]
fn saves_that_remove_the_deck_card_or_element_lower_the_card() {
    let removals: [fn(&mut Project); 3] = [
        |project| project.decks.clear(),
        |project| project.decks[0].cards.retain(|card| card.id != "no"),
        |project| {
            project.decks.clear();
            project.elements.retain(|element| element.id != "slot");
        },
    ];
    for remove in removals {
        let mut engine = raised_engine(deck_project());
        let mut project = deck_project();
        remove(&mut project);
        engine.replace_project(project, 20).unwrap();
        assert_eq!(engine.raised_card("votes"), None);
        assert!(engine.snapshot(20).decks.iter().all(|deck| deck.raised.is_none()));
        if engine.project().element("slot").is_some() {
            assert!(!visible(&engine, "slot"));
        }
        engine.replace_project(deck_project(), 30).unwrap();
        assert_eq!(
            engine.raised_card("votes"),
            None,
            "restoring the deck does not raise it again"
        );
    }
}

#[test]
fn a_kept_card_keeps_its_auto_hide_deadline() {
    let mut engine = raised_engine(auto_hide_project(15_000));
    engine.replace_project(auto_hide_project(15_000), 5_000).unwrap();
    assert_eq!(engine.snapshot(5_000).decks[0].hide_at_ms, Some(15_010));
    assert!(engine.tick(15_010).changed);
    assert_eq!(engine.raised_card("votes"), None);
}

#[test]
fn a_kept_card_follows_a_changed_auto_hide_delay_from_the_save() {
    let mut engine = raised_engine(auto_hide_project(15_000));
    engine.replace_project(auto_hide_project(0), 5_000).unwrap();
    assert_eq!(engine.snapshot(5_000).decks[0].hide_at_ms, None);
    assert!(
        !engine.tick(60_000).changed,
        "the card now stays up until it is lowered"
    );
    assert_eq!(engine.raised_card("votes"), Some("no"));

    let mut engine = raised_engine(deck_project());
    engine.replace_project(auto_hide_project(15_000), 5_000).unwrap();
    assert_eq!(engine.snapshot(5_000).decks[0].hide_at_ms, Some(20_000));
    assert!(!engine.tick(19_999).changed);
    assert!(engine.tick(20_000).changed);
    assert_eq!(engine.raised_card("votes"), None);

    let mut engine = raised_engine(auto_hide_project(15_000));
    engine.replace_project(auto_hide_project(30_000), 5_000).unwrap();
    assert_eq!(engine.snapshot(5_000).decks[0].hide_at_ms, Some(35_000));
}
