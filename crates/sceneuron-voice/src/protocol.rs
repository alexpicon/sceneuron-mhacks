// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The Scribe v2 Realtime messages: the session URL, the audio message, and what the server
//! sends back. Shapes and error types follow
//! <https://elevenlabs.io/docs/api-reference/speech-to-text/v-1-speech-to-text-realtime>.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use crate::audio::SAMPLE_RATE;
use crate::config::VoiceConfig;
use crate::session::Kind;

/// Server text longer than this is cut, once the key is out of it, before it reaches a status
/// or the log.
const MAX_SERVER_TEXT: usize = 200;

/// The URL of a session for `config`: the model, `pcm_16000` audio, manual commits for a turn
/// of held speech or voice activity detection for an always-on stream, the language when one is
/// set and otherwise a request to report the language detected, and each keyterm as its own
/// `keyterms` parameter. The API key is never part of it.
pub fn session_url(config: &VoiceConfig, kind: Kind) -> String {
    let strategy = match kind {
        Kind::Turn => "manual",
        Kind::Stream => "vad",
    };
    let mut query = vec![
        ("model_id", config.model.as_str()),
        ("audio_format", "pcm_16000"),
        ("commit_strategy", strategy),
    ];
    if config.language.trim().is_empty() {
        query.push(("include_language_detection", "true"));
    } else {
        query.push(("language_code", config.language.trim()));
    }
    query.extend(config.keyterms.iter().map(|term| ("keyterms", term.as_str())));
    let query: Vec<String> = query
        .into_iter()
        .map(|(name, value)| format!("{name}={}", encode(value)))
        .collect();
    let separator = if config.endpoint.contains('?') { '&' } else { '?' };
    format!("{}{separator}{}", config.endpoint, query.join("&"))
}

/// Percent-encodes everything but RFC 3986's unreserved characters.
fn encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// One `input_audio_chunk` message carrying 16 kHz PCM; `commit` ends the segment after it.
pub fn audio_message(pcm: &[u8], commit: bool) -> String {
    json!({
        "message_type": "input_audio_chunk",
        "audio_base_64": STANDARD.encode(pcm),
        "commit": commit,
        "sample_rate": SAMPLE_RATE,
    })
    .to_string()
}

/// What a server error means for the session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// Retrying cannot help until the streamer changes something: a refused key, terms not
    /// accepted, or no quota left.
    Fatal,
    /// The session reached its time limit; a new one can start at once.
    Renew,
    /// The session failed; try again after a pause.
    Retry,
    /// The session goes on; one request was refused, such as a commit too soon after another.
    Warning,
}

/// A message from the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerMessage {
    SessionStarted,
    /// Text of the current segment so far; it may still change.
    Partial(String),
    /// The final text of a segment.
    Committed(String),
    /// The language of the segment just committed, by the ISO 639 code the server reports, in a
    /// session that asked for language detection; it comes a little after the segment's text.
    Language(String),
    Error {
        kind: ErrorKind,
        code: String,
        message: String,
    },
    /// A message this client has no use for, such as transcripts with timestamps but no language,
    /// by type.
    Other(String),
}

/// Parses one text frame; `None` when it is not a JSON object with a `message_type`.
pub fn parse_server_message(text: &str) -> Option<ServerMessage> {
    let value: Value = serde_json::from_str(text).ok()?;
    let kind = value.get("message_type")?.as_str()?;
    let field = |name: &str| value.get(name).and_then(Value::as_str).unwrap_or_default().to_owned();
    let message = match kind {
        "session_started" => ServerMessage::SessionStarted,
        "partial_transcript" => ServerMessage::Partial(field("text")),
        "committed_transcript" => ServerMessage::Committed(field("text")),
        "committed_transcript_with_timestamps" if !field("language_code").trim().is_empty() => {
            ServerMessage::Language(field("language_code").trim().to_owned())
        }
        "warning" => ServerMessage::Error {
            kind: ErrorKind::Warning,
            code: kind.to_owned(),
            message: field("warning"),
        },
        code => match error_kind(code) {
            Some(kind) => ServerMessage::Error {
                kind,
                code: code.to_owned(),
                message: field("error"),
            },
            None => ServerMessage::Other(code.to_owned()),
        },
    };
    Some(message)
}

/// The error types the API reference lists, by what they mean for the session.
fn error_kind(code: &str) -> Option<ErrorKind> {
    let kind = match code {
        "auth_error" | "unaccepted_terms" | "quota_exceeded" => ErrorKind::Fatal,
        "session_time_limit_exceeded" => ErrorKind::Renew,
        "commit_throttled" | "insufficient_audio_activity" => ErrorKind::Warning,
        "error"
        | "rate_limited"
        | "queue_overflow"
        | "resource_exhausted"
        | "transcriber_error"
        | "input_error"
        | "invalid_request"
        | "chunk_size_exceeded" => ErrorKind::Retry,
        _ => return None,
    };
    Some(kind)
}

/// `text` cut to a length fit for a status line, on a character boundary.
pub(crate) fn shorten(text: &str) -> String {
    match text.char_indices().nth(MAX_SERVER_TEXT) {
        Some((end, _)) => format!("{}...", &text[..end]),
        None => text.to_owned(),
    }
}
