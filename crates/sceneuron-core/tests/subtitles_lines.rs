// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the pure parts of subtitles: wrapping on word boundaries, BCP 47 target codes and
//! their usual case, and whether two codes name the same language.

use sceneuron_core::{MAX_LINE_CHARS, canonical_language_tag, same_language, valid_language_tag, wrap};

#[test]
fn lines_break_between_words_and_never_exceed_the_width() {
    let text = "the quick brown fox jumps over the lazy dog";
    assert_eq!(wrap(text, 15), ["the quick brown", "fox jumps over", "the lazy dog"]);
    assert_eq!(wrap(text, 16), ["the quick brown", "fox jumps over", "the lazy dog"]);
    assert_eq!(wrap(text, 43), [text], "a line of exactly the width");
    assert_eq!(wrap(text, 42), ["the quick brown fox jumps over the lazy", "dog"]);
    let long = "word ".repeat(60);
    assert!(
        wrap(&long, MAX_LINE_CHARS)
            .iter()
            .all(|line| line.chars().count() <= MAX_LINE_CHARS)
    );
}

#[test]
fn spacing_collapses_and_blank_text_has_no_lines() {
    assert_eq!(wrap("  hello \n  there\tfriend ", 11), ["hello there", "friend"]);
    assert!(wrap("", 20).is_empty());
    assert!(wrap(" \n\t ", 20).is_empty());
}

#[test]
fn a_word_longer_than_a_line_breaks_inside_it() {
    assert_eq!(wrap("ab abcdefghij cd", 4), ["ab", "abcd", "efgh", "ij", "cd"]);
    assert_eq!(wrap("abcdefgh", 4), ["abcd", "efgh"]);
    assert_eq!(wrap("ab", 0), ["a", "b"], "a zero width counts as one character");
}

#[test]
fn lines_count_characters_not_bytes() {
    assert_eq!(wrap("año más qué", 7), ["año más", "qué"]);
    assert_eq!(wrap("日本語のテキスト", 4), ["日本語の", "テキスト"]);
}

#[test]
fn targets_are_bcp_47_tags_of_the_usual_shape() {
    for good in ["es", "pt-BR", "zh-Hans", "es-419", "fil", "sr-Latn-RS", "EN-us"] {
        assert!(valid_language_tag(good), "{good}");
    }
    for bad in [
        "",
        "e",
        "espa",
        "es_MX",
        "pt-",
        "-es",
        "es--MX",
        "es-ABCDEFGHI",
        "1a",
        "es MX",
        "ñu",
        "en-US-aaaaaaaa-bbbbbbbb-cccccccc-dddd",
    ] {
        assert!(!valid_language_tag(bad), "{bad}");
    }
}

#[test]
fn targets_take_bcp_47_s_usual_case() {
    assert_eq!(canonical_language_tag("pt-br").as_deref(), Some("pt-BR"));
    assert_eq!(canonical_language_tag(" ZH-hant ").as_deref(), Some("zh-Hant"));
    assert_eq!(canonical_language_tag("ES-419").as_deref(), Some("es-419"));
    assert_eq!(canonical_language_tag("sr-latn-rs").as_deref(), Some("sr-Latn-RS"));
    assert_eq!(canonical_language_tag("pt_br"), None);
}

#[test]
fn the_same_language_ignores_case_regions_and_long_codes() {
    assert!(same_language("en", "en-US"));
    assert!(same_language("EN", "en"));
    assert!(same_language("spa", "es-MX"));
    assert!(same_language("pt-BR", "por"));
    assert!(same_language("cmn", "zh-Hans"));
    assert!(!same_language("en", "es"));
    assert!(!same_language("pt", "es"));
    assert!(!same_language("", "en"), "a detected language matches nothing");
    assert!(!same_language("en", ""));
}
