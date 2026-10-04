// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the card deck operations of AI edit plans: adding a deck with its card slot,
//! replacing a deck, removing one, and the refusals and validation that keep decks sound.

mod common;

use sceneuron_core::{Deck, EditOp, EditPlan, Project};

fn plan(operations: Vec<EditOp>) -> EditPlan {
    EditPlan {
        summary: "test".to_owned(),
        operations,
    }
}

fn from_json(json: &str) -> EditPlan {
    serde_json::from_str(json).unwrap()
}

fn apply(operations: Vec<EditOp>, project: &Project) -> Result<Project, String> {
    plan(operations).apply(project).map_err(|error| error.0)
}

/// The "Votes" deck of [`common::deck_project`], changed by `change`.
fn votes(change: impl FnOnce(&mut Deck)) -> Deck {
    let mut deck = common::deck_project().decks[0].clone();
    change(&mut deck);
    deck
}

#[test]
fn planner_json_adds_a_card_slot_and_its_deck() {
    let plan = from_json(
        r##"{
        "summary": "Show planning poker estimates",
        "operations": [
            {"op": "add_element", "element": {"id": "poker-slot", "name": "Poker card", "kind": "text",
                "props": {"x": 1560, "y": 600, "width": 300, "height": 420, "visible": false, "enter": "pop"}}},
            {"op": "add_deck", "deck": {"id": "poker", "name": "Planning poker", "element": "poker-slot",
                "auto_hide_ms": 0, "camera": true,
                "cards": [
                    {"id": "p1", "name": "1", "props": {"text": "1"}, "signal": {"fingers": 1}},
                    {"id": "p2", "name": "2", "props": {"text": "2", "background": "#2563ebff"}},
                    {"id": "unsure", "name": "?", "props": {"text": "?"}, "signal": {"sign": "open_palm"}},
                    {"id": "hand", "name": "Hand", "props": {"text": "Hand"}, "signal": "hand_raised"}
                ]}}
        ]}"##,
    );
    let EditOp::AddDeck { deck } = &plan.operations[1] else {
        panic!("the second operation adds a deck");
    };
    let next = plan.apply(&common::deck_project()).unwrap();
    assert_eq!(next.deck("poker"), Some(deck));
    assert_eq!(next.deck_for_element("poker-slot").map(|d| d.cards.len()), Some(4));
    assert_eq!(next.decks.len(), 2, "the existing deck stays");
}

#[test]
fn a_new_deck_needs_a_free_id_and_a_text_element_of_its_own() {
    let base = common::deck_project();
    let duplicate = apply(vec![EditOp::AddDeck { deck: votes(|_| {}) }], &base);
    assert_eq!(duplicate.unwrap_err(), "operation 1: deck \"votes\" already exists");
    let shared = votes(|deck| deck.id = "second".to_owned());
    let error = apply(vec![EditOp::AddDeck { deck: shared }], &base).unwrap_err();
    assert!(error.contains("already shows another deck"), "{error}");
    let framed = votes(|deck| (deck.id, deck.element) = ("framed".to_owned(), "cam-frame".to_owned()));
    let error = apply(vec![EditOp::AddDeck { deck: framed }], &base).unwrap_err();
    assert!(error.contains("needs a text element"), "{error}");
    let dangling = votes(|deck| (deck.id, deck.element) = ("dangling".to_owned(), "nowhere".to_owned()));
    let error = apply(vec![EditOp::AddDeck { deck: dangling }], &base).unwrap_err();
    assert!(error.contains("unknown element"), "{error}");
}

#[test]
fn update_deck_replaces_the_name_cards_and_auto_hide() {
    let base = common::deck_project();
    let deck = votes(|deck| {
        deck.name = "Quick votes".to_owned();
        deck.cards.pop();
        deck.cards[0].props.text = Some("Aye".to_owned());
        deck.auto_hide_ms = 15_000;
    });
    let next = apply(vec![EditOp::UpdateDeck { deck: deck.clone() }], &base).unwrap();
    assert_eq!(next.decks, vec![deck]);
    assert_eq!(next.elements, base.elements, "the card slot itself is unchanged");
}

#[test]
fn update_deck_replaces_the_camera_switch_and_card_signals() {
    let plan = from_json(
        r#"{"summary": "Raise Yes with a thumbs up", "operations": [
            {"op": "update_deck", "deck": {"id": "votes", "name": "Votes", "element": "slot", "camera": true,
                "cards": [{"id": "yes", "name": "Yes", "props": {"text": "Yes"}, "signal": {"sign": "thumbs_up"}}]}}
        ]}"#,
    );
    let EditOp::UpdateDeck { deck } = &plan.operations[0] else {
        panic!("the operation updates a deck");
    };
    let next = plan.apply(&common::deck_project()).unwrap();
    assert_eq!(next.decks, vec![deck.clone()]);
    assert_eq!(next.decks[0].auto_hide_ms, 0, "a left-out delay takes its default");
}

#[test]
fn update_deck_keeps_the_element_and_needs_a_known_deck() {
    let base = common::deck_project();
    let moved = votes(|deck| deck.element = "name-label".to_owned());
    let error = apply(vec![EditOp::UpdateDeck { deck: moved }], &base).unwrap_err();
    assert_eq!(error, "operation 1: deck \"votes\" must keep showing element \"slot\"");
    let unknown = votes(|deck| deck.id = "ghost".to_owned());
    let error = apply(vec![EditOp::UpdateDeck { deck: unknown }], &base).unwrap_err();
    assert_eq!(error, "operation 1: unknown deck \"ghost\"");
    let empty = votes(|deck| deck.cards.clear());
    let error = apply(vec![EditOp::UpdateDeck { deck: empty }], &base).unwrap_err();
    assert!(error.contains("needs at least one card"), "{error}");
    let shown = votes(|deck| deck.cards[1].props.visible = Some(true));
    let error = apply(vec![EditOp::UpdateDeck { deck: shown }], &base).unwrap_err();
    assert!(error.contains("cards must not set visible"), "{error}");
    assert_eq!(base, common::deck_project(), "a refused plan leaves the project alone");
}

#[test]
fn remove_deck_keeps_its_element_and_needs_a_known_deck() {
    let base = common::deck_project();
    let next = apply(vec![EditOp::RemoveDeck { id: "votes".to_owned() }], &base).unwrap();
    assert!(next.decks.is_empty());
    assert_eq!(next.elements, base.elements);
    let error = apply(vec![EditOp::RemoveDeck { id: "votes".to_owned() }], &next).unwrap_err();
    assert_eq!(error, "operation 1: unknown deck \"votes\"");
}

#[test]
fn a_deck_can_move_to_another_element_by_removing_and_adding_it() {
    let base = common::deck_project();
    let mut slot = common::text_element("big-slot", "Big slot");
    slot.props.font_size = 120.0;
    let moved = votes(|deck| deck.element = "big-slot".to_owned());
    let next = apply(
        vec![
            EditOp::AddElement { element: slot },
            EditOp::RemoveDeck { id: "votes".to_owned() },
            EditOp::AddDeck { deck: moved.clone() },
            EditOp::RemoveElement { id: "slot".to_owned() },
        ],
        &base,
    )
    .unwrap();
    assert_eq!(next.decks, vec![moved]);
    assert!(next.element("slot").is_none());
}

#[test]
fn deck_operations_use_snake_case_names() {
    let json = serde_json::to_value(plan(vec![
        EditOp::AddDeck { deck: votes(|_| {}) },
        EditOp::UpdateDeck { deck: votes(|_| {}) },
        EditOp::RemoveDeck { id: "votes".to_owned() },
    ]))
    .unwrap();
    let names: Vec<&str> = json["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|op| op["op"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["add_deck", "update_deck", "remove_deck"]);
    assert_eq!(json["operations"][0]["deck"]["element"], "slot");
}
