// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises what the voice matcher refuses or leaves to the AI: weak matches, which go to the AI
//! unless they say nothing meaningful, ambiguous matches, a card named after a negation, a number
//! or a one-word name inside an ordinary sentence, and decks whose voice is off.

mod common;

use common::voice::{ask, decks, held, raise, reason, said, unaddressed};
use sceneuron_core::Intent;

#[test]
fn weak_matches_go_to_the_ai_and_ambiguous_ones_raise_nothing() {
    let meeting = decks(&["meeting"]);
    assert_eq!(
        said("Sceneuron, order a pizza", &meeting),
        ask("Sceneuron, order a pizza")
    );
    assert_eq!(reason(held("the", &meeting)), "no card matched \"the\"");
    assert_eq!(
        reason(held("um", &meeting)),
        "no card matched \"um\"",
        "noise asks nothing"
    );
    let both = decks(&["meeting", "classroom"]);
    assert_eq!(
        said("Sceneuron, question", &both),
        raise("classroom", "question"),
        "the exact name wins"
    );
    assert_eq!(
        said("Sceneuron, I have a question", &both),
        raise("meeting", "question")
    );
    assert_eq!(said("Sceneuron, b", &both), raise("classroom", "answer-b"));
    let twice = [decks(&["meeting"]), decks(&["meeting"])].concat();
    let refused = reason(said("Sceneuron wrap it up", &twice));
    assert_eq!(
        refused,
        "\"wrap it up\" could be Meeting signals: Wrap it up or Meeting signals: Wrap it up"
    );
}

#[test]
fn a_negation_raises_no_card_and_goes_to_the_ai() {
    let meeting = decks(&["meeting"]);
    for heard in [
        "Sceneuron, I don't agree",
        "Sceneuron, I do not agree",
        "Sceneuron, never wrap it up",
        "Sceneuron, we can't wrap it up yet",
        "Sceneuron, don't ship it",
    ] {
        assert_eq!(said(heard, &meeting), ask(heard));
    }
    assert_eq!(
        said("Sceneuron, disagree", &meeting),
        raise("meeting", "disagree"),
        "a card may still be named by its own words"
    );
}

#[test]
fn a_number_inside_an_ordinary_sentence_votes_for_nothing_and_goes_to_the_ai() {
    let poker = decks(&["poker"]);
    for heard in [
        "Sceneuron, what is 2 plus 2",
        "Sceneuron, give me one second",
        "Sceneuron, give me five",
        "Sceneuron, show 13",
    ] {
        assert_eq!(said(heard, &poker), ask(heard));
    }
    assert_eq!(said("Sceneuron, five", &poker), raise("poker", "p5"), "a number alone");
    assert_eq!(held("13", &poker), raise("poker", "p13"));
    assert_eq!(said("Sceneuron, vote five", &poker), raise("poker", "p5"), "a vote");
}

#[test]
fn a_one_word_name_inside_a_longer_sentence_raises_nothing_and_goes_to_the_ai() {
    let meeting = decks(&["meeting"]);
    for heard in [
        "Sceneuron, tell everyone that I agree with the plan",
        "Sceneuron, I disagree with the last two points entirely",
    ] {
        assert_eq!(said(heard, &meeting), ask(heard));
    }
    assert_eq!(
        said("Sceneuron, I agree", &meeting),
        raise("meeting", "agree"),
        "the name is most of what was said"
    );
}

#[test]
fn decks_whose_voice_is_off_take_no_part() {
    let mut quiet = decks(&["meeting", "poker"]);
    quiet[1].voice = false;
    assert_eq!(
        reason(said("Sceneuron, vote eight", &quiet)),
        "no card for the vote \"8\""
    );
    assert_eq!(
        said("Sceneuron, lower", &quiet),
        Intent::Lower {
            decks: vec!["meeting".to_owned()]
        }
    );
    quiet[0].voice = false;
    assert_eq!(said("Sceneuron, lower", &quiet), Intent::Lower { decks: Vec::new() });
    assert_eq!(said("let's take this offline", &quiet), unaddressed());
}
