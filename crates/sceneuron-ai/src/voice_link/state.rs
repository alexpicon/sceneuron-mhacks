// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What the voice link knows: the voice and subtitle settings, the key, the keyterms, what the
//! voice input is doing and the language it last detected, and whether the hourly budget is
//! spent; and the input's configuration and the statuses that follow from them.
//!
//! Subtitles keep a stream open all the time they are on, whatever voice control's mode, and the
//! same stream serves voice control in its own mode: in push-to-talk mode only held speech is a
//! command, and in always-on mode speech that starts with the wake word is one too.

use sceneuron_core::{SubtitleStatus, VoiceMode, VoiceStatus};
use sceneuron_voice::{ApiKey, Mode, VoiceConfig, VoiceStatus as InputStatus, keyterms};

use crate::elevenlabs::ElevenLabs;
use crate::settings::AiSettings;
use crate::subtitle_settings::SubtitleSettings;
use crate::voice_settings::VoiceSettings;

const BUDGET_SPENT: &str = "the hourly budget is spent; voice control resumes when the budget allows";

pub(super) struct LinkState {
    pub(super) settings: VoiceSettings,
    pub(super) subtitles: SubtitleSettings,
    pub(super) key: String,
    /// Card names, then trigger phrases, of decks whose voice is on.
    pub(super) terms: Vec<String>,
    realtime_url: String,
    pub(super) input: InputStatus,
    /// The language speech to text detected last, by the code it reports, which subtitles name
    /// while the spoken language is left to detection.
    pub(super) detected: String,
    /// Whether the hourly budget is spent, which stops listening.
    pub(super) budget_spent: bool,
    /// Counts changes, so a status computed before a later change is followed by a fresh one.
    pub(super) version: u64,
}

impl LinkState {
    pub(super) fn new(settings: &AiSettings, elevenlabs: &ElevenLabs, budget_spent: bool) -> LinkState {
        LinkState {
            settings: settings.voice.clone(),
            subtitles: settings.subtitles.clone(),
            key: settings.elevenlabs_api_key.clone(),
            terms: Vec::new(),
            realtime_url: elevenlabs.realtime_url.clone(),
            input: InputStatus::Off,
            detected: String::new(),
            budget_spent,
            version: 0,
        }
    }

    /// Whether subtitles are on and agreed to, so the input streams all the time.
    fn subtitled(&self) -> bool {
        self.subtitles.enabled && self.subtitles.consent
    }

    /// The spoken language: the one set for subtitles while they are on, else voice control's.
    pub(super) fn language(&self) -> &str {
        if self.subtitled() && !self.subtitles.language.is_empty() {
            &self.subtitles.language
        } else {
            &self.settings.language
        }
    }

    pub(super) fn config(&self) -> VoiceConfig {
        let settings = &self.settings;
        let commands = settings.enabled && settings.consent;
        let mode = if self.subtitled() || settings.mode == VoiceMode::AlwaysOn {
            Mode::AlwaysOn
        } else {
            Mode::PushToTalk
        };
        // Keyterms help recognize commands, and cost extra, so subtitles alone send none. They
        // follow voice control's own mode, not the stream subtitles keep open: push to talk needs
        // no wake word, so without cards that voice can raise it sends none.
        let subtitles_only = self.subtitled() && !commands;
        let keyterms = if subtitles_only || (settings.mode == VoiceMode::PushToTalk && self.terms.is_empty()) {
            Vec::new()
        } else {
            keyterms(&settings.wake_word, self.terms.iter().map(String::as_str))
        };
        VoiceConfig {
            enabled: (settings.enabled || self.subtitles.enabled) && !self.budget_spent,
            consent: commands || self.subtitled(),
            mode,
            wake_word: settings.wake_word.clone(),
            language: self.language().to_owned(),
            model: settings.stt_model.clone(),
            keyterms,
            endpoint: self.realtime_url.clone(),
            api_key: ApiKey::new(self.key.clone()),
        }
    }

    /// The settings' status, unless the spent budget is what keeps voice control from listening.
    pub(super) fn status(&self) -> VoiceStatus {
        let has_key = !self.key.is_empty();
        let mut status = self.settings.status(has_key, &self.input);
        if self.budget_spent && status.enabled && has_key {
            status.listening = false;
            BUDGET_SPENT.clone_into(&mut status.message);
        }
        status
    }

    /// How subtitles are set up and what they are doing, with the language detected while the
    /// spoken one is left to detection.
    pub(super) fn subtitle_status(&self) -> SubtitleStatus {
        let has_key = !self.key.is_empty();
        let mut status = self
            .subtitles
            .status(has_key, &self.input, self.budget_spent, self.language());
        if self.language().is_empty() {
            self.detected.clone_into(&mut status.detected);
        }
        status
    }
}
