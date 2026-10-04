// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Tengwar in the two modes of Tolkien's own writing for his Elvish languages, as Appendix E of
//! The Lord of the Rings and his samples describe them. Each word is spelled into sounds by the
//! mode's consonant table, then its vowels become tehtar: in the classical mode for Quenya over the
//! consonant before the vowel, in the general mode for Sindarin over the consonant after it, and on
//! a carrier where there is no such consonant. A long vowel sits on the long carrier, and the first
//! vowel of a diphthong on yanta or úrë, which stand for its glide. Silmë and essë turn over to
//! make room for a tehta.
//!
//! In the encoding a tehta follows the tengwa it sits on, whatever the order it is read in. Words
//! are lowercased, as Tengwar has no capitals; apostrophes and hyphens inside words drop out, and
//! punctuation becomes the pusta, the double pusta, and Tengwar's own exclamation and question
//! marks. Digits stay as they are.

mod letters;
mod quenya;
mod sindarin;
mod spelling;

use letters::*;
use spelling::{Letter, Vowel, consonants, letter, vowel_at};

/// A mode of writing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mode {
    Quenya,
    Sindarin,
}

/// A tengwa and the marks that change its consonant, such as the bar of a doubled one.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Tengwa {
    letter: char,
    marks: Vec<char>,
}

impl Tengwa {
    fn new(letter: char) -> Tengwa {
        Tengwa {
            letter,
            marks: Vec::new(),
        }
    }

    fn marked(letter: char, mark: char) -> Tengwa {
        Tengwa {
            letter,
            marks: vec![mark],
        }
    }
}

/// A sound of a word, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Sound {
    Consonant(Tengwa),
    Vowel {
        vowel: Vowel,
        long: bool,
    },
    /// The vowel's tehta on `glide`, yanta or úrë.
    Diphthong {
        vowel: Vowel,
        glide: char,
    },
    Other(char),
}

/// `text` in Tengwar, in `mode`.
pub(super) fn write(text: &str, mode: Mode) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() * 3);
    let mut at = 0;
    while at < chars.len() {
        let character = chars[at];
        if character.is_alphabetic() {
            let end = (at..chars.len())
                .find(|&index| !chars[index].is_alphabetic())
                .unwrap_or(chars.len());
            out.push_str(&word(&chars[at..end], mode));
            at = end;
            continue;
        }
        let letter_before = at > 0 && chars[at - 1].is_alphabetic();
        let letter_after = chars.get(at + 1).is_some_and(|next| next.is_alphabetic());
        let joins = match character {
            '\'' | '\u{2019}' => letter_before || letter_after,
            '-' => letter_before && letter_after,
            _ => false,
        };
        if !joins {
            out.push(punctuation(character));
        }
        at += 1;
    }
    out
}

/// A word in `mode`.
fn word(chars: &[char], mode: Mode) -> String {
    let letters: Vec<Letter> = chars.iter().map(|&character| letter(character, mode)).collect();
    let sounds = match mode {
        Mode::Quenya => quenya::sounds(&letters),
        Mode::Sindarin => sindarin::sounds(&letters),
    };
    let mut written: Vec<(Tengwa, Option<char>)> = Vec::new();
    // A vowel waiting for the consonant after it, in the general mode.
    let mut waiting: Option<Vowel> = None;
    // Whether the last tengwa written is a consonant that can still take the vowel after it.
    let mut open = false;
    let mut out = String::new();
    for (index, sound) in sounds.iter().enumerate() {
        match sound {
            Sound::Consonant(tengwa) => {
                written.push((tengwa.clone(), waiting.take().map(Vowel::tehta)));
                open = true;
            }
            Sound::Vowel { vowel, long: false } => {
                let next_consonant = matches!(sounds.get(index + 1), Some(Sound::Consonant(_)));
                match mode {
                    Mode::Quenya if open && written.last().is_some_and(|(_, tehta)| tehta.is_none()) => {
                        if let Some((_, tehta)) = written.last_mut() {
                            *tehta = Some(vowel.tehta());
                        }
                    }
                    Mode::Sindarin if next_consonant => waiting = Some(*vowel),
                    _ => written.push((Tengwa::new(SHORT_CARRIER), Some(vowel.tehta()))),
                }
                open = false;
            }
            Sound::Vowel { vowel, long: true } => {
                written.push((Tengwa::new(LONG_CARRIER), Some(vowel.tehta())));
                open = false;
            }
            Sound::Diphthong { vowel, glide } => {
                written.push((Tengwa::new(*glide), Some(vowel.tehta())));
                open = false;
            }
            Sound::Other(character) => {
                flush(&mut written, &mut out);
                out.push(*character);
                open = false;
            }
        }
    }
    flush(&mut written, &mut out);
    out
}

/// Writes out the tengwar so far, each with its marks and then its tehta; silmë and essë under a
/// tehta turn over.
fn flush(written: &mut Vec<(Tengwa, Option<char>)>, out: &mut String) {
    for (tengwa, tehta) in written.drain(..) {
        let letter = match (tengwa.letter, tehta) {
            (SILME, Some(_)) => SILME_NUQUERNA,
            (ARE, Some(_)) => ARE_NUQUERNA,
            (letter, _) => letter,
        };
        out.push(letter);
        out.extend(tengwa.marks);
        out.extend(tehta);
    }
}

/// Tengwar punctuation for a pause or a sentence's end; anything else stays as it is.
fn punctuation(character: char) -> char {
    match character {
        ',' => PUSTA,
        '.' | ';' | ':' => DOUBLE_PUSTA,
        '!' => EXCLAMATION_MARK,
        '?' => QUESTION_MARK,
        '\u{201C}' => LEFT_QUOTATION_MARK,
        '\u{201D}' => RIGHT_QUOTATION_MARK,
        other => other,
    }
}
