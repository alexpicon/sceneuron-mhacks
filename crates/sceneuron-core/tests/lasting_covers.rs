// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises covers that last until the deck is uncovered: every way of lowering keeps them, so a
//! card raised right after a lower is covered in the change that raises it; uncovering clears
//! them; a card's cover can become a lasting one without a visible change; and project saves keep
//! them until the deck itself is removed.

mod common;

use common::{auto_hide_project, deck_project, engine, text, visible};
use sceneuron_core::{CardAction, Effect, Engine, PropsPatch};

fn cover() -> PropsPatch {
    PropsPatch {
        text: Some("Voted".to_owned()),
        ..PropsPatch::default()
    }
}

fn act(engine: &mut Engine, card: Option<&str>, action: CardAction, now_ms: u64) {
    engine.card("votes", card, action, now_ms).unwrap();
}

#[test]
fn every_way_of_lowering_keeps_the_cover_so_no_card_ever_shows() {
    let mut engine = engine(deck_project());
    engine.cover_deck("votes", cover(), 0).unwrap();
    act(&mut engine, Some("yes"), CardAction::Raise, 10);
    assert_eq!(text(&engine, "slot"), "Voted");
    act(&mut engine, None, CardAction::Lower, 20);
    assert_eq!(engine.cover("votes"), Some(&cover()), "lowering keeps a lasting cover");
    assert!(!visible(&engine, "slot"));
    act(&mut engine, Some("no"), CardAction::Raise, 30);
    assert_eq!(
        text(&engine, "slot"),
        "Voted",
        "a card raised right after the lower is covered"
    );
    act(&mut engine, Some("no"), CardAction::Toggle, 40);
    assert_eq!(
        (engine.raised_card("votes"), engine.cover("votes")),
        (None, Some(&cover()))
    );
    act(&mut engine, Some("yes"), CardAction::Raise, 45);
    assert!(engine.lower_all_cards(50).changed);
    act(&mut engine, Some("maybe"), CardAction::Raise, 60);
    assert_eq!(text(&engine, "slot"), "Voted");
    let mut timed = common::engine(auto_hide_project(5_000));
    timed.cover_deck("votes", cover(), 0).unwrap();
    timed.card("votes", Some("yes"), CardAction::Raise, 1_000).unwrap();
    assert!(timed.tick(6_000).changed);
    assert_eq!(
        (timed.raised_card("votes"), timed.cover("votes")),
        (None, Some(&cover()))
    );
}

#[test]
fn uncovering_clears_it_and_shows_the_card_with_a_pop() {
    let mut engine = engine(deck_project());
    engine.cover_deck("votes", cover(), 0).unwrap();
    act(&mut engine, Some("yes"), CardAction::Raise, 10);
    assert!(engine.cover_card("votes", None, 20).unwrap().changed);
    assert_eq!(text(&engine, "slot"), "Yes");
    let effect = engine
        .snapshot(20)
        .elements
        .into_iter()
        .find(|element| element.id == "slot")
        .and_then(|element| element.effect)
        .map(|state| state.effect);
    assert_eq!(effect, Some(Effect::Pop));
    act(&mut engine, None, CardAction::Lower, 30);
    act(&mut engine, Some("no"), CardAction::Raise, 40);
    assert_eq!(text(&engine, "slot"), "No", "once uncovered, cards show again");
}

#[test]
fn a_cards_cover_becomes_lasting_without_a_visible_change() {
    let mut engine = engine(deck_project());
    act(&mut engine, Some("yes"), CardAction::Raise, 0);
    engine.cover_card("votes", Some(cover()), 10).unwrap();
    let revision = engine.revision();
    assert!(!engine.cover_deck("votes", cover(), 20).unwrap().changed);
    assert_eq!(engine.revision(), revision);
    act(&mut engine, None, CardAction::Lower, 30);
    assert_eq!(engine.cover("votes"), Some(&cover()), "the cover now lasts");
    engine.cover_card("votes", Some(cover()), 40).unwrap();
    act(&mut engine, Some("yes"), CardAction::Raise, 50);
    act(&mut engine, None, CardAction::Lower, 60);
    assert_eq!(
        engine.cover("votes"),
        None,
        "a card's cover clears when it lowers again"
    );
}

#[test]
fn saves_keep_it_until_the_deck_is_removed() {
    let mut engine = engine(deck_project());
    engine.cover_deck("votes", cover(), 0).unwrap();
    act(&mut engine, Some("yes"), CardAction::Raise, 10);
    let project = engine.project().clone();
    let mut without_card = project.clone();
    without_card.decks[0].cards.retain(|card| card.id != "yes");
    engine.replace_project(without_card, 20).unwrap();
    assert_eq!(
        engine.raised_card("votes"),
        None,
        "the removed card is no longer raised"
    );
    assert_eq!(engine.cover("votes"), Some(&cover()), "the deck stays covered");
    let mut without_deck = project;
    without_deck.decks.clear();
    engine.replace_project(without_deck, 30).unwrap();
    assert_eq!(engine.cover("votes"), None, "a removed deck takes its cover along");
}
