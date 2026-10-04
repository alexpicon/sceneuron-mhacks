// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Subtitle settings: live subtitles of the streamer's words through ElevenLabs speech to text,
//! and translations of each final line through the configured AI provider. They live in `ai.json`
//! with the other AI settings.

use serde::{Deserialize, Serialize};

use sceneuron_core::{
    DEFAULT_CLEAR_AFTER_MS, DEFAULT_LINES, MAX_LINES, MAX_TARGETS, canonical_language_tag, constructed_language,
    language_label, valid_language_tag,
};

use crate::provider::AiError;
use crate::provider_kind::DEFAULT_TRANSLATION_MODEL;
use crate::settings::check_model;

/// How long lines may stay after they last changed, in milliseconds.
const CLEAR_AFTER_MS: (u64, u64) = (1_000, 60_000);
const CONSENT_FIRST: &str = "confirm that microphone audio may be sent to ElevenLabs for subtitles first";

/// How Sceneuron shows what the streamer says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SubtitleSettings {
    /// Subtitles keep a speech-to-text session open the whole time they are on.
    pub enabled: bool,
    /// The streamer confirmed that microphone audio may be sent to ElevenLabs, and when targets are
    /// set, final lines to the AI provider.
    pub consent: bool,
    /// The spoken language as an ISO 639-1 or 639-3 code, or empty to detect it; never a
    /// constructed or fictional language, which speech to text does not hear.
    pub language: String,
    /// Up to three BCP 47 codes each final line is translated into, such as `es`, `pt-BR`, `qya`
    /// for Quenya, or the private-use `art-x-dothraki` for a language without a code.
    pub targets: Vec<String>,
    /// The model that translates, on the provider the AI settings choose, with its key.
    pub translation_model: String,
    /// Lines each language shows, 1 to 3.
    pub lines: u8,
    /// How long lines stay after they last changed, in milliseconds.
    pub clear_after_ms: u64,
    /// Speech addressed to Sceneuron, a command, is left out while voice control is on.
    pub hide_commands: bool,
    /// Final lines go to the stream as CEA-608 closed captions while streaming.
    pub stream_captions: bool,
    /// The captions' language: empty for the spoken one, or one of the targets.
    pub stream_language: String,
    /// Each text that shows subtitles on the overlay names their language in a small label.
    pub labels: bool,
    /// Targets with a script of their own, such as Quenya's Tengwar, that the overlay shows in
    /// Latin letters instead; captions always do.
    pub latin_targets: Vec<String>,
    /// Under each line the overlay writes in a script of its own, the same line in small Latin
    /// letters.
    pub latin_lines: bool,
}

impl Default for SubtitleSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            consent: false,
            language: String::new(),
            targets: Vec::new(),
            translation_model: DEFAULT_TRANSLATION_MODEL.to_owned(),
            lines: DEFAULT_LINES,
            clear_after_ms: DEFAULT_CLEAR_AFTER_MS,
            hide_commands: true,
            stream_captions: false,
            stream_language: String::new(),
            labels: true,
            latin_targets: Vec::new(),
            latin_lines: false,
        }
    }
}

/// A partial change to the subtitle settings; unset fields keep their value.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubtitleSettingsUpdate {
    pub enabled: Option<bool>,
    pub consent: Option<bool>,
    pub language: Option<String>,
    pub targets: Option<Vec<String>>,
    pub translation_model: Option<String>,
    pub lines: Option<u8>,
    pub clear_after_ms: Option<u64>,
    pub hide_commands: Option<bool>,
    pub stream_captions: Option<bool>,
    pub stream_language: Option<String>,
    pub labels: Option<bool>,
    pub latin_targets: Option<Vec<String>>,
    pub latin_lines: Option<bool>,
}

/// The subtitle settings with the prices their translations are estimated at.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SubtitleSettingsView {
    #[serde(flatten)]
    pub settings: SubtitleSettings,
    /// The translation model has a known or configured price; when not, its spend counts as zero.
    pub translation_price_known: bool,
    /// US dollars per million input tokens of translations; 0 when unknown.
    pub translation_input_price_per_mtok: f64,
    /// US dollars per million output tokens of translations; 0 when unknown.
    pub translation_output_price_per_mtok: f64,
}

impl SubtitleSettings {
    /// Checks every value; the error says what to fix.
    pub fn validate(&self) -> Result<(), AiError> {
        let config = |message: String| Err(AiError::Config(message));
        let language = &self.language;
        if !(language.is_empty()
            || ((2..=3).contains(&language.len()) && language.bytes().all(|b| b.is_ascii_lowercase())))
        {
            return config(
                "the spoken language must be empty or an ISO 639-1 or 639-3 code such as en or spa".to_owned(),
            );
        }
        if constructed_language(language) {
            return config(format!(
                "speech to text does not hear {}; choose it as a language to translate into instead",
                language_label(language)
            ));
        }
        if self.targets.len() > MAX_TARGETS {
            return config(format!(
                "subtitles can be translated into at most {MAX_TARGETS} languages"
            ));
        }
        for (index, target) in self.targets.iter().enumerate() {
            if !valid_language_tag(target) {
                return config(format!(
                    "{target:?} is not a language code such as es, pt-BR, zh-Hans, or art-x-dothraki"
                ));
            }
            if self.targets[..index]
                .iter()
                .any(|other| other.eq_ignore_ascii_case(target))
            {
                return config(format!("{target} is listed twice"));
            }
        }
        check_model("translation_model", &self.translation_model)?;
        if !(1..=MAX_LINES).contains(&self.lines) {
            return config(format!("subtitles show 1 to {MAX_LINES} lines"));
        }
        let (min, max) = CLEAR_AFTER_MS;
        if !(min..=max).contains(&self.clear_after_ms) {
            return config(format!("clear_after_ms must be {min} to {max} milliseconds"));
        }
        if !(self.stream_language.is_empty() || self.targets.contains(&self.stream_language)) {
            return config("stream captions must be in the spoken language or one of the targets".to_owned());
        }
        if self.enabled && !self.consent {
            return config(CONSENT_FIRST.to_owned());
        }
        Ok(())
    }

    /// Drops the languages an older Sceneuron accepted and this one refuses, so the rest of a saved
    /// file still loads: malformed targets, captions in one, and a constructed spoken language.
    /// Kept targets take their usual case. Returns what was dropped, in words, if anything was.
    pub fn repair(&mut self) -> Option<String> {
        let (kept, refused): (Vec<String>, Vec<String>) = std::mem::take(&mut self.targets)
            .into_iter()
            .partition(|code| valid_language_tag(code));
        self.targets = kept.iter().filter_map(|code| canonical_language_tag(code)).collect();
        let mut dropped: Vec<String> = refused
            .iter()
            .map(|code| format!("{code:?} is not a language code, so subtitles no longer translate into it"))
            .collect();
        if refused.contains(&self.stream_language) {
            self.stream_language.clear();
            dropped.push("captions are in the spoken language instead".to_owned());
        } else if let Some(stream) = canonical_language_tag(&self.stream_language) {
            self.stream_language = stream;
        }
        if constructed_language(&self.language) {
            let heard = language_label(&self.language);
            dropped.push(format!(
                "speech to text does not hear {heard}, so subtitles detect the spoken language"
            ));
            self.language.clear();
        }
        (!dropped.is_empty()).then(|| dropped.join("; "))
    }

    /// These settings with `update` applied: codes in their usual case; when a target is removed,
    /// stream captions in it go back to the spoken language and its choice of Latin letters is
    /// dropped. Withdrawing consent also turns subtitles off. The caller validates the result.
    pub fn updated(&self, update: &SubtitleSettingsUpdate) -> SubtitleSettings {
        let mut next = self.clone();
        let text = |value: &Option<String>, target: &mut String| {
            if let Some(value) = value {
                value.trim().clone_into(target);
            }
        };
        text(&update.language, &mut next.language);
        text(&update.translation_model, &mut next.translation_model);
        text(&update.stream_language, &mut next.stream_language);
        if let Some(targets) = &update.targets {
            let codes = targets.iter().map(|code| code.trim()).filter(|code| !code.is_empty());
            next.targets = codes
                .map(|code| canonical_language_tag(code).unwrap_or_else(|| code.to_owned()))
                .collect();
        }
        if let Some(stream) = canonical_language_tag(&next.stream_language) {
            next.stream_language = stream;
        }
        if update.stream_language.is_none() && !next.targets.contains(&next.stream_language) {
            next.stream_language.clear();
        }
        if let Some(latin) = &update.latin_targets {
            let codes = latin.iter().map(|code| code.trim());
            next.latin_targets = codes
                .map(|code| canonical_language_tag(code).unwrap_or_else(|| code.to_owned()))
                .collect();
        }
        // A target's choice of Latin letters goes with it.
        next.latin_targets.retain(|code| next.targets.contains(code));
        next.labels = update.labels.unwrap_or(next.labels);
        next.latin_lines = update.latin_lines.unwrap_or(next.latin_lines);
        next.lines = update.lines.unwrap_or(next.lines);
        next.clear_after_ms = update.clear_after_ms.unwrap_or(next.clear_after_ms);
        next.hide_commands = update.hide_commands.unwrap_or(next.hide_commands);
        next.stream_captions = update.stream_captions.unwrap_or(next.stream_captions);
        next.consent = update.consent.unwrap_or(next.consent);
        if update.consent == Some(false) && update.enabled.is_none() {
            next.enabled = false;
        }
        next.enabled = update.enabled.unwrap_or(next.enabled);
        next
    }

    /// Whether subtitles may listen: on, agreed to, and with an ElevenLabs key.
    pub fn active(&self, has_key: bool) -> bool {
        self.enabled && self.consent && has_key
    }
}
