// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the `{{room}}` placeholder: the summary the shared room client sets, rendered in
//! element and card text, on the deck whose round is open, and changes that publish only when the
//! summary changes.

mod common;

use common::{deck_project, engine, text};
use sceneuron_core::{CardAction, Project, PropsPatch};

fn summary_project() -> Project {
    let mut project = deck_project();
    project.elements.push(common::text_element("summary", "Room summary"));
    project.elements.last_mut().unwrap().props.text = "Room: {{ room }}".to_owned();
    project
}

#[test]
fn the_room_placeholder_shows_the_summary() {
    let mut engine = engine(summary_project());
    assert_eq!(text(&engine, "summary"), "Room: ", "no summary before joining a room");
    let revision = engine.revision();
    assert!(engine.set_room_summary("Voted: 1 of 2").changed);
    assert_eq!(engine.revision(), revision + 1);
    assert_eq!(text(&engine, "summary"), "Room: Voted: 1 of 2");
    assert!(engine.set_room_summary("3, 5").changed);
    assert_eq!(text(&engine, "summary"), "Room: 3, 5");
}

#[test]
fn setting_the_same_summary_changes_nothing() {
    let mut engine = engine(summary_project());
    engine.set_room_summary("Voted: 0 of 1");
    let revision = engine.revision();
    assert!(!engine.set_room_summary("Voted: 0 of 1").changed);
    assert_eq!(engine.revision(), revision);
}

#[test]
fn cards_can_show_the_summary_too() {
    let mut project = deck_project();
    project.decks[0].cards[0].props = PropsPatch {
        text: Some("Spread: {{room}}".to_owned()),
        ..PropsPatch::default()
    };
    let mut engine = engine(project);
    engine.set_room_summary("2, 3");
    engine.card("votes", Some("yes"), CardAction::Raise, 10).unwrap();
    assert_eq!(text(&engine, "slot"), "Spread: 2, 3");
}

#[test]
fn the_rooms_deck_carries_the_summary_through_the_reveal() {
    let mut engine = engine(summary_project());
    let summary = |engine: &sceneuron_core::Engine| engine.snapshot(0).decks[0].summary.clone();
    engine.set_room_summary("Voted: 3 of 5");
    assert_eq!(summary(&engine), None, "no round is open for the deck");
    engine.set_room_round(Some("votes"));
    assert_eq!(summary(&engine).as_deref(), Some("Voted: 3 of 5"));
    engine.set_room_summary("");
    assert_eq!(summary(&engine), None, "an empty summary says nothing");
    engine.set_room_summary("Maya 3 · You 5 · Average 4");
    engine.set_room_round(None);
    assert_eq!(
        summary(&engine).as_deref(),
        Some("Maya 3 · You 5 · Average 4"),
        "the revealed votes still show once the reveal ends the round"
    );
    engine.set_room_summary("");
    assert_eq!(summary(&engine), None, "leaving the room empties it");
}
