// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What the session hears from the worker: settings, push to talk, audio, the connection, and
//! the server's messages.

use super::{
    Action, COMMIT_WAIT_MS, FIRST_RETRY_MS, Failure, Kind, Link, MAX_HELD_BYTES, MAX_RETRY_MS, MAX_STREAM_BYTES, Retry,
    Session, Turn, failure_for,
};
use crate::config::{Mode, VoiceConfig};
use crate::heard::Heard;
use crate::protocol::{ErrorKind, ServerMessage};
use crate::status::VoiceEvent;

impl Session {
    /// New settings. A change to what a connection was opened with replaces it once no turn
    /// needs it, and a change of any kind lifts a stop after a fatal error.
    pub fn configure(&mut self, config: VoiceConfig, now_ms: u64) {
        if config == self.config {
            return;
        }
        if !config.same_session(&self.config) {
            self.stale = self.link != Link::Down;
            self.failures = 0;
            self.retry = None;
        }
        self.stopped = None;
        self.config = config;
        self.update(now_ms);
    }

    /// Push to talk pressed or released. A press after a fatal error or during a retry pause
    /// tries again at once.
    pub fn talk(&mut self, down: bool, now_ms: u64) {
        if down == self.held {
            return;
        }
        self.held = down;
        self.last_turn_ms = now_ms;
        if down {
            if self.config.blocker().is_none() {
                self.stopped = None;
                self.retry = None;
                self.turn = Turn::Speaking;
                // A released turn whose commit still waits for the session joins this one.
                self.turn_has_audio = self.commit_waiting;
                self.commit_waiting = false;
            }
        } else if self.turn == Turn::Speaking {
            if self.turn_has_audio {
                self.commit_waiting = true;
                self.turn = Turn::Committing {
                    until_ms: now_ms + COMMIT_WAIT_MS,
                };
            } else {
                self.turn = Turn::Idle;
            }
        }
        self.update(now_ms);
    }

    /// Converted 16 kHz PCM. Held audio goes to the turn; always-on takes the rest for its
    /// stream, and push to talk drops it.
    pub fn audio(&mut self, pcm: &[u8], now_ms: u64) {
        if !self.may_send() || pcm.is_empty() {
            return;
        }
        let (buffer, limit) = if self.turn == Turn::Speaking {
            self.turn_has_audio = true;
            (&mut self.turn_audio, MAX_HELD_BYTES)
        } else if self.config.mode == Mode::AlwaysOn {
            (&mut self.stream_audio, MAX_STREAM_BYTES)
        } else {
            return;
        };
        buffer.extend(pcm);
        let excess = buffer.len().saturating_sub(limit);
        // Dropping an even count keeps the buffer on sample boundaries.
        buffer.drain(..excess + excess % 2);
        if matches!(self.link, Link::Up { .. }) {
            self.flush(now_ms);
        }
    }

    /// The connection asked for by [`Action::Connect`] is open and its session started.
    pub fn connected(&mut self, now_ms: u64) {
        if self.link != Link::Connecting {
            return;
        }
        self.link = Link::Up { since_ms: now_ms };
        if self.stale {
            self.disconnect();
        }
        self.update(now_ms);
    }

    pub fn message(&mut self, message: ServerMessage, now_ms: u64) {
        // Only the open connection speaks; anything else belongs to one already let go.
        if !matches!(self.link, Link::Up { .. }) {
            return;
        }
        match message {
            ServerMessage::Partial(text) => self.heard(text, false, now_ms),
            ServerMessage::Committed(text) => self.heard(text, true, now_ms),
            ServerMessage::Language(code) => self.actions.push(Action::Emit(VoiceEvent::Language(code))),
            ServerMessage::Error {
                kind: ErrorKind::Warning,
                ..
            } => {
                // A refused commit brings no transcript of its own.
                if !self.held && matches!(self.turn, Turn::Committing { .. }) && !self.commit_waiting {
                    self.end_turn();
                }
            }
            ServerMessage::Error { kind, code, message } => {
                if self.link != Link::Down {
                    self.actions.push(Action::Disconnect);
                }
                return self.failed(failure_for(kind, &code, &message), now_ms);
            }
            ServerMessage::SessionStarted | ServerMessage::Other(_) => {}
        }
        self.update(now_ms);
    }

    /// A transcript from the open connection: addressed when that connection is a turn, which
    /// only ever carried held audio. Whether other speech names the wake word is for the engine
    /// to decide.
    fn heard(&mut self, text: String, is_final: bool, now_ms: u64) {
        let addressed = self.kind == Kind::Turn;
        if is_final && addressed && !self.held && !self.commit_waiting && matches!(self.turn, Turn::Committing { .. }) {
            self.end_turn();
            self.last_turn_ms = now_ms;
        }
        let text = text.trim();
        if !text.is_empty() {
            let heard = Heard {
                text: text.to_owned(),
                is_final,
                addressed,
            };
            self.actions.push(Action::Emit(VoiceEvent::Heard(heard)));
        }
    }

    /// The connection failed, was refused, or ended; the worker has already let it go.
    pub fn failed(&mut self, failure: Failure, now_ms: u64) {
        // A server may quote the key back; statuses and the log must not.
        let failure = failure.redacted(&self.config.api_key);
        self.link = Link::Down;
        self.stale = false;
        // A sent commit's transcript is lost with its session.
        if !self.held && !self.commit_waiting && matches!(self.turn, Turn::Committing { .. }) {
            self.end_turn();
        }
        match failure {
            Failure::Fatal(message) => {
                self.stopped = Some(message);
                self.retry = None;
                self.end_turn();
                self.turn_audio.clear();
                self.stream_audio.clear();
            }
            Failure::Renew => self.retry = None,
            Failure::Closed if !self.wants_link() => {}
            Failure::Closed => self.schedule_retry("the server ended the session".to_owned(), now_ms),
            Failure::Retry(message) => self.schedule_retry(message, now_ms),
        }
        self.report_usage();
        self.update(now_ms);
    }

    fn schedule_retry(&mut self, message: String, now_ms: u64) {
        self.failures += 1;
        let doublings = (self.failures - 1).min(16);
        let delay_ms = FIRST_RETRY_MS.saturating_mul(1 << doublings).min(MAX_RETRY_MS);
        self.retry = Some(Retry {
            at_ms: now_ms + delay_ms,
            delay_ms,
            message,
        });
    }

    /// Time passed: ends a turn whose transcript never came, retries, and closes an idle
    /// push-to-talk connection.
    pub fn tick(&mut self, now_ms: u64) {
        if let Turn::Committing { until_ms } = self.turn
            && now_ms >= until_ms
            && !self.held
        {
            self.end_turn();
            self.turn_audio.clear();
        }
        self.update(now_ms);
    }

    /// Closes the connection for good and reports audio not yet reported.
    pub fn shutdown(&mut self) {
        if self.link != Link::Down {
            self.disconnect();
        }
        self.report_usage();
    }
}
