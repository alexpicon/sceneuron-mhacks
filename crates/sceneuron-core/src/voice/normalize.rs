// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Turning transcripts, card names, and phrases into comparable words, and card text into
//! something a voice can say.
//!
//! Comparison ignores case, punctuation, and emoji. Apostrophes join their word ("let's" is
//! "lets"), number words up to twenty-one become digits, "question mark" becomes `?`, and the
//! coffee cup, "coffee", and "break" all become "coffee", so the poker deck's break card can be
//! named. A `?` standing alone stays, so a card called "?" can be named too. Reaction trigger
//! phrases, heard in any speech, compare literal words, where "break" stays "break", so a coffee
//! break needs both words.

const NUMBERS: [&str; 21] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
    "twenty",
];
const APOSTROPHES: [char; 3] = ['\'', '’', 'ʼ'];
const COFFEE: char = '☕';

/// Words that carry little meaning when comparing what was said with a card.
const STOPWORDS: [&str; 26] = [
    "a", "an", "the", "i", "im", "me", "my", "we", "were", "you", "your", "it", "its", "is", "are", "to", "of", "this",
    "that", "and", "or", "be", "have", "has", "can", "do",
];

/// Lowercase words without punctuation or emoji, with numbers as digits, joined by spaces.
pub fn normalize_speech(text: &str) -> String {
    words(text).join(" ")
}

/// The words of [`normalize_speech`].
pub(crate) fn words(text: &str) -> Vec<String> {
    split_words(text, true)
}

/// [`words`] with "break" kept as it is.
pub(crate) fn literal_words(text: &str) -> Vec<String> {
    split_words(text, false)
}

fn split_words(text: &str, coffee_break: bool) -> Vec<String> {
    let mut lower = String::with_capacity(text.len());
    for c in text.to_lowercase().chars().filter(|c| !APOSTROPHES.contains(c)) {
        if c == COFFEE {
            lower.push_str(" coffee ");
        } else {
            lower.push(c);
        }
    }
    let mut raw: Vec<String> = Vec::new();
    for chunk in lower.split_whitespace() {
        let runs: Vec<&str> = chunk
            .split(|c: char| !c.is_alphanumeric())
            .filter(|run| !run.is_empty())
            .collect();
        if runs.is_empty() && chunk.contains('?') {
            raw.push("?".to_owned());
        }
        raw.extend(runs.into_iter().map(str::to_owned));
    }
    let mut out: Vec<String> = Vec::with_capacity(raw.len());
    let mut index = 0;
    while index < raw.len() {
        let (word, used) = match (raw[index].as_str(), raw.get(index + 1).map(String::as_str)) {
            ("question", Some("mark")) => ("?".to_owned(), 2),
            ("twenty", Some("one")) => ("21".to_owned(), 2),
            ("break", _) if coffee_break => ("coffee".to_owned(), 1),
            (word, _) => match NUMBERS.iter().position(|number| *number == word) {
                Some(value) => (value.to_string(), 1),
                None => (word.to_owned(), 1),
            },
        };
        index += used;
        if out.last() != Some(&word) {
            out.push(word);
        }
    }
    out
}

/// Whether a word carries meaning, unlike "the" or "is".
pub(crate) fn is_content(word: &str) -> bool {
    !STOPWORDS.contains(&word)
}

/// Card text as a voice should say it: emoji and placeholders removed, on one line.
pub fn speakable(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        plain.push_str(&rest[..start]);
        rest = rest[start..].find("}}").map_or("", |end| &rest[start + end + 2..]);
    }
    plain.push_str(rest);
    let kept: String = plain
        .chars()
        .map(|c| if is_emoji(c) || c.is_control() { ' ' } else { c })
        .collect();
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Pictographs, dingbats, and the joiners and selectors that build emoji sequences.
fn is_emoji(c: char) -> bool {
    matches!(
        u32::from(c),
        0x203C
            | 0x2049
            | 0x2122
            | 0x2139
            | 0x2190..=0x21FF
            | 0x2300..=0x23FF
            | 0x2460..=0x24FF
            | 0x25A0..=0x27BF
            | 0x2900..=0x297F
            | 0x2B00..=0x2BFF
            | 0x3030
            | 0x303D
            | 0x3297
            | 0x3299
            | 0x200D
            | 0x20E3
            | 0xFE0E
            | 0xFE0F
            | 0x1F000..=0x1FAFF
            | 0xE0020..=0xE007F
    )
}

/// The edit distance between two short strings, in characters.
pub(crate) fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substituted = previous + usize::from(ca != *cb);
            previous = row[j + 1];
            row[j + 1] = substituted.min(row[j] + 1).min(previous + 1);
        }
    }
    row[b.len()]
}
