// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What the voice input is doing, for the editor and the event log, and the events it reports.

use serde::Serialize;

use crate::heard::Heard;

/// The voice input's state. Serialized with a `state` tag, such as `{"state": "listening"}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum VoiceStatus {
    /// Voice control is off.
    Off,
    /// Voice control is on, but the streamer has not agreed to send audio to ElevenLabs.
    NeedsConsent,
    /// Voice control is on and agreed to, but there is no ElevenLabs key.
    NeedsKey,
    /// Waiting for push to talk; no audio leaves the computer.
    Ready,
    /// Opening a session with ElevenLabs.
    Connecting,
    /// Audio is going to ElevenLabs.
    Listening,
    /// The session failed and another attempt follows.
    Retrying { message: String, retry_in_ms: u64 },
    /// The session failed in a way retrying cannot fix, such as a refused key. Nothing is sent
    /// until the settings change or push to talk is pressed again.
    Stopped { message: String },
}

impl VoiceStatus {
    pub fn is_listening(&self) -> bool {
        matches!(self, VoiceStatus::Listening)
    }

    /// One sentence for the event log or the editor.
    pub fn describe(&self) -> String {
        match self {
            VoiceStatus::Off => "voice control is off".to_owned(),
            VoiceStatus::NeedsConsent => "voice control needs your consent to send audio to ElevenLabs".to_owned(),
            VoiceStatus::NeedsKey => "voice control needs an ElevenLabs API key".to_owned(),
            VoiceStatus::Ready => "voice control is ready; hold push to talk to speak".to_owned(),
            VoiceStatus::Connecting => "connecting to ElevenLabs speech to text".to_owned(),
            VoiceStatus::Listening => "listening".to_owned(),
            VoiceStatus::Retrying { message, retry_in_ms } => {
                format!(
                    "{message}; trying again in {:.0} s",
                    (*retry_in_ms as f64 / 1000.0).ceil()
                )
            }
            VoiceStatus::Stopped { message } => format!("stopped: {message}"),
        }
    }
}

/// What the voice input reports, in order, on its own thread.
#[derive(Clone, Debug, PartialEq)]
pub enum VoiceEvent {
    /// A transcript: partial while the speaker talks, final once the segment is committed.
    Heard(Heard),
    /// The language speech to text detected in the segment it last committed, by the ISO 639 code
    /// it reports, while the spoken language is left to detection.
    Language(String),
    Status(VoiceStatus),
    /// Seconds of audio sent to ElevenLabs since the last report, for spend accounting, and
    /// whether the session carried keyterms, which cost extra.
    Usage {
        audio_seconds: f64,
        keyterms: bool,
    },
}
