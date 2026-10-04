// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Carrying out what voice control understood: cards raised and lowered, automation paused and
//! resumed, reactions played, and the actions of local commands and the AI (see [`act`]). What
//! only the host can do waits for it (see [`requests`](super::requests)).
//!
//! [`act`]: Engine::act

use super::actions::{VoiceAction, VoiceTask};
use super::matcher::Intent;
use crate::decks::RaisedBy;
use crate::engine::Engine;

/// What carrying something out did: in words, the deck and card it raised, whether the change
/// already logged itself, and whether the saved project changed.
pub(super) struct Done {
    pub(super) result: String,
    pub(super) raised: Option<(String, String)>,
    pub(super) logged: bool,
    pub(super) saved: bool,
}

impl Done {
    pub(super) fn said(result: String, logged: bool) -> Done {
        Done {
            result,
            raised: None,
            logged,
            saved: false,
        }
    }
}

impl Engine {
    /// Carries out an intent; returns what it did in words, the card it raised, and whether
    /// the saved project changed. Every result is logged, by the change itself where it made one.
    pub(super) fn act_on(&mut self, intent: Intent, now_ms: u64) -> (String, Option<(String, String)>, bool) {
        let done = match intent {
            Intent::Raise { phrase: Some(_), .. } | Intent::Play { .. } if !self.project.automation_enabled => {
                Done::said("a trigger phrase, but automation is paused".to_owned(), false)
            }
            Intent::Raise { deck, card, phrase } => self.raise_by_voice(deck, card, phrase.is_some(), now_ms),
            Intent::Lower { decks } => self.lower_by_voice(&decks, now_ms),
            Intent::Pause => self.automation_by_voice(false, now_ms),
            Intent::Resume => self.automation_by_voice(true, now_ms),
            Intent::Play { flood, phrase } => {
                let name = flood.preset().name;
                let result = match self.rule_flood(flood, now_ms) {
                    Ok(()) => format!("played {} {name}", flood.preset().emojis[0]),
                    Err(ago_ms) => format!("without {name:?}: the last flood began {} s ago", ago_ms / 1_000),
                };
                let detail = format!("{result}, for the trigger phrase {phrase:?}");
                self.record(now_ms, "voice", &detail);
                Done::said(result, true)
            }
            Intent::Act(action) if action.for_host() => {
                Done::said(self.ask_host(VoiceTask::Act { action }, now_ms), false)
            }
            Intent::Act(action) => self.act(&action, now_ms),
            Intent::Ask { text } => Done::said(self.ask_host(VoiceTask::Ask { text }, now_ms), false),
            Intent::Ignore { reason, .. } | Intent::Unclear { reason, .. } => Done::said(reason, false),
        };
        if !done.logged {
            self.record(now_ms, "voice", &done.result);
        }
        (done.result, done.raised, done.saved)
    }

    fn raise_by_voice(&mut self, deck: String, card: String, by_phrase: bool, now_ms: u64) -> Done {
        let names = self
            .project
            .deck(&deck)
            .and_then(|found| Some((found.name.clone(), found.card(&card)?.name.clone())));
        let Some((deck_name, card_name)) = names else {
            return Done::said("that card is gone".to_owned(), false);
        };
        // Raising the shown card again restarts its auto-hide and speaks it again, which a
        // microphone that hears the overlay would take for the phrase once more.
        let again = by_phrase && self.raised_card(&deck) == Some(card.as_str());
        let raised = Some((deck.clone(), card.clone()));
        if !again && self.raise_card(&deck, &card, RaisedBy::Voice, now_ms).changed {
            let result = format!("raised {deck_name}: {card_name}");
            return Done {
                raised,
                ..Done::said(result, true)
            };
        }
        let result = format!("{deck_name} already shows {card_name}");
        self.record(now_ms, "voice", &result);
        Done {
            raised,
            ..Done::said(result, true)
        }
    }

    pub(super) fn lower_by_voice(&mut self, decks: &[String], now_ms: u64) -> Done {
        let mut lowered = Vec::new();
        for deck in decks {
            let name = self
                .project
                .deck(deck)
                .map_or_else(|| deck.clone(), |found| found.name.clone());
            if self.lower_deck(deck, "voice", now_ms).changed {
                lowered.push(name);
            }
        }
        match lowered.as_slice() {
            [] => Done::said("no card was up".to_owned(), false),
            names => Done::said(format!("lowered {}", names.join(", ")), true),
        }
    }

    pub(super) fn automation_by_voice(&mut self, enabled: bool, now_ms: u64) -> Done {
        let saved = self.switch_automation(enabled, "voice", now_ms).project_dirty;
        let verb = if enabled { "resumed" } else { "paused" };
        if saved {
            Done {
                saved,
                ..Done::said(format!("{verb} automation"), true)
            }
        } else {
            Done::said(format!("automation was already {verb}"), false)
        }
    }

    /// Carries out an action at once, from a local command or the AI; an action only the host
    /// can carry out does nothing here.
    pub(super) fn act(&mut self, action: &VoiceAction, now_ms: u64) -> Done {
        match action {
            VoiceAction::React { reaction } => {
                self.start_flood(*reaction, now_ms);
                let detail = format!("flooded the overlay with {}", reaction.describe());
                self.record(now_ms, "voice", &detail);
                let preset = reaction.preset();
                Done::said(format!("played {} {}", preset.emojis[0], preset.name), true)
            }
            VoiceAction::Theme { theme } => self.theme_by_voice(*theme, now_ms),
            VoiceAction::ShowElement { element, visible } => self.show_by_voice(element, *visible, now_ms),
            VoiceAction::RaiseCard { deck, card } => match self.project.deck(deck) {
                Some(found) if found.voice => self.raise_by_voice(deck.clone(), card.clone(), false, now_ms),
                Some(found) => {
                    let result = format!(
                        "{} does not answer voice; turn its voice on in the Cards tab",
                        found.name
                    );
                    Done::said(result, false)
                }
                None => Done::said(format!("there is no deck {deck:?}"), false),
            },
            VoiceAction::LowerCards { deck } => {
                let voiced = |id: &str| self.project.deck(id).is_some_and(|found| found.voice);
                let decks: Vec<String> = if deck.is_empty() {
                    self.project
                        .decks
                        .iter()
                        .filter(|found| found.voice)
                        .map(|found| found.id.clone())
                        .collect()
                } else if voiced(deck) {
                    vec![deck.clone()]
                } else {
                    return Done::said(format!("there is no deck {deck:?} that voice can lower"), false);
                };
                self.lower_by_voice(&decks, now_ms)
            }
            VoiceAction::SetText { element, text } => self.text_by_voice(element, text, now_ms),
            VoiceAction::ReleaseElement { element } => self.release_by_voice(element, now_ms),
            VoiceAction::Automation { enabled } => self.automation_by_voice(*enabled, now_ms),
            VoiceAction::EditProject { summary, operations } => self.edit_by_voice(summary, operations, now_ms),
            VoiceAction::Subtitles { .. }
            | VoiceAction::SubtitleLanguage { .. }
            | VoiceAction::Look
            | VoiceAction::Room { .. } => Done::said("only Sceneuron's AI can do that".to_owned(), false),
        }
    }
}
