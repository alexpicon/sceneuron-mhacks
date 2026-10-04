// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Klingon in pIqaD: each letter of Okrand's romanization, `a b ch D e gh H I j l m n ng o p q Q
//! r S t tlh u v w y '`, is one pIqaD letter at U+F8D0 to U+F8E9, digits are U+F8F0 to U+F8F9,
//! and the comma and full stop have their own signs.
//!
//! The romanization tells `q` from `Q` by case alone, so those keep their case. Every other letter
//! has one case, so a capital at the start of a sentence, or `d`, `h`, `i`, and `s` written in
//! lowercase, still reads as that letter. A lowercase `h` belongs to `ch`, `gh`, or `tlh`, so `ngh`
//! is `n` and `gh`, while `ngH` is `ng` and `H`.

/// Letters by their romanization, digraphs and the trigraph first, so the longest match wins.
const LETTERS: &[(&str, char)] = &[
    ("tlh", '\u{F8E4}'),
    ("ch", '\u{F8D2}'),
    ("gh", '\u{F8D5}'),
    ("ng", '\u{F8DC}'),
    ("a", '\u{F8D0}'),
    ("b", '\u{F8D1}'),
    ("D", '\u{F8D3}'),
    ("e", '\u{F8D4}'),
    ("H", '\u{F8D6}'),
    ("I", '\u{F8D7}'),
    ("j", '\u{F8D8}'),
    ("l", '\u{F8D9}'),
    ("m", '\u{F8DA}'),
    ("n", '\u{F8DB}'),
    ("o", '\u{F8DD}'),
    ("p", '\u{F8DE}'),
    ("q", '\u{F8DF}'),
    ("Q", '\u{F8E0}'),
    ("r", '\u{F8E1}'),
    ("S", '\u{F8E2}'),
    ("t", '\u{F8E3}'),
    ("u", '\u{F8E5}'),
    ("v", '\u{F8E6}'),
    ("w", '\u{F8E7}'),
    ("y", '\u{F8E8}'),
];
const GLOTTAL_STOP: char = '\u{F8E9}';
const DIGIT_ZERO: u32 = 0xF8F0;
const COMMA: char = '\u{F8FD}';
const FULL_STOP: char = '\u{F8FE}';

/// `text` in pIqaD.
pub(super) fn write(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() * 3);
    let mut at = 0;
    while at < chars.len() {
        let (written, used) = letter(&chars[at..])
            .or_else(|| sign(chars[at]).map(|sign| (sign, 1)))
            .unwrap_or((chars[at], 1));
        out.push(written);
        at += used;
    }
    out
}

/// The pIqaD letter `rest` starts with, and how many characters it takes; `None` when `rest` does
/// not start with a letter of the romanization.
fn letter(rest: &[char]) -> Option<(char, usize)> {
    // A lowercase h after ng belongs to gh, as no other letter has a lowercase h.
    if starts(rest, "ngh") && rest[2] == 'h' {
        return Some(('\u{F8DB}', 1));
    }
    LETTERS
        .iter()
        .find(|(latin, _)| starts(rest, latin))
        .map(|(latin, piqad)| (*piqad, latin.chars().count()))
}

/// Whether `rest` starts with the romanized `latin`: digraphs and `tlh` in any case, `q` and `Q` in
/// their own case, and any other letter in either case.
fn starts(rest: &[char], latin: &str) -> bool {
    let wanted: Vec<char> = latin.chars().collect();
    if rest.len() < wanted.len() {
        return false;
    }
    match latin {
        "q" | "Q" => rest[0] == wanted[0],
        _ => rest
            .iter()
            .zip(&wanted)
            .all(|(have, want)| have.eq_ignore_ascii_case(want)),
    }
}

/// The pIqaD sign for a digit, an apostrophe, or a pause; `None` for anything else.
fn sign(character: char) -> Option<char> {
    match character {
        '\'' | '\u{2019}' | '\u{2018}' | '`' => Some(GLOTTAL_STOP),
        '0'..='9' => char::from_u32(DIGIT_ZERO + u32::from(character) - u32::from('0')),
        ',' | ';' | ':' => Some(COMMA),
        '.' | '!' | '?' => Some(FULL_STOP),
        _ => None,
    }
}
