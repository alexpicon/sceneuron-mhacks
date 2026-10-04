// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What heard words ask for, decided without side effects.
//!
//! Addressed words lose the wake word and polite fillers, then are read as a command (lower,
//! pause, resume, or a vote), a local command (see [`local`](super::local)), or else as the name
//! or text of one card or the name of a reaction. A card must be a clear best match: a weak match,
//! or two cards that match about as well, raise nothing. In always-on mode, any final words may
//! also contain a trigger phrase of a card or a reaction; the longest phrase wins, and two with
//! equally long phrases do nothing. Addressed words that nothing here understands go to the AI,
//! unless they say nothing meaningful or name several things at once.

use super::deck_commands::command;
use super::local::local_command;
use super::normalize::{distance, is_content, words};
use super::reacting::{named, trigger};
use super::{VoiceAction, VoiceMode};
use crate::decks::Deck;
use crate::model::Project;
use crate::reactions::Flood;

/// Words that may come before the wake word.
const LEAD_INS: [&str; 7] = ["hey", "ok", "okay", "so", "um", "uh", "oh"];
/// Words that alone ask the AI for nothing.
const NOISE: [&str; 12] = [
    "um", "uh", "oh", "hmm", "hm", "ah", "er", "erm", "ok", "okay", "so", "well",
];

/// What heard words ask for.
#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    /// Raise a card; `phrase` is the trigger phrase that chose it, as the card spells it, rather
    /// than an addressed command.
    Raise {
        deck: String,
        card: String,
        phrase: Option<String>,
    },
    /// Lower whatever these decks show.
    Lower {
        decks: Vec<String>,
    },
    Pause,
    Resume,
    /// Carry out an action at once, as addressed words ask: play a reaction, switch the theme,
    /// show or hide an element, or have the host look or change subtitles.
    Act(VoiceAction),
    /// Speech contains a reaction's trigger phrase, as the preset spells it: play it as
    /// automation.
    Play {
        flood: Flood,
        phrase: String,
    },
    /// Addressed words that no local command understood, for the AI to read.
    Ask {
        text: String,
    },
    /// Do nothing; `addressed` tells whether the words were meant for Sceneuron, and `reason`
    /// says why nothing happens.
    Ignore {
        addressed: bool,
        reason: String,
    },
    /// Words not meant for Sceneuron contain a trigger phrase that more than one card or
    /// reaction has, so nothing happens; `reason` names the phrase and what has it, never the
    /// other words.
    Unclear {
        phrase: String,
        reason: String,
    },
}

/// Why addressed words did nothing here, and whether the AI may still understand them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Miss {
    pub(super) reason: String,
    pub(super) ask: bool,
}

impl Miss {
    /// Nothing here understood the words.
    pub(super) fn unknown(reason: String) -> Miss {
        Miss { reason, ask: true }
    }

    /// The words were understood, but name more than one thing or nothing that exists.
    pub(super) fn refused(reason: String) -> Miss {
        Miss { reason, ask: false }
    }
}

/// Reads final words heard in `mode` against `project`; only decks whose voice is on take part,
/// and only reactions whose trigger phrases play (see [`Reactions::hears`](crate::Reactions::hears)).
pub fn interpret(text: &str, addressed: bool, mode: VoiceMode, wake_word: &str, project: &Project) -> Intent {
    let heard = words(text);
    let decks: Vec<&Deck> = project.decks.iter().filter(|deck| deck.voice).collect();
    let after_wake = strip_wake_word(&heard, &words(wake_word));
    let addressed = addressed || after_wake.is_some();
    let mut miss = Miss::refused(String::new());
    let asked = trim_fillers(after_wake.unwrap_or(&heard));
    if addressed {
        if asked.is_empty() {
            miss = Miss::refused("heard only the wake word".to_owned());
        } else {
            let found = command(asked, &decks)
                .map(|result| result.map_err(Miss::refused))
                .or_else(|| local_command(asked, project, &decks))
                .unwrap_or_else(|| named(asked, &decks));
            match found {
                Ok(intent) => return intent,
                Err(why) => miss = why,
            }
        }
    }
    if mode == VoiceMode::AlwaysOn {
        match trigger(text, &heard, &decks, &project.reactions) {
            Ok(Some(intent)) => return intent,
            Ok(None) => {}
            Err((_, why)) if addressed => return ignore(true, why),
            Err((phrase, reason)) => return Intent::Unclear { phrase, reason },
        }
    }
    let meaningful = asked
        .iter()
        .any(|word| is_content(word) && !NOISE.contains(&word.as_str()));
    if addressed && miss.ask && meaningful {
        return Intent::Ask { text: text.to_owned() };
    }
    ignore(addressed, miss.reason)
}

/// Whether words start with the wake word.
pub(crate) fn is_addressed(text: &str, wake_word: &str) -> bool {
    strip_wake_word(&words(text), &words(wake_word)).is_some()
}

fn ignore(addressed: bool, reason: String) -> Intent {
    Intent::Ignore { addressed, reason }
}

/// The words after a leading wake word, which may follow a lead-in such as "hey" and may be
/// heard as up to three words with a slip or two, as in "scene neuron".
fn strip_wake_word<'a>(heard: &'a [String], wake: &[String]) -> Option<&'a [String]> {
    let wake = wake.concat();
    if wake.is_empty() {
        return None;
    }
    let tolerance = match wake.chars().count() {
        0..=4 => 0,
        5..=8 => 1,
        _ => 2,
    };
    let lead = heard
        .iter()
        .take(2)
        .take_while(|word| LEAD_INS.contains(&word.as_str()))
        .count();
    (0..=lead).rev().find_map(|skip| {
        let rest = &heard[skip..];
        (1..=rest.len().min(3))
            .map(|count| (distance(&rest[..count].concat(), &wake), count))
            .filter(|(slips, _)| *slips <= tolerance)
            .min()
            .map(|(_, count)| &rest[count..])
    })
}

/// Drops "please" and "can you" in front, and "please" or "thanks" at the end.
fn trim_fillers(mut words: &[String]) -> &[String] {
    let starts =
        |words: &[String], start: &[&str]| words.len() >= start.len() && words.iter().zip(start).all(|(a, b)| a == b);
    for filler in [&["please"][..], &["can", "you"], &["could", "you"]] {
        if starts(words, filler) {
            words = &words[filler.len()..];
        }
    }
    while words
        .last()
        .is_some_and(|last| ["please", "thanks", "now"].contains(&last.as_str()))
    {
        words = &words[..words.len() - 1];
    }
    if words.ends_with(&["thank".to_owned(), "you".to_owned()]) {
        words = &words[..words.len() - 2];
    }
    words
}
