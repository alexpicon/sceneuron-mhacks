// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises how heard words are compared and how card text is spoken: case, punctuation,
//! apostrophes, emoji, number words, "question mark", the coffee card's words, and placeholders.

use sceneuron_core::{normalize_speech, speakable};

#[test]
fn case_punctuation_and_emoji_do_not_count() {
    assert_eq!(normalize_speech("Sceneuron, WRAP it up!"), "sceneuron wrap it up");
    assert_eq!(normalize_speech("⏱️ Wrap it up"), "wrap it up");
    assert_eq!(normalize_speech("  We're   off-track... "), "were off track");
    assert_eq!(normalize_speech("Let’s take this offline"), "lets take this offline");
    assert_eq!(normalize_speech("Can't hear you"), "cant hear you");
    assert_eq!(normalize_speech("🗂️🔇"), "");
    assert_eq!(normalize_speech(""), "");
}

#[test]
fn number_words_up_to_twenty_one_become_digits() {
    assert_eq!(normalize_speech("my vote is eight"), "my vote is 8");
    assert_eq!(normalize_speech("Zero one two three five"), "0 1 2 3 5");
    assert_eq!(normalize_speech("thirteen"), "13");
    assert_eq!(normalize_speech("twenty one"), "21");
    assert_eq!(normalize_speech("twenty-one"), "21");
    assert_eq!(normalize_speech("twenty"), "20");
    assert_eq!(normalize_speech("I vote 8"), "i vote 8");
    assert_eq!(normalize_speech("forty"), "forty", "only up to twenty-one");
}

#[test]
fn question_marks_and_the_coffee_card_can_be_named() {
    assert_eq!(normalize_speech("vote question mark"), "vote ?");
    assert_eq!(normalize_speech("?"), "?");
    assert_eq!(
        normalize_speech("I have a question?"),
        "i have a question",
        "a trailing ? is punctuation"
    );
    assert_eq!(normalize_speech("☕"), "coffee");
    assert_eq!(normalize_speech("coffee"), "coffee");
    assert_eq!(normalize_speech("break"), "coffee");
    assert_eq!(normalize_speech("coffee break"), "coffee", "repeated words merge");
    assert_eq!(normalize_speech("☕ Back in 5"), "coffee back in 5");
}

#[test]
fn speakable_text_keeps_words_and_punctuation_without_emoji_or_placeholders() {
    assert_eq!(speakable("⏱️ Wrap it up"), "Wrap it up");
    assert_eq!(speakable("🍽️ At lunch"), "At lunch");
    assert_eq!(speakable("✋ Hand raised"), "Hand raised");
    assert_eq!(speakable("❓ Question"), "Question");
    assert_eq!(speakable("👍"), "");
    assert_eq!(speakable("I'm on mute 🔇!"), "I'm on mute !");
    assert_eq!(speakable("Now showing: {{item}} today"), "Now showing: today");
    assert_eq!(speakable("Broken {{ placeholder"), "Broken");
    assert_eq!(speakable("one\ntwo"), "one two");
}
