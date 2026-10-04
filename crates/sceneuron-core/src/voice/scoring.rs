// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Which card heard words name: scores against each card's name and text, and the trigger
//! phrases that words contain.
//!
//! A card named inside a longer sentence counts only when its words are at least half of the
//! meaningful words said, a card named by a number alone ("2") only when the number is all that was said, and nothing
//! counts after a negation the card itself does not contain, so "I don't agree" and "what is 2
//! plus 2" raise nothing.

use std::cmp::Reverse;
use std::collections::BTreeSet;

use super::Intent;
use super::matcher::Miss;
use super::normalize::{is_content, words};
use crate::decks::{Card, Deck};

/// The least score a card or a reaction needs, out of 100.
pub(super) const MIN_SCORE: u32 = 60;
/// How far ahead of the next card or reaction the best one must be.
pub(super) const MIN_LEAD: u32 = 10;
/// Words that turn what follows into its opposite, apostrophes removed as in [`words`].
pub(super) const NEGATIONS: [&str; 17] = [
    "not", "no", "never", "dont", "doesnt", "didnt", "cant", "cannot", "wont", "isnt", "arent", "wasnt", "werent",
    "shouldnt", "wouldnt", "nothing", "nor",
];

/// The one card that the words name clearly, or why there is none: no card matched, which the AI
/// may still understand, or two cards that match about as well.
pub(super) fn best_card(words: &[String], decks: &[&Deck]) -> Result<Intent, Miss> {
    let scored = scored_cards(words, decks);
    let said = words.join(" ");
    match scored.as_slice() {
        [] => Err(Miss::unknown(format!("no card matched {said:?}"))),
        [(best, ..), ..] if *best < MIN_SCORE => Err(Miss::unknown(format!("no card matched {said:?}"))),
        [(best, ..), (next, deck, card), ..] if *best < next + MIN_LEAD => {
            let (_, first_deck, first_card) = scored[0];
            let both = names(&[(first_deck, first_card), (deck, card)]);
            Err(Miss::refused(format!("{said:?} could be {both}")))
        }
        [(_, deck, card), ..] => Ok(raise(deck, card, None)),
    }
}

/// The card the words name best, with its score, if any scores at all.
pub(super) fn top_card<'d>(words: &[String], decks: &[&'d Deck]) -> Option<(u32, &'d Deck, &'d Card)> {
    scored_cards(words, decks).into_iter().next()
}

/// Every card the words name at all, best first.
fn scored_cards<'d>(words: &[String], decks: &[&'d Deck]) -> Vec<(u32, &'d Deck, &'d Card)> {
    let mut scored: Vec<(u32, &Deck, &Card)> = decks
        .iter()
        .flat_map(|deck| {
            deck.cards
                .iter()
                .map(move |card| (score_card(words, card), *deck, card))
        })
        .filter(|(score, _, _)| *score > 0)
        .collect();
    scored.sort_by_key(|(score, ..)| Reverse(*score));
    scored
}

/// How well the words name a card, out of 100, by its name or its text.
fn score_card(said: &[String], card: &Card) -> u32 {
    let text = card.props.text.as_deref().unwrap_or_default();
    [card.name.as_str(), text]
        .into_iter()
        .map(|candidate| score(said, &words(candidate)))
        .max()
        .unwrap_or(0)
}

fn score(said: &[String], candidate: &[String]) -> u32 {
    if candidate.is_empty() {
        return 0;
    }
    if said == candidate {
        return 100;
    }
    // A number names a card only as an exact utterance or a vote; "give me one second" is not 1.
    if matches!(candidate, [only] if only.chars().all(|c| c.is_ascii_digit())) {
        return 0;
    }
    let meaningful = |words: &[String]| words.iter().any(|word| is_content(word));
    if meaningful(said) && contains(candidate, said) {
        return 90;
    }
    if said
        .iter()
        .any(|word| NEGATIONS.contains(&word.as_str()) && !candidate.contains(word))
    {
        return 0;
    }
    let content = |words: &[String]| -> BTreeSet<String> { words.iter().filter(|w| is_content(w)).cloned().collect() };
    let (a, b) = (content(said), content(candidate));
    // The card's words must be at least half of what was said that carries meaning.
    if meaningful(candidate) && contains(said, candidate) && candidate.len() * 2 >= a.len() {
        return 80;
    }
    if a.is_empty() || b.is_empty() {
        return 0;
    }
    // Shared meaningful words, scaled so that the same words in another order score 70.
    let shared = a.intersection(&b).count();
    u32::try_from(140 * shared / (a.len() + b.len())).unwrap_or(0)
}

/// The card whose trigger phrase the words contain, with the phrase as the card spells it; or
/// the phrase and why it raises nothing when two cards tie.
pub(super) fn phrase(heard: &[String], decks: &[&Deck]) -> Result<Option<Intent>, (String, String)> {
    let mut found: Vec<(usize, &str, &Deck, &Card)> = Vec::new();
    for deck in decks {
        for card in &deck.cards {
            let longest = card
                .phrases
                .iter()
                .map(|phrase| (words(phrase).len(), phrase.as_str()))
                .filter(|(_, phrase)| contains(heard, &words(phrase)))
                .max_by_key(|(length, _)| *length);
            if let Some((length, phrase)) = longest {
                found.push((length, phrase, deck, card));
            }
        }
    }
    found.sort_by_key(|(length, ..)| Reverse(*length));
    match found.as_slice() {
        [] => Ok(None),
        [(length, phrase, ..), (next, _, deck, card), ..] if length == next => {
            let (_, _, first_deck, first_card) = found[0];
            let both = names(&[(first_deck, first_card), (deck, card)]);
            Err((
                (*phrase).to_owned(),
                format!("the trigger phrase {phrase:?} could be {both}"),
            ))
        }
        [(_, phrase, deck, card), ..] => Ok(Some(raise(deck, card, Some(phrase)))),
    }
}

pub(super) fn raise(deck: &Deck, card: &Card, phrase: Option<&str>) -> Intent {
    Intent::Raise {
        deck: deck.id.clone(),
        card: card.id.clone(),
        phrase: phrase.map(str::to_owned),
    }
}

/// Whether `part` appears in `whole` as consecutive words.
pub(super) fn contains(whole: &[String], part: &[String]) -> bool {
    !part.is_empty() && whole.windows(part.len()).any(|window| window == part)
}

/// "Deck: Card and Deck: Card", for refusals.
pub(super) fn names(cards: &[(&Deck, &Card)]) -> String {
    let named: Vec<String> = cards
        .iter()
        .map(|(deck, card)| format!("{}: {}", deck.name, card.name))
        .collect();
    named.join(" or ")
}
