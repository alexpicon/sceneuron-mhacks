// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the deck model: its JSON shape, lookups, and every validation rule for decks and
//! their cards, including their camera signals.

mod common;

use common::{card, deck_project, text_element};
use sceneuron_core::{CardAction, CardSignal, Deck, HandSign, Project, RaisedBy};
use serde_json::json;

fn refusal(edit: impl FnOnce(&mut Project)) -> String {
    let mut project = deck_project();
    edit(&mut project);
    project.validate().unwrap_err().0
}

#[test]
fn decks_use_documented_json_names_and_defaults() {
    let project = deck_project();
    let json = serde_json::to_value(&project.decks[0]).unwrap();
    assert_eq!(
        json,
        json!({
            "id": "votes", "name": "Votes", "element": "slot", "auto_hide_ms": 0, "camera": false,
            "voice": false, "speak": false,
            "cards": [
                {"id": "yes", "name": "Yes", "props": {"text": "Yes", "background": "#16a34aff"}},
                {"id": "no", "name": "No", "props": {"text": "No", "background": "#dc2626ff"}},
                {"id": "maybe", "name": "Maybe", "props": {"text": "Maybe", "background": "#2563ebff"}}
            ]
        })
    );
    let short: Deck = serde_json::from_value(json!({
        "id": "d", "name": "D", "element": "slot", "cards": [{"id": "c", "name": "C"}]
    }))
    .unwrap();
    assert_eq!(
        (short.auto_hide_ms, short.camera, short.voice, short.speak),
        (0, false, false, false)
    );
    assert!(short.cards[0].props.is_empty());
    assert_eq!(short.cards[0].signal, None);
    assert!(short.cards[0].phrases.is_empty() && short.cards[0].speech.is_none());
    let typo = json!({"id": "d", "name": "D", "element": "slot", "cards": [{"id": "c", "name": "C",
        "props": {"txt": "x"}}]});
    assert!(serde_json::from_value::<Deck>(typo).is_err());
}

#[test]
fn raised_by_and_card_actions_are_snake_case_words() {
    assert_eq!(serde_json::to_value(RaisedBy::Manual).unwrap(), "manual");
    assert_eq!(serde_json::to_value(RaisedBy::Camera).unwrap(), "camera");
    assert_eq!(serde_json::to_value(RaisedBy::Voice).unwrap(), "voice");
    let actions = [
        (CardAction::Raise, "raise"),
        (CardAction::Toggle, "toggle"),
        (CardAction::Lower, "lower"),
        (CardAction::Next, "next"),
        (CardAction::Previous, "previous"),
    ];
    for (action, word) in actions {
        assert_eq!(serde_json::to_value(action).unwrap(), word);
        assert_eq!(serde_json::from_value::<CardAction>(json!(word)).unwrap(), action);
    }
    assert!(serde_json::from_value::<CardAction>(json!("flip")).is_err());
}

#[test]
fn projects_find_decks_by_id_and_by_element() {
    let project = deck_project();
    project.validate().unwrap();
    assert_eq!(project.deck("votes").unwrap().name, "Votes");
    assert_eq!(project.deck_for_element("slot").unwrap().id, "votes");
    assert!(project.deck("nope").is_none() && project.deck_for_element("brb").is_none());
    assert_eq!(project.decks[0].card("no").unwrap().name, "No");
    assert!(project.decks[0].card("nope").is_none());
}

#[test]
fn allows_up_to_sixteen_decks() {
    let sixteen = |count: usize| {
        let mut project = deck_project();
        for n in 1..count {
            let id = format!("slot-{n}");
            project.elements.push(text_element(&id, "Another slot"));
            let mut deck = project.decks[0].clone();
            deck.id = format!("votes-{n}");
            deck.element = id;
            project.decks.push(deck);
        }
        project.validate()
    };
    sixteen(16).unwrap();
    assert!(sixteen(17).unwrap_err().0.contains("at most 16 decks"));
}

#[test]
fn decks_hold_one_to_twenty_four_cards() {
    assert!(refusal(|p| p.decks[0].cards.clear()).contains("needs at least one card"));
    let cards = |count: usize| {
        (0..count)
            .map(|n| card(&format!("c{n}"), &format!("Card {n}"), "#000"))
            .collect::<Vec<_>>()
    };
    let mut project = deck_project();
    project.decks[0].cards = cards(24);
    project.validate().unwrap();
    assert!(refusal(|p| p.decks[0].cards = cards(25)).contains("at most 24 cards per deck"));
}

#[test]
fn rejects_bad_and_duplicate_deck_ids_and_names() {
    assert!(refusal(|p| p.decks[0].id = "Votes".into()).contains("must be 1-40 characters"));
    assert!(refusal(|p| p.decks[0].name = String::new()).contains("printable characters"));
    let duplicate = refusal(|p| {
        p.elements.push(text_element("slot-2", "Second slot"));
        let mut deck = p.decks[0].clone();
        deck.element = "slot-2".into();
        p.decks.push(deck);
    });
    assert!(duplicate.contains("duplicate deck id \"votes\""), "{duplicate}");
}

#[test]
fn the_card_slot_is_one_existing_text_element_per_deck() {
    let unknown = refusal(|p| p.decks[0].element = "ghost".into());
    assert!(
        unknown.contains("deck \"votes\" uses unknown element \"ghost\""),
        "{unknown}"
    );
    let frame = refusal(|p| p.decks[0].element = "cam-frame".into());
    assert!(frame.contains("needs a text element"), "{frame}");
    let shared = refusal(|p| {
        let mut deck = p.decks[0].clone();
        deck.id = "other".into();
        p.decks.push(deck);
    });
    assert!(
        shared.contains("element \"slot\" already shows another deck"),
        "{shared}"
    );
}

#[test]
fn rejects_bad_and_duplicate_cards() {
    assert!(refusal(|p| p.decks[0].cards[0].id = "has space".into()).contains("must be 1-40"));
    assert!(refusal(|p| p.decks[0].cards[0].name = "a\tb".into()).contains("printable"));
    let duplicate = refusal(|p| p.decks[0].cards[1].id = "yes".into());
    assert!(
        duplicate.contains("deck \"votes\" has duplicate card id \"yes\""),
        "{duplicate}"
    );
    let color = refusal(|p| p.decks[0].cards[1].props.color = Some("red".into()));
    assert!(color.starts_with("votes: no: color must be a hex color"), "{color}");
    let visible = refusal(|p| p.decks[0].cards[2].props.visible = Some(true));
    assert!(
        visible.starts_with("votes: maybe: cards must not set visible"),
        "{visible}"
    );
}

#[test]
fn card_ids_only_need_to_be_unique_within_their_deck() {
    let mut project = deck_project();
    project.elements.push(text_element("slot-2", "Second slot"));
    let mut deck = project.decks[0].clone();
    deck.id = "votes-2".into();
    deck.element = "slot-2".into();
    project.decks.push(deck);
    project.validate().unwrap();
}

#[test]
fn auto_hide_is_off_or_one_second_to_one_hour() {
    for ok in [0, 1_000, 15_000, 3_600_000] {
        let mut project = deck_project();
        project.decks[0].auto_hide_ms = ok;
        project.validate().unwrap();
    }
    for bad in [1, 999, 3_600_001] {
        let error = refusal(|p| p.decks[0].auto_hide_ms = bad);
        assert_eq!(error, "votes: auto_hide_ms must be 0 or 1000 to 3600000");
    }
}

#[test]
fn card_signals_ask_for_at_most_ten_fingers() {
    for ok in [
        CardSignal::Fingers(0),
        CardSignal::Fingers(10),
        CardSignal::HandRaised,
        CardSignal::Sign(HandSign::Fist),
    ] {
        let mut project = deck_project();
        project.decks[0].camera = true;
        project.decks[0].cards[1].signal = Some(ok);
        project.validate().unwrap();
    }
    let error = refusal(|p| p.decks[0].cards[1].signal = Some(CardSignal::Fingers(11)));
    assert_eq!(error, "votes: no: signal fingers must be 0 to 10");
}
