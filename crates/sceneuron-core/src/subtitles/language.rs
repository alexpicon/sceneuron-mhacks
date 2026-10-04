// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Language codes: BCP 47 tags for the languages subtitles are translated into, and whether two
//! codes name the same language.

use super::long_codes::short_code;

/// The longest tag accepted, in characters.
const MAX_TAG_CHARS: usize = 35;

/// Where a subtag sits in a tag; RFC 5646 allows them only in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Place {
    Language,
    ExtendedLanguage,
    Script,
    Region,
    Variant,
    Extension,
}

/// Whether `tag` is a well-formed BCP 47 language tag (RFC 5646, section 2.1) that starts with a
/// language of 2 or 3 letters, then, each optional and in this order: up to three extended
/// languages of 3 letters, a script of 4 letters, a region of 2 letters or 3 digits, variants,
/// extensions, and a private-use part, `x` and one or more subtags of 1 to 8 letters or digits.
/// Examples are `es`, `pt-BR`, `zh-Hans`, `es-419`, `sr-Latn-RS`, and `art-x-dothraki`. A variant
/// or an extension's letter may not repeat, as RFC 5646 asks of valid tags. Tags that are only
/// private use, such as `x-dothraki`, or grandfathered, such as `i-klingon`, are refused: a
/// language comes first.
pub fn valid_language_tag(tag: &str) -> bool {
    let mut subtags = tag.split('-');
    let language = subtags.next().unwrap_or_default();
    if tag.len() > MAX_TAG_CHARS || !((2..=3).contains(&language.len()) && letters(language)) {
        return false;
    }
    let mut place = Place::Language;
    let mut extended = 0;
    // Variants and extension letters, neither of which may repeat.
    let mut seen: Vec<String> = Vec::new();
    // The subtags after the last extension letter; it needs at least one.
    let mut extension_subtags = 1;
    while let Some(subtag) = subtags.next() {
        if !alphanumeric(subtag) {
            return false;
        }
        let lower = subtag.to_ascii_lowercase();
        if subtag.len() == 1 {
            if extension_subtags == 0 || seen.contains(&lower) {
                return false;
            }
            if lower == "x" {
                let private: Vec<&str> = subtags.collect();
                return !private.is_empty() && private.into_iter().all(alphanumeric);
            }
            seen.push(lower);
            (place, extension_subtags) = (Place::Extension, 0);
        } else if place == Place::Extension {
            extension_subtags += 1;
        } else {
            place = match next_place(subtag, place, extended) {
                Some(Place::Variant) if seen.contains(&lower) => return false,
                Some(next) => next,
                None => return false,
            };
            extended += usize::from(place == Place::ExtendedLanguage);
            if place == Place::Variant {
                seen.push(lower);
            }
        }
    }
    extension_subtags > 0
}

/// Where `subtag`, of 2 to 8 letters or digits after one at `after` and before any extension,
/// sits; `None` when it cannot follow there.
fn next_place(subtag: &str, after: Place, extended: usize) -> Option<Place> {
    let digits = subtag.bytes().all(|b| b.is_ascii_digit());
    let digit_first = subtag.bytes().next().is_some_and(|b| b.is_ascii_digit());
    match subtag.len() {
        3 if letters(subtag) && after <= Place::ExtendedLanguage && extended < 3 => Some(Place::ExtendedLanguage),
        4 if letters(subtag) && after < Place::Script => Some(Place::Script),
        2 if letters(subtag) && after < Place::Region => Some(Place::Region),
        3 if digits && after < Place::Region => Some(Place::Region),
        5..=8 => Some(Place::Variant),
        4 if digit_first => Some(Place::Variant),
        _ => None,
    }
}

fn letters(subtag: &str) -> bool {
    subtag.bytes().all(|b| b.is_ascii_alphabetic())
}

/// Whether `subtag` has 1 to 8 letters or digits, as every subtag must.
fn alphanumeric(subtag: &str) -> bool {
    (1..=8).contains(&subtag.len()) && subtag.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// `tag` with BCP 47's usual case: lowercase, except a two-letter region in uppercase and a
/// four-letter script capitalized, as in `pt-BR` or `zh-Hant`, but never after an extension's or
/// private use's letter, as in `art-x-navi`; `None` when it is not a valid tag.
pub fn canonical_language_tag(tag: &str) -> Option<String> {
    let tag = tag.trim();
    if !valid_language_tag(tag) {
        return None;
    }
    let mut after_letter = false;
    let subtags: Vec<String> = tag
        .split('-')
        .enumerate()
        .map(|(index, subtag)| {
            let lower = subtag.to_ascii_lowercase();
            let cased = match subtag.len() {
                _ if index == 0 || after_letter => lower,
                2 if letters(subtag) => subtag.to_ascii_uppercase(),
                4 if letters(subtag) => lower[..1].to_ascii_uppercase() + &lower[1..],
                _ => lower,
            };
            after_letter |= subtag.len() == 1;
            cased
        })
        .collect();
    Some(subtags.join("-"))
}

/// Whether two codes name the same language: their languages match without regard to case or
/// region, with the three-letter codes of languages that have a two-letter one read as that, so
/// `spa`, `es`, and `es-MX` match. An empty code, a language detected rather than known, matches
/// nothing.
pub fn same_language(one: &str, other: &str) -> bool {
    match (primary(one), primary(other)) {
        (Some(one), Some(other)) => one == other,
        _ => false,
    }
}

/// The language subtag in lowercase, as its ISO 639-1 code when it is a listed three-letter one.
fn primary(code: &str) -> Option<String> {
    let language = code.trim().split('-').next()?.to_ascii_lowercase();
    if language.is_empty() {
        return None;
    }
    Some(short_code(&language).map_or(language, str::to_owned))
}
