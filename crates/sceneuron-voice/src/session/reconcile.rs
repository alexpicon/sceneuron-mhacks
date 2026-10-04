// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Bringing the session in line after every input: closing what is no longer wanted, opening
//! what is, sending what waits, and reporting the status.

use super::{
    Action, CHUNK_BYTES, COMMIT_WAIT_MS, IDLE_CLOSE_MS, Kind, Link, STABLE_MS, Session, Turn, USAGE_REPORT_BYTES,
};
use crate::audio::BYTES_PER_SECOND;
use crate::config::Mode;
use crate::status::{VoiceEvent, VoiceStatus};

impl Session {
    /// Whether audio may leave the computer: on, consented, keyed, and not stopped.
    pub(super) fn may_send(&self) -> bool {
        self.config.blocker().is_none() && self.stopped.is_none()
    }

    pub(super) fn wants_link(&self) -> bool {
        self.may_send() && (self.config.mode == Mode::AlwaysOn || self.held || self.turn != Turn::Idle)
    }

    /// A turn's connection while push to talk is held or its turn is not over, and always in
    /// push-to-talk mode; a stream's otherwise.
    pub(super) fn wanted_kind(&self) -> Kind {
        if self.config.mode == Mode::PushToTalk || self.held || self.turn != Turn::Idle {
            Kind::Turn
        } else {
            Kind::Stream
        }
    }

    pub(super) fn idle_push_to_talk(&self) -> bool {
        self.config.mode == Mode::PushToTalk && !self.held && self.turn == Turn::Idle
    }

    pub(super) fn end_turn(&mut self) {
        self.turn = Turn::Idle;
        self.commit_waiting = false;
        self.turn_has_audio = false;
    }

    pub(super) fn update(&mut self, now_ms: u64) {
        if !self.may_send() {
            self.turn_audio.clear();
            self.stream_audio.clear();
            self.end_turn();
        }
        if self.link != Link::Down {
            let free = !self.held && self.turn == Turn::Idle;
            let idle_for = now_ms.saturating_sub(self.last_turn_ms);
            let idle = self.idle_push_to_talk() && matches!(self.link, Link::Up { .. }) && idle_for >= IDLE_CLOSE_MS;
            let other_kind = self.kind != self.wanted_kind();
            if !self.may_send() || (self.stale && free) || idle || other_kind {
                self.disconnect();
            }
        }
        if self.link == Link::Down && self.wants_link() && self.retry.as_ref().is_none_or(|retry| now_ms >= retry.at_ms)
        {
            self.retry = None;
            self.link = Link::Connecting;
            self.kind = self.wanted_kind();
            self.link_keyterms = !self.config.keyterms.is_empty();
            self.actions.push(Action::Connect);
        }
        if let Link::Up { since_ms } = self.link {
            if now_ms.saturating_sub(since_ms) >= STABLE_MS {
                self.failures = 0;
            }
            self.flush(now_ms);
        }
        let status = self.current_status();
        if status != self.status {
            self.status = status.clone();
            self.actions.push(Action::Emit(VoiceEvent::Status(status)));
        }
    }

    pub(super) fn disconnect(&mut self) {
        self.link = Link::Down;
        self.stale = false;
        self.actions.push(Action::Disconnect);
        self.report_usage();
    }

    /// Sends every full piece of audio waiting for this connection, and when a released turn
    /// waits for its commit, the rest with the commit, so the last piece carries it. A stream
    /// never commits by hand.
    pub(super) fn flush(&mut self, now_ms: u64) {
        let commit = self.kind == Kind::Turn && self.commit_waiting;
        loop {
            let buffer = match self.kind {
                Kind::Turn => &mut self.turn_audio,
                Kind::Stream => &mut self.stream_audio,
            };
            if !(buffer.len() > CHUNK_BYTES || (buffer.len() == CHUNK_BYTES && !commit)) {
                break;
            }
            let pcm: Vec<u8> = buffer.drain(..CHUNK_BYTES).collect();
            self.send(pcm, false);
        }
        if commit {
            let pcm: Vec<u8> = self.turn_audio.drain(..).collect();
            self.send(pcm, true);
            self.commit_waiting = false;
            if let Turn::Committing { .. } = self.turn {
                self.turn = Turn::Committing {
                    until_ms: now_ms + COMMIT_WAIT_MS,
                };
            }
            self.report_usage();
        } else if self.unreported_bytes >= USAGE_REPORT_BYTES {
            self.report_usage();
        }
    }

    fn send(&mut self, pcm: Vec<u8>, commit: bool) {
        self.unreported_bytes += pcm.len();
        self.actions.push(Action::Send { pcm, commit });
    }

    pub(super) fn report_usage(&mut self) {
        if self.unreported_bytes > 0 {
            let audio_seconds = self.unreported_bytes as f64 / BYTES_PER_SECOND as f64;
            self.unreported_bytes = 0;
            let keyterms = self.link_keyterms;
            self.actions.push(Action::Emit(VoiceEvent::Usage {
                audio_seconds,
                keyterms,
            }));
        }
    }

    pub(super) fn current_status(&self) -> VoiceStatus {
        if let Some(blocker) = self.config.blocker() {
            return blocker;
        }
        if let Some(message) = &self.stopped {
            return VoiceStatus::Stopped {
                message: message.clone(),
            };
        }
        match (self.link, &self.retry) {
            (Link::Connecting, _) => VoiceStatus::Connecting,
            (Link::Up { .. }, _) if self.config.mode == Mode::AlwaysOn || self.held => VoiceStatus::Listening,
            (Link::Down, Some(retry)) if self.wants_link() => VoiceStatus::Retrying {
                message: retry.message.clone(),
                retry_in_ms: retry.delay_ms,
            },
            _ => VoiceStatus::Ready,
        }
    }
}
