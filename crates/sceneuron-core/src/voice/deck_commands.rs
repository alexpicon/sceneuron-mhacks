// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The commands addressed words give the decks and automation: lower, pause, resume, or a vote.

use super::commands::{PAUSE, RESUME, said_one_of};
use super::matcher::Intent;
use super::normalize::words;
use super::scoring::{names, raise};
use crate::decks::{Card, Deck};

const LOWER: [&str; 3] = ["lower", "hide", "clear"];
/// What "lower" may be followed by to lower every deck with voice on.
const EVERYTHING: [&str; 7] = ["all", "everything", "it", "them", "that", "card", "cards"];
/// Ways to start a vote, longest first.
pub(super) const VOTES: [&[&str]; 6] = [
    &["my", "vote", "is"],
    &["my", "vote"],
    &["i", "vote", "for"],
    &["i", "vote"],
    &["vote", "for"],
    &["vote"],
];

/// Lower, pause, resume, or a vote; `None` when the words are none of these.
pub(super) fn command(words: &[String], decks: &[&Deck]) -> Option<Result<Intent, String>> {
    if said_one_of(words, &PAUSE) {
        return Some(Ok(Intent::Pause));
    }
    if said_one_of(words, &RESUME) {
        return Some(Ok(Intent::Resume));
    }
    if LOWER.contains(&words[0].as_str()) {
        return lower(&words[1..], decks).map(Ok);
    }
    let vote = VOTES
        .iter()
        .find(|prefix| words.len() > prefix.len() && words.iter().zip(prefix.iter()).all(|(a, b)| a == b))?;
    let mut value = &words[vote.len()..];
    if let Some((first, rest)) = value.split_first()
        && !rest.is_empty()
        && ["a", "an", "the"].contains(&first.as_str())
    {
        value = rest;
    }
    if let Some((last, rest)) = value.split_last()
        && !rest.is_empty()
        && (last == "points" || last == "point")
    {
        value = rest;
    }
    let value = value.join(" ");
    let found: Vec<(&Deck, &Card)> = decks
        .iter()
        .flat_map(|deck| deck.cards.iter().map(move |card| (*deck, card)))
        .filter(|(_, card)| words_of(&card.name) == value)
        .collect();
    Some(match found.as_slice() {
        [(deck, card)] => Ok(raise(deck, card, None)),
        [] => Err(format!("no card for the vote {value:?}")),
        _ => Err(format!(
            "the vote {value:?} is on more than one deck: {}",
            names(&found)
        )),
    })
}

/// Lowering every voice deck, or the one the words name; `None` when they name no deck, so
/// they can still name a card.
fn lower(target: &[String], decks: &[&Deck]) -> Option<Intent> {
    let target = without_ends(target);
    let all = target.is_empty() || (target.len() == 1 && EVERYTHING.contains(&target[0].as_str()));
    let chosen: Vec<String> = decks
        .iter()
        .filter(|deck| all || deck_names(deck).contains(&target.join(" ")))
        .map(|deck| deck.id.clone())
        .collect();
    (all || !chosen.is_empty()).then_some(Intent::Lower { decks: chosen })
}

/// Words naming a deck without "the" in front or "card", "cards", "deck", or "signals" behind.
fn without_ends(words: &[String]) -> &[String] {
    let words = match words.first() {
        Some(first) if ["the", "my", "this"].contains(&first.as_str()) => &words[1..],
        _ => words,
    };
    match words.split_last() {
        Some((last, rest)) if !rest.is_empty() && ["card", "cards", "deck", "signals"].contains(&last.as_str()) => rest,
        _ => words,
    }
}

fn deck_names(deck: &Deck) -> Vec<String> {
    [deck.name.as_str(), deck.id.as_str(), deck.element.as_str()]
        .iter()
        .map(|name| without_ends(&words(name)).join(" "))
        .collect()
}

fn words_of(text: &str) -> String {
    words(text).join(" ")
}
