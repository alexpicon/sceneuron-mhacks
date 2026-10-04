// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The live description's sentence: what a [`Sighting`] shows, as parts joined by semicolons.

use super::{HandSide, SeenHand, Sighting};
use crate::signals::HandSign;

/// The longest line, in characters; a longer one is cut at a word and ends with "…".
pub const MAX_SEEN_CHARS: usize = 120;
/// Animals and things the line names at most.
const MAX_THINGS: usize = 4;
/// Lines of text the line quotes at most.
const MAX_TEXT_LINES: usize = 2;
/// The longest line of text quoted, in characters; a longer one is cut and ends with "…".
const MAX_QUOTE_CHARS: usize = 24;
const ELLIPSIS: char = '…';

impl Sighting {
    /// The live description of this sighting, such as `2 people; right hand raised, 2 fingers
    /// up; mouth open; seen: cat, guitar; text: “OPEN”`.
    ///
    /// Its parts, in order: how many people are in view, a hand over the face (a facepalm), each
    /// hand with its sign or else its finger count, arms up, an open mouth, the animals and things
    /// in view, and the text in view. Hands that read the same are counted, as in "2 hands
    /// raised". One arm up is left out while a hand is raised, which says as much, and arms are
    /// left out during a facepalm, whose hand holds its arm up. A sighting with no person, hand, or
    /// face in it starts with "Nobody in view". The line starts with a capital and is at most
    /// [`MAX_SEEN_CHARS`] long.
    pub fn describe(&self) -> String {
        let mut parts = Vec::new();
        let personal = !self.hands.is_empty() || self.facepalm || self.arms_up > 0 || self.mouth_open;
        match self.people {
            0 if personal => {}
            0 => parts.push("nobody in view".to_owned()),
            1 => parts.push("1 person".to_owned()),
            people => parts.push(format!("{people} people")),
        }
        if self.facepalm {
            parts.push("facepalm".to_owned());
        }
        parts.extend(hands(&self.hands));
        let raised = self.hands.iter().any(|hand| hand.raised);
        match self.arms_up {
            _ if self.facepalm => {}
            0 => {}
            1 if raised => {}
            1 => parts.push("an arm up".to_owned()),
            _ => parts.push("arms up".to_owned()),
        }
        if self.mouth_open {
            parts.push("mouth open".to_owned());
        }
        let mut things: Vec<&str> = Vec::new();
        for name in self.animals.iter().chain(&self.things).map(|name| name.trim()) {
            if !name.is_empty() && !things.contains(&name) && things.len() < MAX_THINGS {
                things.push(name);
            }
        }
        if !things.is_empty() {
            parts.push(format!("seen: {}", things.join(", ")));
        }
        let quotes: Vec<String> = self
            .text
            .iter()
            .map(|line| one_line(line))
            .filter(|line| !line.is_empty())
            .take(MAX_TEXT_LINES)
            .map(|line| format!("“{}”", cut(&line, MAX_QUOTE_CHARS)))
            .collect();
        if !quotes.is_empty() {
            parts.push(format!("text: {}", quotes.join(", ")));
        }
        capitalized(&cut_at_word(&parts.join("; "), MAX_SEEN_CHARS))
    }
}

/// One part per kind of hand, in the order first seen; hands that read the same are counted.
fn hands(hands: &[SeenHand]) -> Vec<String> {
    let mut kinds: Vec<(HandSide, String, usize)> = Vec::new();
    for hand in hands {
        let rest = state(hand);
        match kinds
            .iter_mut()
            .find(|(side, seen, _)| *side == hand.side && *seen == rest)
        {
            Some((_, _, count)) => *count += 1,
            None => kinds.push((hand.side, rest, 1)),
        }
    }
    kinds
        .into_iter()
        .map(|(side, rest, count)| {
            let noun = match side {
                HandSide::Left => "left hand",
                HandSide::Right => "right hand",
                HandSide::Unknown => "hand",
            };
            if count == 1 {
                format!("{noun}{rest}")
            } else {
                format!("{count} {noun}s{rest}")
            }
        })
        .collect()
}

/// What follows a hand's noun: whether it is raised, then its sign, or else its finger count.
fn state(hand: &SeenHand) -> String {
    let raised = if hand.raised { " raised" } else { "" };
    match (hand.sign, hand.fingers) {
        (Some(sign), _) => format!("{raised}, {}", sign_name(sign)),
        (None, Some(1)) => format!("{raised}, 1 finger up"),
        (None, Some(count)) => format!("{raised}, {count} fingers up"),
        (None, None) if hand.raised => raised.to_owned(),
        (None, None) => " in view".to_owned(),
    }
}

fn sign_name(sign: HandSign) -> &'static str {
    match sign {
        HandSign::ThumbsUp => "thumbs up",
        HandSign::ThumbsDown => "thumbs down",
        HandSign::OpenPalm => "open palm",
        HandSign::Peace => "peace sign",
        HandSign::Pointing => "pointing",
        HandSign::Fist => "fist",
    }
}

/// `text` on one line, with runs of whitespace and control characters as single spaces.
fn one_line(text: &str) -> String {
    let flat: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    flat.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `text` cut to at most `max` characters, ending with "…" when it was cut.
fn cut(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let kept: String = text.chars().take(max - 1).collect();
    format!("{}{ELLIPSIS}", kept.trim_end())
}

/// `text` cut to at most `max` characters, ending with "…" when it was cut. The cut falls between
/// words, where the punctuation before it goes too, unless that would drop more than a third of
/// the line, as inside one long word.
fn cut_at_word(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().take(max).collect();
    let end = match chars.iter().rposition(|c| *c == ' ') {
        Some(space) if space >= max * 2 / 3 => space,
        _ => max - 1,
    };
    let kept: String = chars[..end].iter().collect();
    format!("{}{ELLIPSIS}", kept.trim_end_matches([';', ',', ':', ' ']))
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}
