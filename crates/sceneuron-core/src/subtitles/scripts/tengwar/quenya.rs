// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The classical mode for Quenya, as in Tolkien's Namárië: the third series is velar, `c`, and the
//! fourth labiovelar, `qu`; the fourth grade holds the nasal clusters `nt`, `mp`, `nc`, and `nqu`,
//! with `nd`, `mb`, `ng`, and `ngw` above them, and `ld`, `rd`, and `ss` have letters of their own.
//! A word starting with `ng` or `nw` starts with the velar nasal, noldo or nwalmë. `r` is rómen
//! before a vowel and órë elsewhere; `h` is hyarmen at the start of a word and aha inside it, `hy`
//! is hyarmen with two dots under it, and `hl` and `hr` start with halla. A `y` after a consonant
//! is two dots under that consonant, and anna with them elsewhere. Other doubled consonants take
//! the bar below. The diphthongs `ai`, `oi`, and `ui` are written on yanta, and `au`, `eu`, and
//! `iu` on úrë.

use super::letters::*;
use super::{Letter, Sound, Tengwa, Vowel, consonants, vowel_at};

/// Clusters with a tengwa of their own, longest first.
const CLUSTERS: &[(&str, char)] = &[
    ("nqu", UNQUE),
    ("nkw", UNQUE),
    ("ngw", UNGWE),
    ("nc", ANCA),
    ("nk", ANCA),
    ("nt", ANTO),
    ("mp", AMPA),
    ("nd", ANDO),
    ("mb", UMBAR),
    ("ng", ANGA),
    ("ld", ALDA),
    ("rd", ARDA),
    ("ss", ARE),
    ("th", THULE),
    ("hw", HWESTA),
    ("ch", HARMA),
    ("kw", QUESSE),
    ("qu", QUESSE),
];

/// The clusters that differ at the start of a word: the velar nasal ñ alone and before w.
const STARTING: &[(&str, char)] = &[("ngw", NWALME), ("nw", NWALME), ("ng", NOLDO)];

/// The sounds of a word's letters.
pub(super) fn sounds(letters: &[Letter]) -> Vec<Sound> {
    let mut sounds = Vec::new();
    let mut at = 0;
    while at < letters.len() {
        match letters[at] {
            Letter::Vowel { vowel, long, .. } => match diphthong(vowel, long, letters.get(at + 1)) {
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
                let (tengwar, used) = consonant(letters, at, first);
                at += used;
                sounds.extend(tengwar.into_iter().map(Sound::Consonant));
                if letters.get(at) == Some(&Letter::Consonant('y'))
                    && vowel_at(letters, at + 1)
                    && let Some(Sound::Consonant(last)) = sounds.last_mut()
                    && !last.marks.contains(&TWO_DOTS_BELOW)
                {
                    last.marks.push(TWO_DOTS_BELOW);
                    at += 1;
                }
            }
            Letter::Other(character) => {
                sounds.push(Sound::Other(character));
                at += 1;
            }
        }
    }
    sounds
}

/// The glide of a diphthong that a short `vowel` starts with `next`, yanta or úrë; `None` when
/// they are not one.
fn diphthong(vowel: Vowel, long: bool, next: Option<&Letter>) -> Option<char> {
    let Some(Letter::Vowel {
        vowel: second,
        long: false,
        apart: false,
    }) = next
    else {
        return None;
    };
    match (long, vowel, second) {
        (false, Vowel::A | Vowel::O | Vowel::U, Vowel::I) => Some(YANTA),
        (false, Vowel::A | Vowel::E | Vowel::I, Vowel::U) => Some(URE),
        _ => None,
    }
}

/// The tengwar of the consonants at `at`, which start with `first`, and how many letters they
/// take; `qu` takes its vowel.
fn consonant(letters: &[Letter], at: usize, first: char) -> (Vec<Tengwa>, usize) {
    let mut ahead = consonants(&letters[at..]);
    let short_u = matches!(
        letters.get(at + ahead.len()),
        Some(Letter::Vowel {
            vowel: Vowel::U,
            long: false,
            ..
        })
    );
    if ahead.ends_with('q') && short_u {
        ahead.push('u');
    }
    let starting = if at == 0 { STARTING } else { &[] };
    if let Some((cluster, letter)) = starting
        .iter()
        .chain(CLUSTERS)
        .find(|(cluster, _)| ahead.starts_with(cluster))
    {
        return (vec![Tengwa::new(*letter)], cluster.len());
    }
    let after = |used: usize| letters.get(at + used);
    let before_vowel = |used: usize| {
        vowel_at(letters, at + used)
            || (after(used) == Some(&Letter::Consonant('y')) && vowel_at(letters, at + used + 1))
    };
    let tengwar = match first {
        'h' if ahead.starts_with("hy") => return (vec![Tengwa::marked(HYARMEN, TWO_DOTS_BELOW)], 2),
        'h' if ahead.starts_with("hl") => return (vec![Tengwa::new(HALLA), Tengwa::new(LAMBE)], 2),
        'h' if ahead.starts_with("hr") => {
            let r = if before_vowel(2) { ROMEN } else { ORE };
            return (vec![Tengwa::new(HALLA), Tengwa::new(r)], 2);
        }
        'y' | 'j' => return (vec![Tengwa::marked(ANNA, TWO_DOTS_BELOW)], 1),
        'x' => return (vec![Tengwa::new(CALMA), Tengwa::new(SILME)], 1),
        _ => first,
    };
    let doubled = ahead.chars().nth(1) == Some(first);
    let used = 1 + usize::from(doubled);
    let mut tengwa = Tengwa::new(single(tengwar, at == 0, before_vowel(used)));
    if doubled {
        tengwa.marks.push(DOUBLER);
    }
    (vec![tengwa], used)
}

/// The tengwa of one consonant: `start` at the start of a word, `before_vowel` when a vowel comes
/// next.
fn single(consonant: char, start: bool, before_vowel: bool) -> char {
    match consonant {
        't' => TINCO,
        'p' => PARMA,
        'c' | 'k' => CALMA,
        'q' => QUESSE,
        'd' => ANDO,
        'b' => UMBAR,
        'g' => ANGA,
        'f' => FORMEN,
        'h' if start => HYARMEN,
        'h' => HARMA,
        'm' => MALTA,
        'ñ' => NOLDO,
        'r' if before_vowel => ROMEN,
        'r' => ORE,
        'v' => VALA,
        'w' => VILYA,
        'l' => LAMBE,
        's' => SILME,
        'z' => ARE,
        _ => NUMEN,
    }
}
