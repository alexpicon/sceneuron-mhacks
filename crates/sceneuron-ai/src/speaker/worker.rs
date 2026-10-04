// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The speaker thread's work: which voice speaks, and turning one cue into cached audio.
//!
//! The voice is checked against the account once a key is set and spoken cards are on, and
//! again when ElevenLabs says the voice was not found. The default voice is replaced by one
//! the account has, since new accounts lack it; a voice the streamer chose is never replaced,
//! and when it is missing every card says so instead of being spoken.

use std::sync::{Arc, PoisonError};

use super::{Job, Shared};
use crate::director::{Ledger, Sink};
use crate::provider::AiError;
use crate::settings::AiSettings;
use crate::speech_cache::SpeechCache;
use crate::tts::{TextToSpeech, missing_voice};
use crate::voice_cost::{VoiceUsage, voice_cost_usd};
use crate::voice_settings::DEFAULT_VOICE_ID;

/// Which voice speaks the cards.
#[derive(Clone, Debug, PartialEq)]
enum Voice {
    /// Not checked against the account since the key or voice changed.
    Unchecked,
    Use(String),
    /// The account cannot speak in the chosen voice; the message says what to do.
    Missing(String),
}

pub(super) struct Worker {
    shared: Arc<Shared>,
    cache: SpeechCache,
    base_url: String,
    ledger: Ledger,
    sink: Arc<dyn Sink>,
    settings: AiSettings,
    client: Option<TextToSpeech>,
    voice: Voice,
    /// Said since the settings last changed why cards cannot be spoken.
    warned: bool,
}

impl Worker {
    pub(super) fn new(
        shared: Arc<Shared>,
        cache: SpeechCache,
        base_url: &str,
        ledger: Ledger,
        sink: Arc<dyn Sink>,
    ) -> Worker {
        Worker {
            shared,
            cache,
            base_url: base_url.to_owned(),
            ledger,
            sink,
            settings: AiSettings::default(),
            client: None,
            voice: Voice::Unchecked,
            warned: false,
        }
    }

    pub(super) fn run(mut self) {
        loop {
            let (settings, job) = {
                let mut inbox = self.shared.inbox.lock().unwrap_or_else(PoisonError::into_inner);
                while !inbox.shutdown && inbox.settings.is_none() && inbox.job.is_none() {
                    inbox = self.shared.wake.wait(inbox).unwrap_or_else(PoisonError::into_inner);
                }
                if inbox.shutdown {
                    return;
                }
                (inbox.settings.take(), inbox.job.take())
            };
            if let Some(settings) = settings {
                self.apply(settings);
            }
            // Checked as soon as a key is set, so the first card does not wait for it.
            if self.voice == Voice::Unchecked && self.settings.voice.speak && self.client.is_some() {
                self.check_voice();
            }
            if let Some(job) = job {
                self.speak(&job);
            }
        }
    }

    pub(super) fn apply(&mut self, settings: AiSettings) {
        let key = &settings.elevenlabs_api_key;
        let changed =
            *key != self.settings.elevenlabs_api_key || settings.voice.voice_id != self.settings.voice.voice_id;
        self.client = (!key.is_empty()).then(|| TextToSpeech::new(&self.base_url, key));
        if changed {
            self.voice = Voice::Unchecked;
        }
        self.settings = settings;
        self.warned = false;
    }

    /// Asks the account which voices it has and decides which one speaks. When the list cannot
    /// be read, the chosen voice is tried as it is.
    fn check_voice(&mut self) {
        let Some(client) = &self.client else {
            return;
        };
        let wanted = self.settings.voice.voice_id.clone();
        self.voice = match client.voices() {
            Ok(list) if list.contains(&wanted) || list.has_more => Voice::Use(wanted),
            Ok(list) if wanted == DEFAULT_VOICE_ID => match list.replacement() {
                Some(voice) => {
                    self.sink.voice_note(&format!(
                        "spoken cards use the voice {} ({}) from your ElevenLabs account, which lacks the default \
                         voice; choose another in the AI tab",
                        voice.name, voice.voice_id
                    ));
                    Voice::Use(voice.voice_id.clone())
                }
                None => Voice::Missing("your ElevenLabs account has no voices; add one in ElevenLabs".to_owned()),
            },
            Ok(_) => Voice::Missing(missing_voice(&wanted).message().to_owned()),
            Err(_) => Voice::Use(wanted),
        };
    }

    fn speak(&mut self, job: &Job) {
        if !self.settings.voice.speak {
            return;
        }
        if self.client.is_none() {
            self.warn("add an ElevenLabs key in the AI tab to speak cards");
            return;
        }
        if self.voice == Voice::Unchecked {
            self.check_voice();
        }
        // A voice the account lost since it was checked is checked again, and the card is tried
        // once more when that finds another voice to use.
        for retry in [false, true] {
            let voice = match &self.voice {
                Voice::Use(voice) => voice.clone(),
                Voice::Missing(message) => {
                    let message = format!("cards are not spoken: {message}");
                    return self.warn(&message);
                }
                Voice::Unchecked => return,
            };
            let model = self.settings.voice.tts_model.clone();
            let id = SpeechCache::id(&job.text, &voice, &model);
            if self.cache.contains(&id) {
                return self.sink.spoken(job.nonce, &id);
            }
            if self.ledger.over_budget() {
                return self
                    .sink
                    .speech_failed("a card was not spoken: the hourly budget is spent");
            }
            match self.generate(&job.text, &voice, &model) {
                Ok(audio) => return self.keep(job.nonce, &id, &audio),
                Err(error) if error == missing_voice(&voice) && !retry => {
                    self.check_voice();
                    if !matches!(&self.voice, Voice::Use(other) if *other != voice) {
                        return self.sink.speech_failed(&format!("a card was not spoken: {error}"));
                    }
                }
                Err(error) => return self.sink.speech_failed(&format!("a card was not spoken: {error}")),
            }
        }
    }

    fn keep(&self, nonce: u64, id: &str, audio: &[u8]) {
        match self.cache.put(id, audio) {
            Ok(()) => self.sink.spoken(nonce, id),
            Err(error) => {
                let folder = self.cache.dir().display();
                self.sink
                    .speech_failed(&format!("cannot keep a spoken card in {folder}: {error}"));
            }
        }
    }

    /// One text to speech request; its characters count toward the totals and the budget.
    fn generate(&self, text: &str, voice: &str, model: &str) -> Result<Vec<u8>, AiError> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| AiError::Config("no key".to_owned()))?;
        let audio = client.speak(text, voice, model)?;
        let usage = VoiceUsage {
            characters: text.chars().count() as u64,
            ..VoiceUsage::default()
        };
        let cost = voice_cost_usd(&usage, model, &self.settings.voice.stt_model);
        self.ledger.record_voice(usage, cost);
        Ok(audio)
    }

    /// Says why cards cannot be spoken, once per settings change.
    fn warn(&mut self, message: &str) {
        if !self.warned {
            self.warned = true;
            self.sink.speech_failed(message);
        }
    }
}
