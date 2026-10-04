// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the packs' camera signals: which cards have one, that no deck gives two cards the
//! same signal or uses an open palm, that every camera starts off, that the meeting signs raise
//! their cards, and that a project saved before a pack gained signals keeps its own.

mod common;

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use common::camera::{show, sign};
use common::engine;
use sceneuron_core::{CardSignal, HandSign, Pack, default_project, load_project, pack_for_canvas, packs, save_project};

/// The packs that bring a deck of cards.
fn card_packs() -> Vec<Pack> {
    packs().into_iter().filter(|pack| !pack.decks.is_empty()).collect()
}

#[test]
fn some_cards_carry_camera_signals_and_every_camera_starts_off() {
    let mut signals = Vec::new();
    for pack in card_packs() {
        let deck = &pack.decks[0];
        assert!(!deck.camera, "{}", deck.id);
        for card in &deck.cards {
            if let Some(signal) = card.signal {
                signals.push((format!("{}.{}", deck.id, card.id), signal));
            }
        }
    }
    let sign = CardSignal::Sign;
    let expected = [
        ("meeting.wrap-up", sign(HandSign::Fist)),
        ("meeting.question", sign(HandSign::Pointing)),
        ("meeting.agree", sign(HandSign::ThumbsUp)),
        ("meeting.disagree", sign(HandSign::ThumbsDown)),
        ("meeting.next-topic", sign(HandSign::Peace)),
        ("poker.p0", CardSignal::Fingers(0)),
        ("poker.p1", CardSignal::Fingers(1)),
        ("poker.p2", CardSignal::Fingers(2)),
        ("poker.p3", CardSignal::Fingers(3)),
        ("poker.p5", CardSignal::Fingers(5)),
        ("poker.p8", CardSignal::Fingers(8)),
        ("classroom.hand", CardSignal::HandRaised),
        ("classroom.got-it", sign(HandSign::ThumbsUp)),
    ];
    let expected: Vec<_> = expected
        .iter()
        .map(|(card, signal)| (card.to_string(), *signal))
        .collect();
    assert_eq!(signals, expected);
}

#[test]
fn no_deck_gives_two_cards_the_same_signal_or_uses_an_open_palm() {
    for pack in card_packs() {
        let deck = &pack.decks[0];
        let signals: Vec<CardSignal> = deck.cards.iter().filter_map(|card| card.signal).collect();
        let distinct: BTreeSet<String> = signals.iter().map(|signal| format!("{signal:?}")).collect();
        assert_eq!(distinct.len(), signals.len(), "{}", deck.id);
        assert!(
            !signals.contains(&CardSignal::Sign(HandSign::OpenPalm)),
            "{}: hands open while talking",
            deck.id
        );
    }
}

#[test]
fn packs_validate_with_their_camera_on() {
    let mut project = default_project();
    for pack in packs() {
        project = project.add_pack(&pack).unwrap();
    }
    for deck in &mut project.decks {
        deck.camera = true;
    }
    project.validate().unwrap();
}

#[test]
fn each_meeting_sign_raises_its_card_and_an_open_palm_raises_none() {
    let mut project = default_project()
        .add_pack(&pack_for_canvas("meeting", &default_project().canvas).unwrap())
        .unwrap();
    project.decks[0].camera = true;
    let cases = [
        (HandSign::Fist, Some("wrap-up")),
        (HandSign::Pointing, Some("question")),
        (HandSign::ThumbsUp, Some("agree")),
        (HandSign::ThumbsDown, Some("disagree")),
        (HandSign::Peace, Some("next-topic")),
        (HandSign::OpenPalm, None),
    ];
    for (shown, card) in cases {
        let mut engine = engine(project.clone());
        show(&mut engine, &sign(shown), 0, 700);
        assert_eq!(engine.raised_card("meeting"), card, "{shown:?}");
    }
}

#[test]
fn a_project_saved_before_the_meeting_signs_keeps_its_own_signals() {
    let mut saved = default_project()
        .add_pack(&pack_for_canvas("meeting", &default_project().canvas).unwrap())
        .unwrap();
    let older = ["wrap-up", "question", "next-topic"];
    for card in &mut saved.decks[0].cards {
        if older.contains(&card.id.as_str()) {
            card.signal = None;
        }
    }
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("pack-signals-older-project");
    let _ = fs::remove_dir_all(&dir);
    let path = dir.join("project.json");
    save_project(&path, &saved).unwrap();
    let loaded = load_project(&path).unwrap().unwrap();
    assert_eq!(loaded, saved, "loading never adds a pack's newer signals");
    let mut engine = engine(loaded);
    let mut project = engine.project().clone();
    project.decks[0].camera = true;
    engine.replace_project(project, 0).unwrap();
    show(&mut engine, &sign(HandSign::Fist), 0, 700);
    assert_eq!(
        engine.raised_card("meeting"),
        None,
        "a fist raises nothing in the older deck"
    );
    show(&mut engine, &sign(HandSign::ThumbsUp), 800, 1_500);
    assert_eq!(engine.raised_card("meeting"), Some("agree"));
    let fresh = default_project()
        .add_pack(&pack_for_canvas("meeting", &default_project().canvas).unwrap())
        .unwrap();
    let wrap_up = fresh.decks[0].card("wrap-up").unwrap();
    assert_eq!(
        wrap_up.signal,
        Some(CardSignal::Sign(HandSign::Fist)),
        "a pack added now has them"
    );
    fs::remove_dir_all(&dir).unwrap();
}
