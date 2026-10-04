// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The simulated voice settings, kept in memory and checked like the real ones.
//!
//! No ElevenLabs key is stored: a key the editor sends only leaves its last four characters as a
//! hint, and the demo starts with a fake key so the editor shows the "key saved" state.

use sceneuron_core::{DEFAULT_WAKE_WORD, VoiceMode, VoiceStatus};
use sceneuron_server::AssistantError;
use serde_json::{Map, Value, json};

const MODES: [&str; 2] = ["push_to_talk", "always_on"];
const MAX_TEXT_CHARS: usize = 100;
const SIMULATED: &str = "Simulated voice: no microphone is used and nothing is sent";

/// Voice settings as the editor sees them through `GET /api/ai`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DemoVoice {
    pub(crate) enabled: bool,
    pub(crate) consent: bool,
    pub(crate) mode: VoiceMode,
    pub(crate) wake_word: String,
    pub(crate) language: String,
    pub(crate) speak: bool,
    pub(crate) voice_id: String,
    pub(crate) tts_model: String,
    pub(crate) stt_model: String,
    /// Last characters of the saved key; `None` when no key is saved.
    pub(crate) key_hint: Option<String>,
}

impl Default for DemoVoice {
    fn default() -> Self {
        Self {
            enabled: false,
            consent: false,
            mode: VoiceMode::PushToTalk,
            wake_word: DEFAULT_WAKE_WORD.to_owned(),
            language: String::new(),
            speak: false,
            voice_id: "simulated-voice".to_owned(),
            tts_model: "simulated-speech".to_owned(),
            stt_model: "simulated-listening".to_owned(),
            key_hint: Some("demo".to_owned()),
        }
    }
}

impl DemoVoice {
    /// Adds the voice settings and the key's hint to the AI settings JSON.
    pub(crate) fn add_to(&self, settings: &mut Value) {
        let Some(fields) = settings.as_object_mut() else {
            return;
        };
        let mode = if self.mode == VoiceMode::AlwaysOn {
            MODES[1]
        } else {
            MODES[0]
        };
        let voice = json!({
            "enabled": self.enabled,
            "consent": self.consent,
            "mode": mode,
            "wake_word": self.wake_word,
            "language": self.language,
            "speak": self.speak,
            "voice_id": self.voice_id,
            "tts_model": self.tts_model,
            "stt_model": self.stt_model,
            "tts_price_per_1k_chars": 0.0,
            "stt_price_per_hour": 0.0,
        });
        fields.insert("voice".to_owned(), voice);
        fields.insert("has_elevenlabs_key".to_owned(), json!(self.key_hint.is_some()));
        let hint = self.key_hint.clone().unwrap_or_default();
        fields.insert("elevenlabs_key_hint".to_owned(), json!(hint));
        let source = if self.key_hint.is_some() { "editor" } else { "" };
        fields.insert("elevenlabs_key_source".to_owned(), json!(source));
    }

    /// Applies the `voice` object and `elevenlabs_api_key` of an update to a copy, or refuses.
    /// Withdrawing consent also turns voice control off, which cannot be on without it.
    pub(crate) fn updated(&self, update: &Map<String, Value>) -> Result<DemoVoice, AssistantError> {
        let mut next = self.clone();
        if let Some(key) = update.get("elevenlabs_api_key") {
            next.key_hint = key_hint(key)?;
        }
        let Some(voice) = update.get("voice") else {
            return Ok(next);
        };
        let fields = voice.as_object().ok_or_else(|| refuse("voice must be a JSON object"))?;
        for (key, value) in fields {
            next.set(key, value)?;
        }
        if fields.get("consent") == Some(&Value::Bool(false)) && !fields.contains_key("enabled") {
            next.enabled = false;
        }
        if next.enabled && !next.consent {
            return Err(refuse("confirm that microphone audio may be sent to ElevenLabs first"));
        }
        Ok(next)
    }

    /// The status the engine shows; it always says that voice is simulated.
    pub(crate) fn status(&self) -> VoiceStatus {
        VoiceStatus {
            enabled: self.enabled,
            mode: self.mode,
            wake_word: self.wake_word.clone(),
            listening: false,
            message: if self.enabled {
                SIMULATED
            } else {
                "voice control is off"
            }
            .to_owned(),
        }
    }

    fn set(&mut self, key: &str, value: &Value) -> Result<(), AssistantError> {
        let flag = || {
            value
                .as_bool()
                .ok_or_else(|| refuse(&format!("voice.{key} must be true or false")))
        };
        match key {
            "enabled" => self.enabled = flag()?,
            "consent" => self.consent = flag()?,
            "speak" => self.speak = flag()?,
            "mode" => {
                self.mode = match value.as_str() {
                    Some("push_to_talk") => VoiceMode::PushToTalk,
                    Some("always_on") => VoiceMode::AlwaysOn,
                    _ => return Err(refuse("voice.mode must be push_to_talk or always_on")),
                };
            }
            "wake_word" => self.wake_word = text(key, value, 2)?,
            "language" => {
                let language = text(key, value, 0)?;
                let code = (2..=3).contains(&language.len()) && language.bytes().all(|b| b.is_ascii_lowercase());
                if !(language.is_empty() || code) {
                    return Err(refuse("voice.language must be empty or a code such as en or spa"));
                }
                self.language = language;
            }
            "voice_id" => self.voice_id = text(key, value, 1)?,
            "tts_model" => self.tts_model = text(key, value, 1)?,
            "stt_model" => self.stt_model = text(key, value, 1)?,
            _ => return Err(refuse(&format!("unknown voice setting {key:?}"))),
        }
        Ok(())
    }
}

fn refuse(message: &str) -> AssistantError {
    AssistantError::new(400, message)
}

fn text(key: &str, value: &Value, min: usize) -> Result<String, AssistantError> {
    let text = value.as_str().map(str::trim).unwrap_or_default();
    let count = text.chars().count();
    if value.is_string() && (min..=MAX_TEXT_CHARS).contains(&count) && !text.chars().any(char::is_control) {
        Ok(text.to_owned())
    } else {
        Err(refuse(&format!(
            "voice.{key} must be {min} to {MAX_TEXT_CHARS} characters on one line"
        )))
    }
}

/// An empty key removes the saved one; any other key is reduced to its last four characters.
fn key_hint(value: &Value) -> Result<Option<String>, AssistantError> {
    let key = value
        .as_str()
        .ok_or_else(|| refuse("elevenlabs_api_key must be text"))?
        .trim();
    if key.is_empty() {
        return Ok(None);
    }
    if key.chars().count() > 400 || key.chars().any(char::is_control) {
        return Err(refuse("elevenlabs_api_key must be one line of at most 400 characters"));
    }
    let chars: Vec<char> = key.chars().collect();
    Ok(Some(chars[chars.len().saturating_sub(4)..].iter().collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update(value: Value) -> Result<DemoVoice, AssistantError> {
        DemoVoice::default().updated(value.as_object().unwrap())
    }

    #[test]
    fn voice_starts_off_with_a_fake_key_and_says_it_is_simulated() {
        let mut settings = json!({"available": true});
        DemoVoice::default().add_to(&mut settings);
        assert_eq!(settings["voice"]["enabled"], false);
        assert_eq!(settings["voice"]["mode"], "push_to_talk");
        assert_eq!(
            (
                settings["has_elevenlabs_key"].clone(),
                settings["elevenlabs_key_hint"].clone()
            ),
            (json!(true), json!("demo"))
        );
        let on = update(json!({"voice": {"consent": true, "enabled": true}})).unwrap();
        assert!(on.status().message.starts_with("Simulated voice"));
    }

    #[test]
    fn listening_needs_consent_and_keys_leave_only_a_hint() {
        assert_eq!(update(json!({"voice": {"enabled": true}})).unwrap_err().status, 400);
        let on = update(json!({"voice": {"consent": true, "enabled": true, "mode": "always_on"}})).unwrap();
        assert_eq!(on.mode, VoiceMode::AlwaysOn);
        let off = on
            .updated(json!({"voice": {"consent": false}}).as_object().unwrap())
            .unwrap();
        assert!(!off.enabled && !off.consent);
        let keyed = update(json!({"elevenlabs_api_key": " sk-el-secret-wxyz "})).unwrap();
        assert_eq!(keyed.key_hint.as_deref(), Some("wxyz"));
        assert_eq!(update(json!({"elevenlabs_api_key": ""})).unwrap().key_hint, None);
    }

    #[test]
    fn a_refused_field_changes_nothing() {
        for bad in [
            json!({"voice": {"mode": "sometimes"}}),
            json!({"voice": {"wake_word": "x"}}),
            json!({"voice": {"language": "English"}}),
            json!({"voice": {"speak": "yes"}}),
            json!({"voice": {"volume": 3}}),
            json!({"voice": [1]}),
            json!({"elevenlabs_api_key": 5}),
        ] {
            assert!(update(bad.clone()).is_err(), "{bad}");
        }
    }
}
