// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises card actions on a running engine: raise, toggle, lower, next and previous with
//! wrap-around, the pop on a replaced card, when the raised card went up, lowering every deck,
//! refusals, and the event log.

mod common;

use common::{deck_project, engine, last_event, text, text_element, visible};
use sceneuron_core::{CardAction, Effect, Engine, EngineError, Project, default_project, pack_for_canvas};

fn act(engine: &mut Engine, card: Option<&str>, action: CardAction, now_ms: u64) -> bool {
    engine.card("votes", card, action, now_ms).unwrap().changed
}

fn slot_effect(engine: &Engine) -> Option<(Effect, u64)> {
    let view = engine
        .snapshot(0)
        .elements
        .into_iter()
        .find(|e| e.id == "slot")
        .unwrap();
    view.effect.map(|state| (state.effect, state.nonce))
}

#[test]
fn raising_a_card_shows_it_on_the_slot() {
    let mut engine = engine(deck_project());
    assert!(!visible(&engine, "slot"));
    let revision = engine.revision();
    let outcome = engine.card("votes", Some("no"), CardAction::Raise, 10).unwrap();
    assert!(outcome.changed && !outcome.project_dirty);
    assert_eq!(engine.revision(), revision + 1);
    assert_eq!(engine.raised_card("votes"), Some("no"));
    let props = engine.effective_props("slot").unwrap();
    assert!(props.visible);
    assert_eq!((props.text.as_str(), props.background.as_str()), ("No", "#dc2626ff"));
    assert_eq!(engine.project().element("slot").unwrap().props.text, "base");
}

#[test]
fn raising_the_raised_card_again_changes_nothing() {
    let mut engine = engine(deck_project());
    act(&mut engine, Some("yes"), CardAction::Raise, 10);
    let revision = engine.revision();
    let events = engine.snapshot(20).events.len();
    assert!(!act(&mut engine, Some("yes"), CardAction::Raise, 20));
    assert_eq!(engine.revision(), revision);
    assert_eq!(engine.snapshot(20).events.len(), events);
}

#[test]
fn replacing_a_raised_card_pops_the_slot_but_raising_from_lowered_does_not() {
    let mut engine = engine(deck_project());
    act(&mut engine, Some("yes"), CardAction::Raise, 10);
    assert_eq!(slot_effect(&engine), None, "the enter animation shows the first card");
    act(&mut engine, Some("no"), CardAction::Raise, 20);
    let (effect, first) = slot_effect(&engine).unwrap();
    assert_eq!(effect, Effect::Pop);
    assert_eq!(text(&engine, "slot"), "No");
    act(&mut engine, None, CardAction::Next, 30);
    let (_, second) = slot_effect(&engine).unwrap();
    assert!(second > first, "every replacement replays the pop");
}

#[test]
fn the_deck_view_says_when_its_card_went_up_and_a_raise_again_keeps_it() {
    let raised_at = |engine: &Engine| engine.snapshot(0).decks[0].raised_at_ms;
    let mut engine = engine(deck_project());
    assert_eq!(raised_at(&engine), None);
    act(&mut engine, Some("yes"), CardAction::Raise, 10);
    assert_eq!(raised_at(&engine), Some(10));
    engine.card("votes", Some("yes"), CardAction::Raise, 20).unwrap();
    assert_eq!(
        raised_at(&engine),
        Some(10),
        "the same card stays up from when it went up"
    );
    act(&mut engine, Some("no"), CardAction::Raise, 30);
    assert_eq!(raised_at(&engine), Some(30));
    act(&mut engine, None, CardAction::Lower, 40);
    assert_eq!(raised_at(&engine), None);
}

#[test]
fn toggle_raises_a_lowered_card_and_lowers_the_raised_one() {
    let mut engine = engine(deck_project());
    assert!(act(&mut engine, Some("maybe"), CardAction::Toggle, 10));
    assert_eq!(engine.raised_card("votes"), Some("maybe"));
    assert!(act(&mut engine, Some("yes"), CardAction::Toggle, 20));
    assert_eq!(engine.raised_card("votes"), Some("yes"), "another card replaces it");
    assert!(act(&mut engine, Some("yes"), CardAction::Toggle, 30));
    assert_eq!(engine.raised_card("votes"), None);
    assert!(!visible(&engine, "slot"));
    assert_eq!(text(&engine, "slot"), "base");
}

#[test]
fn lower_ignores_the_card_and_changes_nothing_when_lowered() {
    let mut engine = engine(deck_project());
    let revision = engine.revision();
    assert!(!act(&mut engine, None, CardAction::Lower, 10));
    assert_eq!(engine.revision(), revision);
    act(&mut engine, Some("no"), CardAction::Raise, 20);
    assert!(act(&mut engine, Some("ghost"), CardAction::Lower, 30));
    assert_eq!(engine.raised_card("votes"), None);
}

#[test]
fn next_and_previous_start_at_the_ends_and_wrap_around() {
    let mut engine = engine(deck_project());
    act(&mut engine, None, CardAction::Next, 10);
    assert_eq!(engine.raised_card("votes"), Some("yes"));
    let forward: Vec<_> = (0..3)
        .map(|n| {
            act(&mut engine, Some("ignored"), CardAction::Next, 20 + n);
            engine.raised_card("votes").unwrap().to_owned()
        })
        .collect();
    assert_eq!(forward, ["no", "maybe", "yes"]);
    act(&mut engine, None, CardAction::Lower, 30);
    act(&mut engine, None, CardAction::Previous, 40);
    assert_eq!(engine.raised_card("votes"), Some("maybe"));
    let backward: Vec<_> = (0..3)
        .map(|n| {
            act(&mut engine, None, CardAction::Previous, 50 + n);
            engine.raised_card("votes").unwrap().to_owned()
        })
        .collect();
    assert_eq!(backward, ["no", "yes", "maybe"]);
}

#[test]
fn stepping_through_a_one_card_deck_keeps_that_card() {
    let mut project = deck_project();
    project.decks[0].cards.truncate(1);
    let mut engine = engine(project);
    assert!(act(&mut engine, None, CardAction::Previous, 10));
    assert!(!act(&mut engine, None, CardAction::Next, 20));
    assert_eq!(engine.raised_card("votes"), Some("yes"));
    assert_eq!(slot_effect(&engine), None);
}

#[test]
fn refuses_unknown_decks_and_cards_and_missing_cards_without_changing_anything() {
    let mut engine = engine(deck_project());
    act(&mut engine, Some("yes"), CardAction::Raise, 10);
    let revision = engine.revision();
    assert_eq!(
        engine.card("ghost", Some("yes"), CardAction::Raise, 20),
        Err(EngineError::UnknownDeck("ghost".into()))
    );
    assert_eq!(
        engine.card("ghost", None, CardAction::Lower, 20),
        Err(EngineError::UnknownDeck("ghost".into()))
    );
    for action in [CardAction::Raise, CardAction::Toggle] {
        let unknown = engine.card("votes", Some("ghost"), action, 20).unwrap_err();
        let expected = EngineError::UnknownCard {
            deck: "votes".into(),
            card: "ghost".into(),
        };
        assert_eq!(unknown, expected);
        assert_eq!(unknown.to_string(), "deck \"votes\" has no card \"ghost\"");
        assert_eq!(engine.card("votes", None, action, 20), Err(EngineError::MissingCard));
    }
    assert_eq!(engine.raised_card("votes"), Some("yes"));
    assert_eq!(engine.revision(), revision);
    assert_eq!(engine.raised_card("ghost"), None);
}

#[test]
fn card_changes_are_logged_with_deck_and_card_names() {
    let mut engine = engine(deck_project());
    act(&mut engine, Some("maybe"), CardAction::Raise, 10);
    assert_eq!(
        last_event(&engine, 10),
        ("manual".to_owned(), "card raised: Votes: Maybe".to_owned())
    );
    act(&mut engine, None, CardAction::Lower, 20);
    assert_eq!(
        last_event(&engine, 20),
        ("manual".to_owned(), "card lowered: Votes".to_owned())
    );
}

#[test]
fn pack_cards_log_in_plain_words() {
    let pack = pack_for_canvas("poker", &Default::default()).unwrap();
    let mut engine = engine(default_project().add_pack(&pack).unwrap());
    engine.card("poker", Some("p5"), CardAction::Raise, 10).unwrap();
    assert_eq!(last_event(&engine, 10).1, "card raised: Planning poker: 5");
    assert_eq!(text(&engine, "poker-card"), "5");
    engine.card("poker", None, CardAction::Lower, 20).unwrap();
    assert_eq!(last_event(&engine, 20).1, "card lowered: Planning poker");
}

fn two_decks() -> Project {
    let mut project = deck_project();
    project.elements.push(text_element("slot-2", "Second slot"));
    let mut deck = project.decks[0].clone();
    deck.id = "votes-2".into();
    deck.name = "More votes".into();
    deck.element = "slot-2".into();
    project.decks.push(deck);
    project
}

#[test]
fn decks_raise_independently_and_lower_all_lowers_every_one() {
    let mut engine = engine(two_decks());
    let revision = engine.revision();
    assert!(!engine.lower_all_cards(5).changed);
    assert_eq!(engine.revision(), revision);
    act(&mut engine, Some("yes"), CardAction::Raise, 10);
    engine.card("votes-2", Some("no"), CardAction::Raise, 20).unwrap();
    assert_eq!(
        (text(&engine, "slot"), text(&engine, "slot-2")),
        ("Yes".into(), "No".into())
    );
    let revision = engine.revision();
    assert!(engine.lower_all_cards(30).changed);
    assert_eq!(engine.revision(), revision + 1);
    assert!(!visible(&engine, "slot") && !visible(&engine, "slot-2"));
    assert_eq!(
        (engine.raised_card("votes"), engine.raised_card("votes-2")),
        (None, None)
    );
    let lowered: Vec<_> = engine
        .snapshot(30)
        .events
        .into_iter()
        .take(2)
        .map(|e| e.detail)
        .collect();
    assert!(lowered.contains(&"card lowered: Votes".to_owned()), "{lowered:?}");
    assert!(lowered.contains(&"card lowered: More votes".to_owned()), "{lowered:?}");
}
