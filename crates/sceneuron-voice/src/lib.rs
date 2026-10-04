// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Sceneuron's voice input: audio from a microphone or a file, converted to 16 kHz mono PCM and
//! streamed to ElevenLabs' Scribe v2 Realtime speech to text over a TLS WebSocket, with the
//! transcripts delivered as [`Heard`] events.
//!
//! Push to talk opens a session when the key is pressed, commits the segment when it is
//! released, and closes the session after a while without use; everything said meanwhile is
//! addressed to Sceneuron. Always-on keeps a stream open, reconnecting with backoff, and leaves
//! the wake word to the engine; a press there closes the stream and holds a session of its own
//! for the held speech. Nothing is sent unless voice control is on, the streamer consented, and
//! there is a key; the key travels only in a request header. The audio thread hands audio over
//! through a ring allocated once, with atomics only. Nothing here depends on OBS.
//!
//! tungstenite logs the raw handshake request, key header included, at trace level under its
//! own `tungstenite` target. A host must not forward that target's trace records anywhere; the
//! OBS plugin forwards only Sceneuron's own targets, and only up to debug.

mod audio;
mod config;
#[cfg(feature = "fake")]
pub mod fake;
mod heard;
mod keyterms;
mod protocol;
mod resample;
mod ring;
pub mod session;
mod shared;
mod socket;
mod status;
mod transport;
mod voice;
mod wav;
mod worker;

pub use audio::{AudioChunk, BYTES_PER_SECOND, Converter, SAMPLE_RATE};
pub use config::{ApiKey, DEFAULT_ENDPOINT, DEFAULT_MODEL, DEFAULT_WAKE_WORD, Mode, VoiceConfig};
pub use heard::Heard;
pub use keyterms::{MAX_KEYTERM_CHARS, MAX_KEYTERMS, keyterms};
pub use protocol::{ErrorKind, ServerMessage, audio_message, parse_server_message, session_url};
pub use resample::Resampler;
pub use status::{VoiceEvent, VoiceStatus};
pub use transport::Transport;
pub use voice::{Voice, VoiceInput};
pub use wav::{Wav, WavError, pcm16_wav, read_wav};
