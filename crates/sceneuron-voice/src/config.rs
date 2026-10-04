// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What the voice input may do: whether it listens, how speech addresses Sceneuron, and where
//! audio goes.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::status::VoiceStatus;

/// The Scribe v2 Realtime WebSocket, from
/// <https://elevenlabs.io/docs/api-reference/speech-to-text/v-1-speech-to-text-realtime>.
pub const DEFAULT_ENDPOINT: &str = "wss://api.elevenlabs.io/v1/speech-to-text/realtime";
/// The realtime model, from the same page.
pub const DEFAULT_MODEL: &str = "scribe_v2_realtime";
pub const DEFAULT_WAKE_WORD: &str = "Sceneuron";

/// How speech addresses Sceneuron.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Audio leaves the computer only while the push-to-talk key is held, and everything said
    /// then is addressed to Sceneuron.
    #[default]
    PushToTalk,
    /// Audio leaves the computer all the time; speech that names the wake word is addressed to
    /// Sceneuron, and holding push to talk addresses it too.
    AlwaysOn,
}

/// An ElevenLabs API key. It travels only in the `xi-api-key` header, never in a URL, and
/// neither `Debug` nor any message shows it.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct ApiKey(String);

impl ApiKey {
    pub fn new(key: impl Into<String>) -> ApiKey {
        ApiKey(key.into().trim().to_owned())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }

    /// `text` with every occurrence of the key replaced, for messages that quote a server.
    pub fn redact(&self, text: &str) -> String {
        if self.0.is_empty() {
            text.to_owned()
        } else {
            text.replace(&self.0, "[key]")
        }
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.0.is_empty() {
            "ApiKey(none)"
        } else {
            "ApiKey(<set>)"
        })
    }
}

/// Everything the voice input needs to decide whether and how to listen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceConfig {
    pub enabled: bool,
    /// The streamer agreed that audio may go to ElevenLabs. Nothing is sent without it.
    pub consent: bool,
    pub mode: Mode,
    /// Names Sceneuron in always-on mode; empty means only push to talk addresses it.
    pub wake_word: String,
    /// An ISO 639-1 or 639-3 code; empty lets Scribe detect the language.
    pub language: String,
    pub model: String,
    /// Words recognition is biased toward, already within Scribe's limits; see
    /// [`keyterms`](crate::keyterms).
    pub keyterms: Vec<String>,
    /// A `wss://` URL. A `ws://` URL is accepted only for a server on this computer.
    pub endpoint: String,
    pub api_key: ApiKey,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            consent: false,
            mode: Mode::PushToTalk,
            wake_word: DEFAULT_WAKE_WORD.to_owned(),
            language: String::new(),
            model: DEFAULT_MODEL.to_owned(),
            keyterms: Vec::new(),
            endpoint: DEFAULT_ENDPOINT.to_owned(),
            api_key: ApiKey::default(),
        }
    }
}

impl VoiceConfig {
    /// Why no audio may leave the computer now, or `None` when it may.
    pub fn blocker(&self) -> Option<VoiceStatus> {
        if !self.enabled {
            Some(VoiceStatus::Off)
        } else if !self.consent {
            Some(VoiceStatus::NeedsConsent)
        } else if self.api_key.is_empty() {
            Some(VoiceStatus::NeedsKey)
        } else {
            None
        }
    }

    /// Whether a session opened for `other` also serves this config; when not, a new
    /// connection is needed, because these settings are fixed when a session opens.
    pub(crate) fn same_session(&self, other: &VoiceConfig) -> bool {
        self.mode == other.mode
            && self.language == other.language
            && self.model == other.model
            && self.keyterms == other.keyterms
            && self.endpoint == other.endpoint
            && self.api_key == other.api_key
    }
}
