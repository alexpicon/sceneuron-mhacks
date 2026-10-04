// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The general mode for Sindarin, the "general use" of Appendix E: the third series is unused and
//! the fourth is velar, `c`, `g`, `ch`, and `gh`; the fourth grade holds voiced fricatives, `dh`
//! anto and `v` ampa, as does a word's last `f`, said v. A nasal before a consonant of its own
//! series is a bar over that consonant, as in `nd` and `mb`, except `ng` at the end of a word, said
//! as one sound and written nwalmë. `rh` and `lh` are arda and alda, `hw` the Sindarin hwesta, `w`
//! vala, and `h` hyarmen. `r` is rómen before a vowel and órë elsewhere. Doubled consonants take the
//! bar below, except `ss`, essë. `y` is a vowel. The diphthongs `ai`, `ei`, `ui`, `ae`, and `oe`
//! are written on yanta, and `au`, or `aw` at the end of a word, on úrë.

use super::letters::*;
use super::{Letter, Sound, Tengwa, Vowel, consonants, vowel_at};

/// Clusters with a tengwa of their own, or a nasal written as a bar over the consonant after it.
const CLUSTERS: &[(&str, char, Option<char>)] = &[
    ("nth", THULE, Some(NASALIZER)),
    ("nc", QUESSE, Some(NASALIZER)),
    ("nk", QUESSE, Some(NASALIZER)),
    ("ng", UNGWE, Some(NASALIZER)),
    ("nt", TINCO, Some(NASALIZER)),
    ("nd", ANDO, Some(NASALIZER)),
    ("mp", PARMA, Some(NASALIZER)),
    ("mb", UMBAR, Some(NASALIZER)),
    ("th", THULE, None),
    ("dh", ANTO, None),
    ("ph", FORMEN, None),
    ("ch", HWESTA, None),
    ("gh", UNQUE, None),
    ("rh", ARDA, None),
    ("lh", ALDA, None),
    ("hw", HWESTA_SINDARINWA, None),
    ("mh", MH, None),
    ("ss", ARE, None),
];

/// The sounds of a word's letters.
pub(super) fn sounds(letters: &[Letter]) -> Vec<Sound> {
    let mut sounds = Vec::new();
    let mut at = 0;
    while at < letters.len() {
        match letters[at] {
            Letter::Vowel { vowel, long, .. } => match diphthong(vowel, long, letters, at + 1) {
                Some(glide) => {
                    sounds.push(Sound::Diphthong { vowel, glide });
                    at += 2;
                }
                None => {
                    sounds.push(Sound::Vowel { vowel, long });
                    at += 1;
                }
            },
            Letter::Consonant(first) => {
                let (tengwa, used) = consonant(letters, at, first);
                sounds.push(Sound::Consonant(tengwa));
                at += used;
            }
            Letter::Other(character) => {
                sounds.push(Sound::Other(character));
                at += 1;
            }
        }
    }
    sounds
}

/// The glide of a diphthong that a short `vowel` starts with the letter at `next`, yanta or úrë;
/// `None` when they are not one.
fn diphthong(vowel: Vowel, long: bool, letters: &[Letter], next: usize) -> Option<char> {
    if long {
        return None;
    }
    let glide = match letters.get(next)? {
        Letter::Vowel {
            vowel: second,
            long: false,
            apart: false,
        } => match (vowel, second) {
            (Vowel::A | Vowel::E | Vowel::U, Vowel::I) | (Vowel::A | Vowel::O, Vowel::E) => YANTA,
            (Vowel::A, Vowel::U) => URE,
            _ => return None,
        },
        Letter::Consonant('w') if vowel == Vowel::A && next + 1 == letters.len() => URE,
        _ => return None,
    };
    Some(glide)
}

/// The tengwa of the consonants at `at`, which start with `first`, and how many letters it takes.
fn consonant(letters: &[Letter], at: usize, first: char) -> (Tengwa, usize) {
    let ahead = consonants(&letters[at..]);
    if ahead == "ng" && at + 2 == letters.len() {
        return (Tengwa::new(NWALME), 2);
    }
    if let Some((cluster, letter, mark)) = CLUSTERS.iter().find(|(cluster, ..)| ahead.starts_with(cluster)) {
        let tengwa = Tengwa {
            letter: *letter,
            marks: mark.iter().copied().collect(),
        };
        return (tengwa, cluster.len());
    }
    let doubled = ahead.chars().nth(1) == Some(first);
    let used = 1 + usize::from(doubled);
    let last = at + used == letters.len();
    let letter = match first {
        't' => TINCO,
        'p' => PARMA,
        'c' | 'k' | 'q' | 'x' => QUESSE,
        'd' => ANDO,
        'b' => UMBAR,
        'g' => UNGWE,
        'f' if last => AMPA,
        'f' => FORMEN,
        'v' => AMPA,
        'h' => HYARMEN,
        'm' => MALTA,
        'r' if vowel_at(letters, at + used) => ROMEN,
        'r' => ORE,
        'l' => LAMBE,
        's' => SILME,
        'w' => VALA,
        'z' => ARE,
        'j' => ANNA,
        _ => NUMEN,
    };
    let marks = if doubled { vec![DOUBLER] } else { Vec::new() };
    (Tengwa { letter, marks }, used)
}
