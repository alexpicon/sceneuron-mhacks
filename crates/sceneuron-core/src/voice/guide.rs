// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What the streamer can say, for the editor: ways to say each command, built from the tables the
//! matcher reads (see [`commands`](super::commands)), the reaction presets, and the project (see
//! [`guide_entries`](super::guide_entries)). Each way is read back by the matcher and listed only
//! when it does what its entry says, so the guide cannot promise what the matcher would not do,
//! such as a reaction named like a card.

use serde::Serialize;

use super::VoiceMode;
use super::matcher::{Intent, interpret};
use crate::engine::Engine;

/// Requests for the AI, which no local command takes.
const ANYTHING: [&str; 2] = [
    "make the name label say Back in five",
    "put a red border around the camera frame",
];

/// What can be said now, by kind.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VoiceGuide {
    pub wake_word: String,
    pub mode: VoiceMode,
    pub groups: Vec<GuideGroup>,
}

/// Commands of one kind.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GuideGroup {
    pub title: String,
    /// Said to Sceneuron, after the wake word or with push to talk held; otherwise anywhere in
    /// what is said, in always-on mode.
    pub addressed: bool,
    pub entries: Vec<GuideEntry>,
}

/// One command: ways to say it, most natural first, and what it does. An entry without ways
/// says what to do first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GuideEntry {
    pub say: Vec<String>,
    pub does: String,
}

impl Engine {
    /// What can be said now: reactions, trigger phrases, cards, themes, elements, the camera,
    /// subtitles, and automation, and anything else for the AI.
    pub fn voice_guide(&self) -> VoiceGuide {
        let asks = |intent: &Intent| matches!(intent, Intent::Ask { .. });
        let anything = self.entry(&ANYTHING, "goes to the AI, which carries it out", asks);
        let groups = [
            ("Reactions", true, self.reaction_entries()),
            ("Trigger phrases", false, self.phrase_entries()),
            ("Cards", true, self.card_entries()),
            ("Themes", true, self.theme_entries()),
            ("Overlay elements", true, self.element_entries()),
            ("Camera, subtitles, and automation", true, self.other_entries()),
            ("Anything else", true, anything.into_iter().collect()),
        ];
        let status = &self.voice.status;
        VoiceGuide {
            wake_word: status.wake_word.clone(),
            mode: status.mode,
            groups: groups
                .into_iter()
                .filter(|(_, _, entries)| !entries.is_empty())
                .map(|(title, addressed, entries)| GuideGroup {
                    title: title.to_owned(),
                    addressed,
                    entries,
                })
                .collect(),
        }
    }

    /// An entry with the ways in `say` that the matcher reads as `expected` when said to
    /// Sceneuron; `None` when none is.
    pub(super) fn entry<S: AsRef<str>>(
        &self,
        say: &[S],
        does: &str,
        expected: impl Fn(&Intent) -> bool,
    ) -> Option<GuideEntry> {
        self.entry_heard(say, does, true, expected)
    }

    /// [`Engine::entry`] for words said to Sceneuron or, unless `addressed`, anywhere in
    /// always-on mode.
    pub(super) fn entry_heard<S: AsRef<str>>(
        &self,
        say: &[S],
        does: &str,
        addressed: bool,
        expected: impl Fn(&Intent) -> bool,
    ) -> Option<GuideEntry> {
        let wake_word = &self.voice.status.wake_word;
        let mut kept: Vec<String> = Vec::new();
        for way in say.iter().map(AsRef::as_ref) {
            let intent = interpret(way, addressed, VoiceMode::AlwaysOn, wake_word, &self.project);
            if expected(&intent) && !kept.iter().any(|known| known == way) {
                kept.push(way.to_owned());
            }
        }
        (!kept.is_empty()).then(|| GuideEntry {
            say: kept,
            does: does.to_owned(),
        })
    }
}
