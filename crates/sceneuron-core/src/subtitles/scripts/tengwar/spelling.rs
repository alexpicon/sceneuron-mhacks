// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Reading a word of the romanization: each character as a vowel with its accent, a consonant, or
//! a letter neither mode has, and what the modes' consonant tables match against.

use super::Mode;
use super::letters::*;

/// A vowel of the romanization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Vowel {
    A,
    E,
    I,
    O,
    U,
    /// Sindarin's front rounded vowel.
    Y,
}

impl Vowel {
    pub(super) fn tehta(self) -> char {
        match self {
            Vowel::A => THREE_DOTS,
            Vowel::E => ACUTE,
            Vowel::I => DOT,
            Vowel::O => RIGHT_CURL,
            Vowel::U => LEFT_CURL,
            Vowel::Y => TWO_DOTS,
        }
    }
}

/// A letter of a word, with its accent read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Letter {
    /// `long` for an acute or a circumflex; `apart` for a diaeresis, which keeps the vowel out of a
    /// diphthong.
    Vowel {
        vowel: Vowel,
        long: bool,
        apart: bool,
    },
    Consonant(char),
    /// A letter neither mode has, kept as it is.
    Other(char),
}

/// A character of a word as a letter in `mode`: vowels with their accents, and in Sindarin `y` as
/// a vowel.
pub(super) fn letter(character: char, mode: Mode) -> Letter {
    let lower = character.to_lowercase().next().unwrap_or(character);
    let (vowel, long, apart) = match lower {
        'a' => (Vowel::A, false, false),
        'e' => (Vowel::E, false, false),
        'i' => (Vowel::I, false, false),
        'o' => (Vowel::O, false, false),
        'u' => (Vowel::U, false, false),
        'y' if mode == Mode::Sindarin => (Vowel::Y, false, false),
        'á' | 'â' => (Vowel::A, true, false),
        'é' | 'ê' => (Vowel::E, true, false),
        'í' | 'î' => (Vowel::I, true, false),
        'ó' | 'ô' => (Vowel::O, true, false),
        'ú' | 'û' => (Vowel::U, true, false),
        'ý' | 'ŷ' => (Vowel::Y, true, false),
        'ä' => (Vowel::A, false, true),
        'ë' => (Vowel::E, false, true),
        'ï' => (Vowel::I, false, true),
        'ö' => (Vowel::O, false, true),
        'ü' => (Vowel::U, false, true),
        'ÿ' => (Vowel::Y, false, true),
        'b'..='z' | 'ñ' => return Letter::Consonant(lower),
        _ => return Letter::Other(character),
    };
    Letter::Vowel { vowel, long, apart }
}

/// The consonant letters `letters` starts with, as a string to match a mode's table against.
pub(super) fn consonants(letters: &[Letter]) -> String {
    letters
        .iter()
        .map_while(|letter| match letter {
            Letter::Consonant(consonant) => Some(*consonant),
            _ => None,
        })
        .collect()
}

/// Whether the letter at `index` is a vowel.
pub(super) fn vowel_at(letters: &[Letter], index: usize) -> bool {
    matches!(letters.get(index), Some(Letter::Vowel { .. }))
}
