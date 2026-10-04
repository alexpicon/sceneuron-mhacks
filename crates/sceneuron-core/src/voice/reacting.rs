// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Reactions in what was said: addressed words name a preset by its name, short name, banner, or a
//! trigger phrase, and in always-on mode any speech may contain a trigger phrase. A reaction named
//! as plainly as a card, or a trigger phrase as long as a card's, plays nothing and says why.

use std::cmp::{Ordering, Reverse};

use super::VoiceAction;
use super::commands::{PLAY, fill};
use super::matcher::{Intent, Miss};
use super::normalize::{is_content, literal_words, words};
use super::scoring::{MIN_LEAD, MIN_SCORE, NEGATIONS, best_card, contains, names, phrase, top_card};
use crate::decks::Deck;
use crate::reactions::{Flood, Preset, Reactions, presets};

/// Every name addressed words may call a preset by.
pub(super) fn reaction_names(preset: &Preset) -> Vec<&'static str> {
    let mut names = vec![preset.name, preset.short];
    names.extend(preset.banner);
    names.extend(preset.phrases);
    names
}

/// The reaction the words name clearly, with its score: all of them as one of its names scores
/// 100, and one of its names as at least half of the meaningful words 80. Nothing counts after a
/// negation, so "don't ship it" plays nothing.
pub(super) fn top_reaction(said: &[String]) -> Option<(u32, Flood)> {
    if said.iter().any(|word| NEGATIONS.contains(&word.as_str())) {
        return None;
    }
    let meaningful = said.iter().filter(|word| is_content(word)).count();
    let score = |name: &str| {
        let name = words(name);
        if name == said {
            100
        } else if contains(said, &name) && name.len() * 2 >= meaningful {
            80
        } else {
            0
        }
    };
    let mut scored: Vec<(u32, Flood)> = presets()
        .iter()
        .map(|preset| {
            let best = reaction_names(preset).into_iter().map(score).max().unwrap_or(0);
            (best, preset.flood)
        })
        .filter(|(score, _)| *score >= MIN_SCORE)
        .collect();
    scored.sort_by_key(|(score, _)| Reverse(*score));
    match scored.as_slice() {
        [(best, _), (next, _), ..] if *best < next + MIN_LEAD => None,
        [first, ..] => Some(*first),
        [] => None,
    }
}

/// The reaction or card that addressed words name: a reaction that a card matches about as well
/// plays nothing, and the refusal says how to ask for the reaction.
pub(super) fn named(asked: &[String], decks: &[&Deck]) -> Result<Intent, Miss> {
    let Some((reaction_score, flood)) = top_reaction(asked) else {
        return best_card(asked, decks);
    };
    match top_card(asked, decks) {
        Some((card_score, ..)) if card_score >= reaction_score + MIN_LEAD => best_card(asked, decks),
        Some((card_score, deck, card)) if card_score >= MIN_SCORE && card_score + MIN_LEAD > reaction_score => {
            let preset = flood.preset();
            let play = fill(PLAY[0], &preset.name.to_lowercase());
            let card = names(&[(deck, card)]);
            let said = asked.join(" ");
            let reason = format!(
                "{said:?} could be the {} reaction or {card}; say {play:?} for the reaction",
                preset.name
            );
            Err(Miss::refused(reason))
        }
        _ => Ok(Intent::Act(VoiceAction::React { reaction: flood })),
    }
}

/// The longest trigger phrase the speech contains among presets whose phrases play, by literal
/// words, with its length in words and as the preset spells it. A phrase right after a negation,
/// as in "we shouldn't ship it yet", does not count.
fn reaction_phrase(literal: &[String], reactions: &Reactions) -> Option<(usize, &'static str, Flood)> {
    presets()
        .iter()
        .filter(|preset| reactions.hears(preset.flood))
        .flat_map(|preset| preset.phrases.iter().map(move |phrase| (*phrase, preset.flood)))
        .map(|(phrase, flood)| (literal_words(phrase), phrase, flood))
        .filter(|(words, ..)| said_plainly(literal, words))
        .map(|(words, phrase, flood)| (words.len(), phrase, flood))
        .max_by_key(|(length, ..)| *length)
}

/// Whether `phrase` appears in `heard` with no negation among the few words before it.
fn said_plainly(heard: &[String], phrase: &[String]) -> bool {
    const BEFORE: usize = 3;
    !phrase.is_empty()
        && heard.windows(phrase.len()).enumerate().any(|(start, window)| {
            let before = &heard[start.saturating_sub(BEFORE)..start];
            window == phrase && !before.iter().any(|word| NEGATIONS.contains(&word.as_str()))
        })
}

/// The card or reaction whose trigger phrase the speech contains; the longest phrase wins, and a
/// card's and a reaction's phrases of the same length play nothing, which the error names.
pub(super) fn trigger(
    text: &str,
    heard: &[String],
    decks: &[&Deck],
    reactions: &Reactions,
) -> Result<Option<Intent>, (String, String)> {
    let card = phrase(heard, decks);
    let Some((length, said, flood)) = reaction_phrase(&literal_words(text), reactions) else {
        return card;
    };
    let play = Intent::Play {
        flood,
        phrase: said.to_owned(),
    };
    let (other, raised) = match &card {
        Ok(Some(Intent::Raise {
            phrase: Some(other),
            deck,
            card,
        })) => (other.clone(), Some((deck.as_str(), card.as_str()))),
        Err((other, _)) => (other.clone(), None),
        _ => return Ok(Some(play)),
    };
    match words(&other).len().cmp(&length) {
        Ordering::Greater => card,
        Ordering::Less => Ok(Some(play)),
        Ordering::Equal => {
            let shown = raised
                .and_then(|(deck, card)| card_name(decks, deck, card))
                .unwrap_or_else(|| format!("a card's phrase {other:?}"));
            let name = flood.preset().name;
            let reason = format!("the trigger phrase {said:?} could be the {name} reaction or {shown}");
            Err((said.to_owned(), reason))
        }
    }
}

fn card_name(decks: &[&Deck], deck: &str, card: &str) -> Option<String> {
    let deck = decks.iter().find(|found| found.id == deck)?;
    let card = deck.card(card)?;
    Some(names(&[(deck, card)]))
}
