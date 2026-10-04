// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises Klingon in pIqaD: every letter, digraph, and the trigraph of the romanization at its
//! ConScript code point, the apostrophe, digits and punctuation, known words and phrases, the case
//! that tells q from Q, the case the other letters may lose, and `ngh` against `ngH`.

use sceneuron_core::NativeScript;

fn piqad(text: &str) -> String {
    NativeScript::Piqad.write(text)
}

fn chars(codes: &[u32]) -> String {
    codes.iter().map(|&code| char::from_u32(code).unwrap()).collect()
}

/// The romanization in the order of the ConScript Unicode Registry, from U+F8D0.
const ALPHABET: [&str; 26] = [
    "a", "b", "ch", "D", "e", "gh", "H", "I", "j", "l", "m", "n", "ng", "o", "p", "q", "Q", "r", "S", "t", "tlh", "u",
    "v", "w", "y", "'",
];

#[test]
fn every_letter_has_its_own_code_point() {
    for (offset, letter) in (0..).zip(ALPHABET) {
        assert_eq!(piqad(letter), chars(&[0xF8D0 + offset]), "{letter}");
    }
    assert_eq!(piqad(&ALPHABET.concat()), chars(&(0xF8D0..=0xF8E9).collect::<Vec<_>>()));
}

#[test]
fn digraphs_and_the_trigraph_are_one_letter_each() {
    assert_eq!(piqad("tlhIngan"), chars(&[0xF8E4, 0xF8D7, 0xF8DC, 0xF8D0, 0xF8DB]));
    assert_eq!(piqad("chech"), chars(&[0xF8D2, 0xF8D4, 0xF8D2]));
    assert_eq!(piqad("ghargh"), chars(&[0xF8D5, 0xF8D0, 0xF8E1, 0xF8D5]));
    assert_eq!(piqad("ngoq"), chars(&[0xF8DC, 0xF8DD, 0xF8DF]));
    assert_eq!(piqad("tlhegh"), chars(&[0xF8E4, 0xF8D4, 0xF8D5]));
    assert_eq!(
        piqad("tl"),
        chars(&[0xF8E3, 0xF8D9]),
        "t and l without h are two letters"
    );
}

#[test]
fn known_words_and_phrases() {
    assert_eq!(
        piqad("Qapla'"),
        chars(&[0xF8E0, 0xF8D0, 0xF8DE, 0xF8D9, 0xF8D0, 0xF8E9])
    );
    assert_eq!(
        piqad("nuqneH"),
        chars(&[0xF8DB, 0xF8E5, 0xF8DF, 0xF8DB, 0xF8D4, 0xF8D6])
    );
    assert_eq!(
        piqad("tlhIngan Hol vIjatlh."),
        format!(
            "{} {} {}",
            chars(&[0xF8E4, 0xF8D7, 0xF8DC, 0xF8D0, 0xF8DB]),
            chars(&[0xF8D6, 0xF8DD, 0xF8D9]),
            chars(&[0xF8E6, 0xF8D7, 0xF8D8, 0xF8D0, 0xF8E4, 0xF8FE]),
        ),
    );
    assert_eq!(
        piqad("DaHjaj QaQ jaj"),
        format!(
            "{} {} {}",
            chars(&[0xF8D3, 0xF8D0, 0xF8D6, 0xF8D8, 0xF8D0, 0xF8D8]),
            chars(&[0xF8E0, 0xF8D0, 0xF8E0]),
            chars(&[0xF8D8, 0xF8D0, 0xF8D8]),
        ),
    );
}

#[test]
fn q_keeps_its_case_and_other_letters_read_in_either() {
    assert_ne!(piqad("qaw"), piqad("Qaw"), "remember and destroy are different words");
    assert_eq!(piqad("Nuqneh"), piqad("nuqneH"), "a sentence's capital, and h for H");
    assert_eq!(piqad("tlhingan"), piqad("tlhIngan"));
    assert_eq!(piqad("Tlhingan"), piqad("tlhIngan"));
    assert_eq!(piqad("TLH"), piqad("tlh"));
    assert_eq!(piqad("daHjaj"), piqad("DaHjaj"));
    assert_eq!(piqad("soS"), piqad("SoS"));
}

#[test]
fn ngh_is_n_and_gh_unless_its_h_is_a_capital() {
    assert_eq!(piqad("nghep"), chars(&[0xF8DB, 0xF8D5, 0xF8D4, 0xF8DE]));
    assert_eq!(piqad("ngHa'"), chars(&[0xF8DC, 0xF8D6, 0xF8D0, 0xF8E9]));
}

#[test]
fn digits_apostrophes_and_punctuation() {
    assert_eq!(piqad("0123456789"), chars(&(0xF8F0..=0xF8F9).collect::<Vec<_>>()));
    assert_eq!(piqad("a, e; o: u."), piqad("a\u{F8FD} e\u{F8FD} o\u{F8FD} u\u{F8FE}"));
    assert_eq!(
        piqad("!?"),
        chars(&[0xF8FE, 0xF8FE]),
        "pIqaD ends a sentence the same way"
    );
    for apostrophe in ["'", "\u{2019}", "\u{2018}", "`"] {
        assert_eq!(
            piqad(&format!("ba{apostrophe}")),
            chars(&[0xF8D1, 0xF8D0, 0xF8E9]),
            "{apostrophe}"
        );
    }
    assert_eq!(
        piqad("Kirk"),
        format!("K{}k", chars(&[0xF8D7, 0xF8E1])),
        "letters Klingon lacks stay"
    );
    assert_eq!(piqad("  "), "  ");
    assert_eq!(piqad(""), "");
}
