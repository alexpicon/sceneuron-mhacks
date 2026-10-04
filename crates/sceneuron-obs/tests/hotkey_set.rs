// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the hotkey set: names and descriptions of the fixed, element, card, and deck
//! hotkeys, the sync plan for added, removed, and renamed ones, and what each press does.

use std::collections::BTreeMap;

use sceneuron::hotkey_set::{Action, Hotkey, SyncPlan, plan_sync, wanted};
use sceneuron_core::{Engine, EngineError, Outcome, Project, default_project, pack_for_canvas};

fn project_with(packs: &[&str]) -> Project {
    let mut project = default_project();
    for id in packs {
        let pack = pack_for_canvas(id, &project.canvas).unwrap();
        project = project.add_pack(&pack).unwrap();
    }
    project
}

fn hotkey(description: &str, action: Action) -> Hotkey {
    Hotkey {
        description: description.to_owned(),
        action,
    }
}

fn toggle_card(deck: &str, card: &str) -> Action {
    Action::ToggleCard {
        deck: deck.to_owned(),
        card: card.to_owned(),
    }
}

/// The registered hotkeys, by name with their descriptions, after a sync for `project`.
fn registered(project: &Project) -> BTreeMap<String, String> {
    wanted(project)
        .into_iter()
        .map(|(name, hotkey)| (name, hotkey.description))
        .collect()
}

// Names and descriptions

#[test]
fn fixed_and_element_hotkeys_keep_their_names() {
    let project = default_project();
    let wanted = wanted(&project);
    let automation = hotkey(
        "Sceneuron: Pause or resume automatic reactions",
        Action::ToggleAutomation,
    );
    assert_eq!(wanted["sceneuron.automation"], automation);
    let release = hotkey("Sceneuron: Release manual overrides", Action::ReleaseOverrides);
    assert_eq!(wanted["sceneuron.release"], release);
    let brb = hotkey(
        "Sceneuron: Show or hide Be right back",
        Action::ToggleElement("brb".to_owned()),
    );
    assert_eq!(wanted["sceneuron.element.brb"], brb);
    let talk = hotkey("Sceneuron: Push to talk", Action::PushToTalk);
    assert_eq!(wanted["sceneuron.voice.talk"], talk);
    assert_eq!(wanted.len(), 3 + project.elements.len());
}

#[test]
fn every_card_gets_a_toggle_and_every_deck_a_lower_hotkey() {
    let wanted = wanted(&project_with(&["poker", "meeting"]));
    let five = hotkey(
        "Sceneuron: Raise or lower Planning poker: 5",
        toggle_card("poker", "p5"),
    );
    assert_eq!(wanted["sceneuron.card.poker.p5"], five);
    let agree = hotkey(
        "Sceneuron: Raise or lower Meeting signals: Agree",
        toggle_card("meeting", "agree"),
    );
    assert_eq!(wanted["sceneuron.card.meeting.agree"], agree);
    let lower = hotkey("Sceneuron: Lower Planning poker", Action::LowerDeck("poker".to_owned()));
    assert_eq!(wanted["sceneuron.deck.poker.lower"], lower);
    let cards = |deck: &str| {
        let prefix = format!("sceneuron.card.{deck}.");
        wanted.keys().filter(|name| name.starts_with(&prefix)).count()
    };
    assert_eq!((cards("poker"), cards("meeting")), (10, 8));
    assert!(wanted.contains_key("sceneuron.element.poker-card"));
    assert!(wanted.contains_key("sceneuron.deck.meeting.lower"));
}

// Sync plans

#[test]
fn a_first_sync_registers_every_hotkey() {
    let project = project_with(&["poker"]);
    let plan = plan_sync(&BTreeMap::new(), wanted(&project));
    assert!(plan.remove.is_empty() && plan.describe.is_empty());
    let added: Vec<_> = plan.add.into_iter().map(|(name, _)| name).collect();
    let all: Vec<_> = wanted(&project).into_keys().collect();
    assert_eq!(added, all);
}

#[test]
fn a_sync_without_changes_does_nothing() {
    let project = project_with(&["poker"]);
    assert_eq!(plan_sync(&registered(&project), wanted(&project)), SyncPlan::default());
}

#[test]
fn renaming_a_deck_or_card_only_changes_descriptions() {
    let before = project_with(&["poker"]);
    let mut after = before.clone();
    after.decks[0].name = "Sprint poker".to_owned();
    let five = after.decks[0].cards.iter_mut().find(|card| card.id == "p5").unwrap();
    five.name = "Five".to_owned();
    let plan = plan_sync(&registered(&before), wanted(&after));
    assert!(plan.add.is_empty() && plan.remove.is_empty());
    assert_eq!(plan.describe.len(), 11);
    let described: BTreeMap<_, _> = plan.describe.into_iter().collect();
    assert_eq!(
        described["sceneuron.card.poker.p5"],
        "Sceneuron: Raise or lower Sprint poker: Five"
    );
    assert_eq!(described["sceneuron.deck.poker.lower"], "Sceneuron: Lower Sprint poker");
}

#[test]
fn removing_a_deck_unregisters_its_hotkeys_and_keeps_the_fixed_ones() {
    let plan = plan_sync(&registered(&project_with(&["poker"])), wanted(&default_project()));
    assert!(plan.add.is_empty() && plan.describe.is_empty());
    assert_eq!(plan.remove.len(), 12);
    assert!(plan.remove.contains(&"sceneuron.element.poker-card".to_owned()));
    assert!(plan.remove.contains(&"sceneuron.deck.poker.lower".to_owned()));
    assert!(
        plan.remove
            .iter()
            .all(|name| name != "sceneuron.automation" && name != "sceneuron.release")
    );
}

#[test]
fn removing_a_card_unregisters_only_its_hotkey() {
    let before = project_with(&["poker"]);
    let mut after = before.clone();
    after.decks[0].cards.retain(|card| card.id != "break");
    let plan = plan_sync(&registered(&before), wanted(&after));
    assert_eq!(plan.remove, ["sceneuron.card.poker.break"]);
    assert!(plan.add.is_empty() && plan.describe.is_empty());
}

// Presses

#[test]
fn a_card_hotkey_raises_its_card_and_lowers_it_on_the_next_press() {
    let mut engine = Engine::new(project_with(&["poker"]), 0).unwrap();
    let five = toggle_card("poker", "p5");
    let revision = engine.revision();
    assert!(five.apply(&mut engine, 10).unwrap().changed);
    assert_eq!(engine.revision(), revision + 1);
    assert_eq!(engine.raised_card("poker"), Some("p5"));
    toggle_card("poker", "p8").apply(&mut engine, 20).unwrap();
    assert_eq!(engine.raised_card("poker"), Some("p8"));
    toggle_card("poker", "p8").apply(&mut engine, 30).unwrap();
    assert_eq!(engine.raised_card("poker"), None);
}

#[test]
fn the_lower_hotkey_lowers_its_deck() {
    let mut engine = Engine::new(project_with(&["poker", "meeting"]), 0).unwrap();
    toggle_card("poker", "p5").apply(&mut engine, 10).unwrap();
    toggle_card("meeting", "agree").apply(&mut engine, 10).unwrap();
    let lower = Action::LowerDeck("poker".to_owned());
    assert!(lower.apply(&mut engine, 20).unwrap().changed);
    assert_eq!(
        (engine.raised_card("poker"), engine.raised_card("meeting")),
        (None, Some("agree"))
    );
    assert!(!lower.apply(&mut engine, 30).unwrap().changed);
}

#[test]
fn a_press_for_a_removed_card_or_deck_is_refused() {
    let mut engine = Engine::new(project_with(&["poker"]), 0).unwrap();
    let card = toggle_card("poker", "p4").apply(&mut engine, 10);
    assert!(matches!(card, Err(EngineError::UnknownCard { .. })));
    let deck = Action::LowerDeck("votes".to_owned()).apply(&mut engine, 10);
    assert_eq!(deck, Err(EngineError::UnknownDeck("votes".to_owned())));
}

#[test]
fn an_element_hotkey_holds_the_element_shown_or_hidden() {
    let mut engine = Engine::new(default_project(), 0).unwrap();
    let toggle = Action::ToggleElement("name-label".to_owned());
    let shown = engine.effective_props("name-label").unwrap().visible;
    toggle.apply(&mut engine, 10).unwrap();
    assert_eq!(engine.effective_props("name-label").unwrap().visible, !shown);
    toggle.apply(&mut engine, 20).unwrap();
    assert_eq!(engine.effective_props("name-label").unwrap().visible, shown);
    Action::ReleaseOverrides.apply(&mut engine, 30).unwrap();
    assert!(
        engine
            .snapshot(30)
            .elements
            .iter()
            .all(|element| element.overridden.is_empty())
    );
}

#[test]
fn push_to_talk_leaves_the_engine_alone() {
    let mut engine = Engine::new(default_project(), 0).unwrap();
    let before = engine.snapshot(10);
    assert_eq!(Action::PushToTalk.apply(&mut engine, 10), Ok(Outcome::default()));
    assert_eq!(engine.snapshot(10), before);
}

#[test]
fn the_automation_hotkey_pauses_and_resumes() {
    let mut engine = Engine::new(default_project(), 0).unwrap();
    assert!(engine.project().automation_enabled);
    let outcome = Action::ToggleAutomation.apply(&mut engine, 10).unwrap();
    assert!(outcome.project_dirty && !engine.project().automation_enabled);
    Action::ToggleAutomation.apply(&mut engine, 20).unwrap();
    assert!(engine.project().automation_enabled);
}
