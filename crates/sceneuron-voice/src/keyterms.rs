// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Keyterm prompting: the words Scribe is biased toward, within its realtime limits.

/// Scribe v2 Realtime takes at most 50 keyterms of up to 20 characters each, from
/// <https://elevenlabs.io/docs/eleven-api/guides/how-to/speech-to-text/batch/keyterm-prompting>.
pub const MAX_KEYTERMS: usize = 50;
pub const MAX_KEYTERM_CHARS: usize = 20;
/// A term with fewer letters and digits than this, such as a lone poker number, is left out:
/// recognition needs no help with it.
const MIN_KEYTERM_ALPHANUMERICS: usize = 2;

/// The keyterms for a session: the wake word first, then `terms` in order (card names before
/// trigger phrases, so names win when there are too many).
///
/// Each term keeps only letters, digits, apostrophes, and hyphens, with emoji and other
/// punctuation turned into spaces. A term longer than the limit is cut after its last whole
/// word that fits, and dropped when its first word does not fit. Terms that repeat another
/// regardless of case are left out.
pub fn keyterms<'a>(wake_word: &'a str, terms: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut chosen: Vec<String> = Vec::new();
    for term in std::iter::once(wake_word).chain(terms) {
        if chosen.len() == MAX_KEYTERMS {
            break;
        }
        let Some(term) = clean(term) else {
            continue;
        };
        if !chosen.iter().any(|other| other.to_lowercase() == term.to_lowercase()) {
            chosen.push(term);
        }
    }
    chosen
}

fn clean(term: &str) -> Option<String> {
    let kept: String = term
        .chars()
        .map(|c| match c {
            '\u{2019}' => '\'',
            c if c.is_alphanumeric() || c == '\'' || c == '-' => c,
            _ => ' ',
        })
        .collect();
    let mut fitted = String::new();
    for word in kept.split_whitespace() {
        let length = fitted.chars().count() + usize::from(!fitted.is_empty()) + word.chars().count();
        if length > MAX_KEYTERM_CHARS {
            break;
        }
        if !fitted.is_empty() {
            fitted.push(' ');
        }
        fitted.push_str(word);
    }
    let alphanumerics = fitted.chars().filter(|c| c.is_alphanumeric()).count();
    (alphanumerics >= MIN_KEYTERM_ALPHANUMERICS).then_some(fitted)
}
