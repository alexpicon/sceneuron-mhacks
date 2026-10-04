// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The session policy: when to open and close the connection, what audio to send and when to
//! commit it, and whether a transcript was addressed to Sceneuron.
//!
//! Held speech goes to a session of its own, a turn, which commits by hand; in always-on mode
//! the rest streams to another session that commits on voice activity. A press closes the
//! stream and opens a turn, and once the turn's transcript is back the stream opens again. So
//! every transcript comes from a session that carried only held audio or only other audio, and
//! whether it was addressed follows the audio it covers, never when it arrives.
//!
//! It does no I/O and reads no clock. The worker carries out the [`Action`]s it asks for and
//! reports back, so every rule here is testable with made-up times.

mod inputs;
mod reconcile;
mod state;

use std::collections::VecDeque;

use crate::audio::BYTES_PER_SECOND;
use crate::config::{ApiKey, VoiceConfig};
use crate::protocol::{ErrorKind, shorten};
use crate::status::{VoiceEvent, VoiceStatus};

/// Audio goes out in pieces of 100 ms.
pub const CHUNK_BYTES: usize = BYTES_PER_SECOND / 10;
/// Push-to-talk audio kept while its session opens, enough for a long command.
pub const MAX_HELD_BYTES: usize = BYTES_PER_SECOND * 30;
/// Always-on audio kept while a session reopens; older audio is dropped.
pub const MAX_STREAM_BYTES: usize = BYTES_PER_SECOND * 5;
/// A push-to-talk session stays open this long after its last turn, so the next press is
/// heard at once, and then closes.
pub const IDLE_CLOSE_MS: u64 = 30_000;
/// How long a released turn waits for its final transcript.
pub const COMMIT_WAIT_MS: u64 = 10_000;
/// The first retry waits this long, and each failure in a row doubles it up to the maximum.
pub const FIRST_RETRY_MS: u64 = 1_000;
pub const MAX_RETRY_MS: u64 = 60_000;
/// A session that stayed open this long makes the next failure count as the first.
pub const STABLE_MS: u64 = 30_000;
/// Audio sent is reported for spend once this much is unreported, and at every commit.
const USAGE_REPORT_BYTES: usize = BYTES_PER_SECOND * 10;

/// Why a session ended or never started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// Retrying cannot help until something changes, such as a refused key.
    Fatal(String),
    /// The session reached its time limit; a new one may start at once.
    Renew,
    /// Something that may pass; retry after a pause.
    Retry(String),
    /// The server ended the session normally.
    Closed,
}

impl Failure {
    /// The same failure with the key removed from its message, which is then cut to fit a
    /// status; in that order, so a key across the cut cannot leave a part of itself behind.
    pub fn redacted(self, key: &ApiKey) -> Failure {
        match self {
            Failure::Fatal(message) => Failure::Fatal(shorten(&key.redact(&message))),
            Failure::Retry(message) => Failure::Retry(shorten(&key.redact(&message))),
            other => other,
        }
    }
}

/// The failure a server error stands for; warnings do not end a session.
pub(crate) fn failure_for(kind: ErrorKind, code: &str, message: &str) -> Failure {
    let text = if message.is_empty() {
        code.to_owned()
    } else {
        format!("{code}: {message}")
    };
    match kind {
        ErrorKind::Fatal => Failure::Fatal(format!("ElevenLabs refused the session ({text})")),
        ErrorKind::Renew => Failure::Renew,
        ErrorKind::Retry | ErrorKind::Warning => Failure::Retry(format!("ElevenLabs reported an error ({text})")),
    }
}

/// What the session asks the worker to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// Open a connection for the current config, then call [`Session::connected`] or
    /// [`Session::failed`].
    Connect,
    Send {
        pcm: Vec<u8>,
        commit: bool,
    },
    /// Close the connection, or abandon the one being opened.
    Disconnect,
    Emit(VoiceEvent),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Link {
    Down,
    Connecting,
    Up { since_ms: u64 },
}

/// What a connection carries: held speech, committed by hand, or everything else in always-on
/// mode, committed on voice activity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Turn,
    Stream,
}

/// A stretch of addressed speech: push to talk held, then released until its final transcript.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Turn {
    Idle,
    Speaking,
    Committing { until_ms: u64 },
}

#[derive(Clone, Debug)]
struct Retry {
    at_ms: u64,
    delay_ms: u64,
    message: String,
}

pub struct Session {
    config: VoiceConfig,
    link: Link,
    /// What the open or opening connection carries.
    kind: Kind,
    /// The open or opening connection was asked to bias toward keyterms, which costs extra.
    link_keyterms: bool,
    held: bool,
    turn: Turn,
    /// Held audio waiting for the turn's connection or for a full piece.
    turn_audio: VecDeque<u8>,
    /// Always-on audio said outside a turn, waiting for the stream's connection.
    stream_audio: VecDeque<u8>,
    /// The released turn's last audio and commit wait for the connection.
    commit_waiting: bool,
    turn_has_audio: bool,
    failures: u32,
    retry: Option<Retry>,
    stopped: Option<String>,
    /// The open connection was made for an older config.
    stale: bool,
    last_turn_ms: u64,
    unreported_bytes: usize,
    status: VoiceStatus,
    actions: Vec<Action>,
}
