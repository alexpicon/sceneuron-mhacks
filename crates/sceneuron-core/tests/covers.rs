// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises covered cards: a deck's cover in place of its raised card, covering before a raise,
//! keeping the cover across raises, uncovering with a pop, every way of lowering clearing it,
//! refusals, manual holds above it, project saves, and `covered` in the snapshot.

mod common;

use common::{auto_hide_project, deck_project, engine, last_event, text, visible};
use sceneuron_core::{CardAction, Effect, Engine, EngineError, PropsPatch};

fn cover() -> PropsPatch {
    PropsPatch {
        text: Some("Voted".to_owned()),
        background: Some("#1d4ed8ff".to_owned()),
        ..PropsPatch::default()
    }
}

fn raise(engine: &mut Engine, card: &str, now_ms: u64) {
    engine.card("votes", Some(card), CardAction::Raise, now_ms).unwrap();
}

fn covered(engine: &Engine) -> bool {
    engine
        .snapshot(0)
        .decks
        .iter()
        .any(|deck| deck.id == "votes" && deck.covered)
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
fn a_covered_card_shows_the_cover_and_uncovering_pops_the_card_back() {
    let mut engine = engine(deck_project());
    raise(&mut engine, "yes", 10);
    let revision = engine.revision();
    assert!(engine.cover_card("votes", Some(cover()), 20).unwrap().changed);
    assert_eq!(engine.revision(), revision + 1);
    let props = engine.effective_props("slot").unwrap();
    assert!(props.visible);
    assert_eq!((props.text.as_str(), props.background.as_str()), ("Voted", "#1d4ed8ff"));
    assert!(covered(&engine));
    assert_eq!(engine.cover("votes"), Some(&cover()));
    assert_eq!(
        engine.raised_card("votes"),
        Some("yes"),
        "the card stays raised underneath"
    );
    assert_eq!(
        last_event(&engine, 20),
        ("cards".to_owned(), "card covered: Votes".to_owned())
    );
    assert!(engine.cover_card("votes", None, 30).unwrap().changed);
    assert_eq!(text(&engine, "slot"), "Yes");
    assert_eq!(slot_effect(&engine).map(|(effect, _)| effect), Some(Effect::Pop));
    assert!(!covered(&engine));
    assert_eq!(
        last_event(&engine, 30),
        ("cards".to_owned(), "card uncovered: Votes".to_owned())
    );
}

#[test]
fn covering_a_lowered_deck_covers_the_next_card_raised() {
    let mut engine = engine(deck_project());
    let revision = engine.revision();
    let armed = engine.cover_card("votes", Some(cover()), 10).unwrap();
    assert!(!armed.changed, "nothing shows differently while the deck is lowered");
    assert_eq!(engine.revision(), revision);
    assert!(!covered(&engine) && !visible(&engine, "slot"));
    raise(&mut engine, "no", 20);
    assert_eq!(text(&engine, "slot"), "Voted", "the raised card never shows");
    assert!(covered(&engine));
}

#[test]
fn raising_another_card_keeps_the_cover() {
    let mut engine = engine(deck_project());
    engine.cover_card("votes", Some(cover()), 10).unwrap();
    raise(&mut engine, "yes", 20);
    raise(&mut engine, "maybe", 30);
    assert_eq!(engine.raised_card("votes"), Some("maybe"));
    assert_eq!(text(&engine, "slot"), "Voted");
    engine.cover_card("votes", None, 40).unwrap();
    assert_eq!(text(&engine, "slot"), "Maybe");
}

#[test]
fn lowering_clears_the_cover() {
    let mut engine = engine(deck_project());
    engine.cover_card("votes", Some(cover()), 10).unwrap();
    raise(&mut engine, "yes", 20);
    engine.card("votes", None, CardAction::Lower, 30).unwrap();
    assert_eq!(engine.cover("votes"), None);
    raise(&mut engine, "no", 40);
    assert_eq!(text(&engine, "slot"), "No");
    engine.cover_card("votes", Some(cover()), 50).unwrap();
    engine.card("votes", Some("no"), CardAction::Toggle, 60).unwrap();
    assert_eq!(engine.cover("votes"), None, "toggling the raised card off lowers it");
    raise(&mut engine, "yes", 70);
    engine.cover_card("votes", Some(cover()), 80).unwrap();
    assert!(engine.lower_all_cards(90).changed);
    assert_eq!(engine.cover("votes"), None);
}

#[test]
fn auto_hide_clears_the_cover() {
    let mut engine = engine(auto_hide_project(5_000));
    engine.cover_card("votes", Some(cover()), 0).unwrap();
    raise(&mut engine, "yes", 1_000);
    assert!(engine.tick(6_000).changed);
    assert_eq!((engine.raised_card("votes"), engine.cover("votes")), (None, None));
}

#[test]
fn covering_twice_or_uncovering_an_uncovered_deck_changes_nothing() {
    let mut engine = engine(deck_project());
    raise(&mut engine, "yes", 10);
    assert!(!engine.cover_card("votes", None, 20).unwrap().changed);
    engine.cover_card("votes", Some(cover()), 30).unwrap();
    let revision = engine.revision();
    assert!(!engine.cover_card("votes", Some(cover()), 40).unwrap().changed);
    assert_eq!(engine.revision(), revision);
    let mut other = cover();
    other.text = Some("Hidden".to_owned());
    assert!(engine.cover_card("votes", Some(other), 50).unwrap().changed);
    assert_eq!(text(&engine, "slot"), "Hidden");
    engine.card("votes", None, CardAction::Lower, 60).unwrap();
    assert!(!engine.cover_card("votes", None, 70).unwrap().changed);
}

#[test]
fn refuses_unknown_decks_visibility_and_invalid_values() {
    let mut engine = engine(deck_project());
    raise(&mut engine, "yes", 10);
    let revision = engine.revision();
    assert_eq!(
        engine.cover_card("nope", Some(cover()), 20),
        Err(EngineError::UnknownDeck("nope".to_owned()))
    );
    let mut shown = cover();
    shown.visible = Some(true);
    assert!(matches!(
        engine.cover_card("votes", Some(shown), 20),
        Err(EngineError::Invalid(_))
    ));
    let mut bad = cover();
    bad.color = Some("blue".to_owned());
    assert!(matches!(
        engine.cover_card("votes", Some(bad), 20),
        Err(EngineError::Invalid(_))
    ));
    assert_eq!(engine.revision(), revision);
    assert_eq!(engine.cover("votes"), None);
}

#[test]
fn a_manual_hold_still_wins_over_the_cover() {
    let mut engine = engine(deck_project());
    raise(&mut engine, "yes", 10);
    engine.cover_card("votes", Some(cover()), 20).unwrap();
    let held = PropsPatch {
        text: Some("Held".to_owned()),
        ..PropsPatch::default()
    };
    engine.set_override("slot", &held, 30).unwrap();
    assert_eq!(text(&engine, "slot"), "Held");
    engine.release_overrides(Some("slot"), 40).unwrap();
    assert_eq!(text(&engine, "slot"), "Voted");
}

#[test]
fn saving_the_project_keeps_the_cover_and_removing_the_deck_drops_it() {
    let mut engine = engine(deck_project());
    raise(&mut engine, "yes", 10);
    engine.cover_card("votes", Some(cover()), 20).unwrap();
    let project = engine.project().clone();
    engine.replace_project(project.clone(), 30).unwrap();
    assert_eq!(
        text(&engine, "slot"),
        "Voted",
        "an editor save never shows the covered card"
    );
    let mut without_card = project.clone();
    without_card.decks[0].cards.retain(|card| card.id != "yes");
    engine.replace_project(without_card, 40).unwrap();
    assert_eq!((engine.raised_card("votes"), engine.cover("votes")), (None, None));
    engine.replace_project(project.clone(), 50).unwrap();
    engine.cover_card("votes", Some(cover()), 60).unwrap();
    let mut without_deck = project;
    without_deck.decks.clear();
    engine.replace_project(without_deck, 70).unwrap();
    assert_eq!(engine.cover("votes"), None, "a removed deck takes its cover along");
}
