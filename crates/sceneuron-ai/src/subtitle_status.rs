// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What subtitles report to the engine: whether they listen and why not, and how the overlay shows
//! them, from their settings and what the voice input is doing.

use sceneuron_core::SubtitleStatus;
use sceneuron_voice::VoiceStatus as InputStatus;

use crate::subtitle_settings::SubtitleSettings;

const BUDGET_SPENT: &str = "the hourly budget is spent; subtitles resume when the budget allows";

impl SubtitleSettings {
    /// The status reported to the engine, given whether an ElevenLabs key is set, what the voice
    /// input is doing, whether the hourly budget is spent, and the spoken language in use. The
    /// language detected is the voice input's to add.
    pub fn status(&self, has_key: bool, input: &InputStatus, budget_spent: bool, language: &str) -> SubtitleStatus {
        let (state, message) = if !self.enabled {
            ("off", "subtitles are off".to_owned())
        } else if !self.consent {
            (
                "needs_consent",
                "confirm that microphone audio may be sent to ElevenLabs for subtitles".to_owned(),
            )
        } else if !has_key {
            ("needs_key", "add an ElevenLabs key to show subtitles".to_owned())
        } else if budget_spent {
            ("paused", BUDGET_SPENT.to_owned())
        } else {
            match input {
                InputStatus::Listening => ("listening", input.describe()),
                InputStatus::Retrying { .. } => ("retrying", input.describe()),
                InputStatus::Stopped { .. } => ("stopped", input.describe()),
                _ => ("connecting", InputStatus::Connecting.describe()),
            }
        };
        SubtitleStatus {
            enabled: self.active(has_key) && !budget_spent,
            state: state.to_owned(),
            message,
            language: language.to_owned(),
            detected: String::new(),
            targets: self.targets.clone(),
            lines: self.lines,
            clear_after_ms: self.clear_after_ms,
            hide_commands: self.hide_commands,
            stream_captions: self.stream_captions,
            stream_language: self.stream_language.clone(),
            labels: self.labels,
            latin_targets: self.latin_targets.clone(),
            latin_lines: self.latin_lines,
        }
    }
}
