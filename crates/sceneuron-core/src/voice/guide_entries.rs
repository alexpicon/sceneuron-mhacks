// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The guide's entries of each kind: the ways to say a command, filled in from the matcher's
//! tables with the names of the project's reactions, cards, themes, and elements, and what each
//! does. [`Engine::entry`] keeps only the ways the matcher reads as the command.

use super::VoiceAction;
use super::commands::{
    HIDE, LOOK, NEXT_ROUND, PAUSE, PLAY, RESUME, REVEAL, SHOW, SUBTITLES_INTO, SUBTITLES_OFF, SUBTITLES_ON, THEME,
    THEME_OFF, THEMES, fill,
};
use super::deck_commands::VOTES;
use super::guide::GuideEntry;
use super::matcher::Intent;
use crate::engine::Engine;
use crate::reactions::{Flood, presets};
use crate::theme::Theme;

const NO_VOICE_DECK: &str = "Turn a deck's voice on in the Cards tab to raise its cards by name.";

/// Words that play `flood` at once.
fn plays(flood: Flood) -> impl Fn(&Intent) -> bool {
    move |intent| matches!(intent, Intent::Act(VoiceAction::React { reaction }) if *reaction == flood)
}

/// Words with the trigger phrase of `flood`.
fn phrase_plays(flood: Flood) -> impl Fn(&Intent) -> bool {
    move |intent| matches!(intent, Intent::Play { flood: played, .. } if *played == flood)
}

/// Words that raise the card `id` of any deck, by a phrase or not, as `by_phrase` says.
fn raises(id: &str, by_phrase: Option<bool>) -> impl Fn(&Intent) -> bool {
    move |intent| match intent {
        Intent::Raise { card, phrase, .. } => card == id && by_phrase.is_none_or(|by| by == phrase.is_some()),
        _ => false,
    }
}

/// Words that raise a card of the deck `id` by its name or text.
fn raises_from(id: &str) -> impl Fn(&Intent) -> bool {
    move |intent| matches!(intent, Intent::Raise { deck, phrase: None, .. } if deck == id)
}

/// Words that switch to `theme`.
fn switches_to(theme: Theme) -> impl Fn(&Intent) -> bool {
    move |intent| matches!(intent, Intent::Act(VoiceAction::Theme { theme: chosen }) if *chosen == theme)
}

/// Words that show or hide the element `id`.
fn shows(id: &str) -> impl Fn(&Intent) -> bool {
    move |intent| matches!(intent, Intent::Act(VoiceAction::ShowElement { element, .. }) if element == id)
}

impl Engine {
    pub(super) fn reaction_entries(&self) -> Vec<GuideEntry> {
        let entries = presets().iter().map(|preset| {
            let (name, short) = (preset.name.to_lowercase(), preset.short.to_lowercase());
            let say = [name.clone(), short.clone(), fill(PLAY[0], &short), fill(PLAY[0], &name)];
            let does = format!("plays {} {} at once", preset.emojis[0], preset.name);
            self.entry(&say, &does, plays(preset.flood))
        });
        entries.flatten().collect()
    }

    pub(super) fn phrase_entries(&self) -> Vec<GuideEntry> {
        let heard = presets()
            .iter()
            .filter(|preset| self.project.reactions.hears(preset.flood));
        let reactions = heard.map(|preset| {
            let (emoji, name) = (preset.emojis[0], preset.name);
            let does = format!("plays {emoji} {name} while automation runs, at most once per cooldown");
            self.entry_heard(preset.phrases, &does, false, phrase_plays(preset.flood))
        });
        let decks = self.project.decks.iter().filter(|deck| deck.voice);
        let cards = decks.flat_map(|deck| {
            let phrased = deck.cards.iter().filter(|card| !card.phrases.is_empty());
            phrased.map(move |card| {
                let does = format!("raises {}: {}", deck.name, card.name);
                self.entry_heard(&card.phrases, &does, false, raises(&card.id, Some(true)))
            })
        });
        reactions.chain(cards).flatten().collect()
    }

    pub(super) fn card_entries(&self) -> Vec<GuideEntry> {
        let decks: Vec<_> = self.project.decks.iter().filter(|deck| deck.voice).collect();
        if decks.is_empty() {
            let note = GuideEntry {
                say: Vec::new(),
                does: NO_VOICE_DECK.to_owned(),
            };
            return vec![note];
        }
        let mut entries: Vec<Option<GuideEntry>> = decks
            .iter()
            .map(|deck| {
                let named = deck
                    .cards
                    .iter()
                    .filter(|card| card.name.chars().any(char::is_alphanumeric));
                let say: Vec<String> = named.take(3).map(|card| card.name.clone()).collect();
                let does = format!("raises that card of {}", deck.name);
                self.entry(&say, &does, raises_from(&deck.id))
            })
            .collect();
        let short = decks.iter().flat_map(|deck| &deck.cards).find(|card| {
            let length = card.name.chars().count();
            (1..=3).contains(&length) && card.name.chars().all(char::is_alphanumeric)
        });
        if let Some(card) = short {
            let say = [
                format!("{} {}", VOTES[0].join(" "), card.name),
                format!("vote {}", card.name),
            ];
            let does = "votes: raises the card with exactly that name";
            entries.push(self.entry(&say, does, raises(&card.id, None)));
        }
        let mut lower = vec!["lower".to_owned(), "lower everything".to_owned()];
        lower.extend(decks.iter().map(|deck| format!("lower {}", deck.name.to_lowercase())));
        let lowers = |intent: &Intent| matches!(intent, Intent::Lower { .. });
        entries.push(self.entry(&lower, "lowers the raised cards", lowers));
        entries.into_iter().flatten().collect()
    }

    pub(super) fn theme_entries(&self) -> Vec<GuideEntry> {
        let entries = THEMES.iter().map(|(theme, name, spoken)| {
            let (say, does) = match theme {
                Theme::None => (
                    THEME_OFF[..2].iter().map(|form| (*form).to_owned()).collect(),
                    "turns the theme off".to_owned(),
                ),
                _ => (
                    vec![fill(THEME[0], spoken[0]), fill(THEME[1], spoken[0])],
                    format!("switches the overlay to the {name} theme"),
                ),
            };
            self.entry(&say, &does, switches_to(*theme))
        });
        entries.flatten().collect()
    }

    pub(super) fn element_entries(&self) -> Vec<GuideEntry> {
        let slot = |id: &str| self.project.decks.iter().any(|deck| deck.element == id);
        let elements = self.project.elements.iter().filter(|element| !slot(&element.id));
        let entries = elements.map(|element| {
            let name = element.name.to_lowercase();
            let say = [fill(SHOW[0], &name), fill(HIDE[0], &name)];
            let does = format!("shows or hides {} until you release it in the editor", element.name);
            self.entry(&say, &does, shows(&element.id))
        });
        entries.flatten().collect()
    }

    pub(super) fn other_entries(&self) -> Vec<GuideEntry> {
        let look = |intent: &Intent| matches!(intent, Intent::Act(VoiceAction::Look));
        let room = |intent: &Intent| matches!(intent, Intent::Act(VoiceAction::Room { .. }));
        let subtitles = |intent: &Intent| matches!(intent, Intent::Act(VoiceAction::Subtitles { .. }));
        let language = |intent: &Intent| matches!(intent, Intent::Act(VoiceAction::SubtitleLanguage { .. }));
        let automation = |intent: &Intent| matches!(intent, Intent::Pause | Intent::Resume);
        let into = [fill(SUBTITLES_INTO[0], "Spanish"), fill(SUBTITLES_INTO[3], "Japanese")];
        let languages = "translates subtitles into that language first; any language the Subtitles section lists";
        let automated = "pauses or resumes rules, trigger phrases, and camera reactions";
        let entries = [
            self.entry(&["look now", LOOK[1]], "the vision model looks at the camera now", look),
            self.entry(
                &[REVEAL[2], NEXT_ROUND[0]],
                "reveals the shared room's votes, or starts its next round",
                room,
            ),
            self.entry(
                &[SUBTITLES_ON[0], SUBTITLES_OFF[0]],
                "turns live subtitles on or off",
                subtitles,
            ),
            self.entry(&into, languages, language),
            self.entry(&[PAUSE[0], RESUME[0]], automated, automation),
        ];
        entries.into_iter().flatten().collect()
    }
}
