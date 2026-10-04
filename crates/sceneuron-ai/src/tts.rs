// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! ElevenLabs text to speech for spoken cards: one request per line of text, answered with MP3
//! audio, and the list of voices the account has. The key travels only in the `xi-api-key`
//! header and never appears in an address, a log, or an error.

use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::{Failure, Http, with_retries};
use crate::provider::AiError;

/// The ElevenLabs API (https://elevenlabs.io/docs/api-reference/text-to-speech/convert).
pub const DEFAULT_ELEVENLABS_URL: &str = "https://api.elevenlabs.io";
/// MP3 at 44.1 kHz and 128 kbit/s, the API's default output format and available on every
/// plan (https://elevenlabs.io/docs/api-reference/text-to-speech/convert).
pub const SPEECH_FORMAT: &str = "mp3_44100_128";
/// The content type of [`SPEECH_FORMAT`] audio.
pub const SPEECH_CONTENT_TYPE: &str = "audio/mpeg";
/// A spoken card is a sentence or two; a slower answer is not worth waiting for.
const DEADLINE: Duration = Duration::from_secs(20);
/// Listing voices costs nothing and is small.
const VOICES_TIMEOUT: Duration = Duration::from_secs(10);
/// The most voices one page of `GET /v2/voices` holds
/// (https://elevenlabs.io/docs/api-reference/voices/search).
const VOICES_PAGE: usize = 100;

/// A voice in the account.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct AccountVoice {
    pub voice_id: String,
    #[serde(default)]
    pub name: String,
    /// Such as `premade`, `cloned`, `generated`, or `professional`.
    #[serde(default)]
    pub category: String,
}

/// The first page of the account's voices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountVoices {
    pub voices: Vec<AccountVoice>,
    /// More voices follow, so a voice missing from this page may still be in the account.
    pub has_more: bool,
}

impl AccountVoices {
    pub fn contains(&self, voice_id: &str) -> bool {
        self.voices.iter().any(|voice| voice.voice_id == voice_id)
    }

    /// A voice to use instead of one the account lacks: its first premade voice, else its first.
    pub fn replacement(&self) -> Option<&AccountVoice> {
        self.voices
            .iter()
            .find(|voice| voice.category == "premade")
            .or_else(|| self.voices.first())
    }
}

/// The error for a voice id the account does not have.
pub fn missing_voice(voice_id: &str) -> AiError {
    AiError::Config(format!(
        "the voice id {voice_id} is not in your ElevenLabs account; paste one from My Voices into the AI tab"
    ))
}

/// A text to speech client for one key.
pub struct TextToSpeech {
    http: Http,
    base_url: String,
    key: String,
}

impl TextToSpeech {
    /// A client for `base_url`, such as [`DEFAULT_ELEVENLABS_URL`], that sends `key`.
    pub fn new(base_url: &str, key: &str) -> TextToSpeech {
        TextToSpeech {
            http: Http::new(base_url, key),
            base_url: base_url.trim_end_matches('/').to_owned(),
            key: key.to_owned(),
        }
    }

    /// Speaks `text` in the voice `voice_id` with `model`, as MP3 audio.
    ///
    /// Rate limits and outages are retried briefly. The voice id and model must be plain
    /// letters, digits, and `_.-`, as the voice settings check. A voice the account does not
    /// have is [`missing_voice`].
    pub fn speak(&self, text: &str, voice_id: &str, model: &str) -> Result<Vec<u8>, AiError> {
        self.check_key()?;
        if voice_id.is_empty() || !voice_id.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err(AiError::Config("the voice id must be letters and digits".to_owned()));
        }
        let url = format!(
            "{}/v1/text-to-speech/{voice_id}?output_format={SPEECH_FORMAT}",
            self.base_url
        );
        let body = json!({ "text": text, "model_id": model }).to_string();
        let headers = [("xi-api-key", self.key.as_str()), ("accept", SPEECH_CONTENT_TYPE)];
        with_retries(Instant::now() + DEADLINE, |remaining| {
            let reply = self.http.post_bytes(&url, &headers, body.as_bytes(), remaining)?;
            if reply.status == 404 && names_missing_voice(&reply.body) {
                return Err(Failure::from(missing_voice(voice_id)));
            }
            if reply.status != 200 {
                return Err(self.http.bytes_error(&reply));
            }
            if !is_mp3(&reply.body) {
                let message = "ElevenLabs answered without MP3 audio".to_owned();
                return Err(Failure::from(AiError::Invalid(message)));
            }
            Ok(reply.body)
        })
    }

    /// The first page of the voices the account may use, which costs nothing
    /// (https://elevenlabs.io/docs/api-reference/voices/search).
    pub fn voices(&self) -> Result<AccountVoices, AiError> {
        self.check_key()?;
        let url = format!("{}/v2/voices?page_size={VOICES_PAGE}", self.base_url);
        let headers = [("xi-api-key", self.key.as_str()), ("accept", "application/json")];
        let reply = self.http.get_bytes(&url, &headers, VOICES_TIMEOUT)?;
        if reply.status != 200 {
            return Err(self.http.bytes_error(&reply).error);
        }
        #[derive(Deserialize)]
        struct Page {
            voices: Vec<AccountVoice>,
            #[serde(default)]
            has_more: bool,
        }
        let page: Page = serde_json::from_slice(&reply.body)
            .map_err(|_| AiError::Invalid("ElevenLabs answered the voice list in an unknown shape".to_owned()))?;
        Ok(AccountVoices {
            voices: page.voices,
            has_more: page.has_more,
        })
    }

    fn check_key(&self) -> Result<(), AiError> {
        if self.key.is_empty() {
            return Err(AiError::Config("add an ElevenLabs key first".to_owned()));
        }
        Ok(())
    }
}

/// Whether an error body says the voice was not found
/// (https://elevenlabs.io/docs/developers/resources/errors).
fn names_missing_voice(body: &[u8]) -> bool {
    let Ok(json) = serde_json::from_slice::<Value>(body) else {
        return false;
    };
    ["/detail/code", "/detail/status"]
        .iter()
        .any(|field| json.pointer(field).and_then(Value::as_str) == Some("voice_not_found"))
}

/// Whether bytes start like an MP3 file: an ID3 tag or an MPEG audio frame header.
fn is_mp3(bytes: &[u8]) -> bool {
    bytes.starts_with(b"ID3") || matches!(bytes, [0xFF, second, ..] if second & 0xE0 == 0xE0)
}
