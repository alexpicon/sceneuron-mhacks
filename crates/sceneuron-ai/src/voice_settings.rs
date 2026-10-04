// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Voice settings: voice control through ElevenLabs speech to text, and spoken cards through
//! ElevenLabs text to speech. They live in `ai.json` with the other AI settings; the ElevenLabs
//! key is kept with the other keys.

use serde::{Deserialize, Serialize};

use sceneuron_core::{DEFAULT_WAKE_WORD, VoiceMode, VoiceStatus, normalize_speech};
use sceneuron_voice::VoiceStatus as InputStatus;

use crate::provider::AiError;
use crate::voice_cost::{
    keyterm_rate_per_hour, stt_price_per_hour, stt_rate_per_hour, tts_price_per_1k_chars, tts_rate_per_1k_chars,
};

/// A premade voice from the ElevenLabs text to speech quickstart
/// (https://elevenlabs.io/docs/quickstart): "George". Accounts created after March 2026 lack it,
/// so the speaker then speaks in a voice the account has.
pub const DEFAULT_VOICE_ID: &str = "JBFqnCBsd6RMkjVDRZzb";
/// ElevenLabs' low-latency text to speech model (https://elevenlabs.io/docs/models).
pub const DEFAULT_TTS_MODEL: &str = "eleven_flash_v2_5";
/// ElevenLabs' realtime speech to text model (https://elevenlabs.io/docs/models).
pub const DEFAULT_STT_MODEL: &str = "scribe_v2_realtime";
const WAKE_WORD_CHARS: (usize, usize) = (2, 30);
const MAX_VOICE_ID_CHARS: usize = 64;
const MAX_MODEL_CHARS: usize = 100;
const CONSENT_FIRST: &str = "confirm that microphone audio may be sent to ElevenLabs first";

/// How Sceneuron listens and speaks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct VoiceSettings {
    /// Voice control: microphone audio is transcribed while push-to-talk is held, or all the
    /// time in always-on mode.
    pub enabled: bool,
    /// The streamer confirmed that microphone audio may be sent to ElevenLabs.
    pub consent: bool,
    pub mode: VoiceMode,
    pub wake_word: String,
    /// The spoken language as an ISO 639-1 or 639-3 code, or empty to detect it.
    pub language: String,
    /// Spoken cards: raised cards of decks that speak are said aloud in `voice_id`.
    pub speak: bool,
    pub voice_id: String,
    pub tts_model: String,
    pub stt_model: String,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            consent: false,
            mode: VoiceMode::PushToTalk,
            wake_word: DEFAULT_WAKE_WORD.to_owned(),
            language: String::new(),
            speak: false,
            voice_id: DEFAULT_VOICE_ID.to_owned(),
            tts_model: DEFAULT_TTS_MODEL.to_owned(),
            stt_model: DEFAULT_STT_MODEL.to_owned(),
        }
    }
}

/// A partial change to the voice settings; unset fields keep their value.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceSettingsUpdate {
    pub enabled: Option<bool>,
    pub consent: Option<bool>,
    pub mode: Option<VoiceMode>,
    pub wake_word: Option<String>,
    pub language: Option<String>,
    pub speak: Option<bool>,
    pub voice_id: Option<String>,
    pub tts_model: Option<String>,
    pub stt_model: Option<String>,
}

/// The voice settings with the prices their spend is estimated at.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VoiceSettingsView {
    #[serde(flatten)]
    pub settings: VoiceSettings,
    /// US dollars per 1,000 characters of spoken cards.
    pub tts_price_per_1k_chars: f64,
    /// The speech model has a list price; when not, the highest known one is counted.
    pub tts_price_listed: bool,
    /// US dollars per hour of transcribed audio, keyterms aside.
    pub stt_price_per_hour: f64,
    /// The listening model has a list price; when not, the highest known one is counted.
    pub stt_price_listed: bool,
    /// US dollars per hour of audio sent with keyterms, the wake word and the card words that
    /// help recognition, on top of the listening price.
    pub keyterm_price_per_hour: f64,
}

impl VoiceSettings {
    /// Checks every value; the error says what to fix.
    pub fn validate(&self) -> Result<(), AiError> {
        let count = self.wake_word.chars().count();
        let allowed = |c: char| c.is_alphanumeric() || " '-".contains(c);
        let (min, max) = WAKE_WORD_CHARS;
        if !(min..=max).contains(&count)
            || !self.wake_word.chars().all(allowed)
            || normalize_speech(&self.wake_word).is_empty()
        {
            let message = format!("the wake word must be {min}-{max} letters, digits, spaces, apostrophes, or hyphens");
            return Err(AiError::Config(message));
        }
        let language = &self.language;
        if !(language.is_empty()
            || ((2..=3).contains(&language.len()) && language.bytes().all(|b| b.is_ascii_lowercase())))
        {
            let message = "the language must be empty or an ISO 639-1 or 639-3 code such as en or spa";
            return Err(AiError::Config(message.to_owned()));
        }
        let id = &self.voice_id;
        if id.is_empty() || id.len() > MAX_VOICE_ID_CHARS || !id.bytes().all(|b| b.is_ascii_alphanumeric()) {
            let message = format!("the voice id must be 1 to {MAX_VOICE_ID_CHARS} letters and digits");
            return Err(AiError::Config(message));
        }
        check_model("tts_model", &self.tts_model)?;
        check_model("stt_model", &self.stt_model)?;
        if self.enabled && !self.consent {
            return Err(AiError::Config(CONSENT_FIRST.to_owned()));
        }
        Ok(())
    }

    /// These settings with `update` applied; withdrawing consent also turns voice control off.
    /// The caller validates the result.
    pub fn updated(&self, update: &VoiceSettingsUpdate) -> VoiceSettings {
        let mut next = self.clone();
        let text = |value: &Option<String>, target: &mut String| {
            if let Some(value) = value {
                value.trim().clone_into(target);
            }
        };
        text(&update.wake_word, &mut next.wake_word);
        text(&update.language, &mut next.language);
        text(&update.voice_id, &mut next.voice_id);
        text(&update.tts_model, &mut next.tts_model);
        text(&update.stt_model, &mut next.stt_model);
        next.mode = update.mode.unwrap_or(next.mode);
        next.speak = update.speak.unwrap_or(next.speak);
        next.consent = update.consent.unwrap_or(next.consent);
        if update.consent == Some(false) && update.enabled.is_none() {
            next.enabled = false;
        }
        next.enabled = update.enabled.unwrap_or(next.enabled);
        next
    }

    /// What the editor may see, with the prices of the chosen models.
    pub fn view(&self) -> VoiceSettingsView {
        VoiceSettingsView {
            settings: self.clone(),
            tts_price_per_1k_chars: tts_rate_per_1k_chars(&self.tts_model),
            tts_price_listed: tts_price_per_1k_chars(&self.tts_model).is_some(),
            stt_price_per_hour: stt_rate_per_hour(&self.stt_model),
            stt_price_listed: stt_price_per_hour(&self.stt_model).is_some(),
            keyterm_price_per_hour: keyterm_rate_per_hour(),
        }
    }

    /// The status reported to the engine, given whether an ElevenLabs key is set and what the
    /// voice input is doing.
    pub fn status(&self, has_key: bool, input: &InputStatus) -> VoiceStatus {
        let message = if !self.enabled {
            "voice control is off".to_owned()
        } else if !has_key {
            "add an ElevenLabs key to use voice control".to_owned()
        } else {
            input.describe()
        };
        VoiceStatus {
            enabled: self.enabled,
            mode: self.mode,
            wake_word: self.wake_word.clone(),
            listening: self.enabled && has_key && input.is_listening(),
            message,
        }
    }
}

/// Model ids go into request bodies and addresses, so they are kept to plain characters.
fn check_model(field: &str, model: &str) -> Result<(), AiError> {
    let plain = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b);
    if (1..=MAX_MODEL_CHARS).contains(&model.len()) && model.bytes().all(plain) {
        Ok(())
    } else {
        let message = format!("{field} must be a model id of 1 to {MAX_MODEL_CHARS} characters of a-z, 0-9, _, . or -");
        Err(AiError::Config(message))
    }
}
