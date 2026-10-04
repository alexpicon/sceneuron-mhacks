// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises keyterms: the wake word first, cleaned and cut to Scribe's limits, without repeats.

use sceneuron_voice::{MAX_KEYTERM_CHARS, MAX_KEYTERMS, keyterms};

#[test]
fn the_wake_word_comes_first_and_terms_keep_their_order() {
    let terms = keyterms("Sceneuron", ["Wrap it up", "Next topic", "Agree"]);
    assert_eq!(terms, vec!["Sceneuron", "Wrap it up", "Next topic", "Agree"]);
}

#[test]
fn emoji_and_punctuation_go_while_apostrophes_and_hyphens_stay() {
    let terms = keyterms("", ["⏱️ Wrap it up!", "We\u{2019}re off-track?", "☕  Coffee"]);
    assert_eq!(terms, vec!["Wrap it up", "We're off-track", "Coffee"]);
}

#[test]
fn long_terms_are_cut_after_the_last_whole_word_that_fits() {
    let terms = keyterms(
        "",
        [
            "Let's take this offline",
            "I have a question about it",
            "Supercalifragilisticexpialidocious",
        ],
    );
    assert_eq!(terms, vec!["Let's take this", "I have a question"]);
    assert!(terms.iter().all(|term| term.chars().count() <= MAX_KEYTERM_CHARS));
    assert_eq!(keyterms("", ["exactly twenty chars"]), vec!["exactly twenty chars"]);
}

#[test]
fn single_characters_and_repeats_are_left_out() {
    let terms = keyterms("Sceneuron", ["8", "?", "13", "sceneuron", "Agree", "AGREE", ""]);
    assert_eq!(terms, vec!["Sceneuron", "13", "Agree"]);
}

#[test]
fn at_most_fifty_terms_are_kept() {
    let names: Vec<String> = (0..80).map(|n| format!("card {n}")).collect();
    let terms = keyterms("Sceneuron", names.iter().map(String::as_str));
    assert_eq!(terms.len(), MAX_KEYTERMS);
    assert_eq!(terms[0], "Sceneuron");
    assert_eq!(terms[49], "card 48");
}

#[test]
fn an_empty_wake_word_leaves_only_the_terms() {
    assert_eq!(keyterms("", ["Agree"]), vec!["Agree"]);
    assert!(keyterms("  ", std::iter::empty()).is_empty());
}
