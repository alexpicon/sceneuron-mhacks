// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Creating a session and reading its state.

use std::collections::VecDeque;

use super::{Action, IDLE_CLOSE_MS, Kind, Link, Session, Turn};
use crate::config::{Mode, VoiceConfig};
use crate::status::{VoiceEvent, VoiceStatus};

impl Session {
    /// A session for `config` at `now_ms`; its first actions report the status and, for
    /// always-on, open a connection.
    pub fn new(config: VoiceConfig, now_ms: u64) -> Session {
        let mut session = Session {
            config,
            link: Link::Down,
            kind: Kind::Turn,
            link_keyterms: false,
            held: false,
            turn: Turn::Idle,
            turn_audio: VecDeque::new(),
            stream_audio: VecDeque::new(),
            commit_waiting: false,
            turn_has_audio: false,
            failures: 0,
            retry: None,
            stopped: None,
            stale: false,
            last_turn_ms: now_ms,
            unreported_bytes: 0,
            status: VoiceStatus::Off,
            actions: Vec::new(),
        };
        session.status = session.current_status();
        session.update(now_ms);
        let reported = session
            .actions
            .iter()
            .any(|action| matches!(action, Action::Emit(VoiceEvent::Status(_))));
        if !reported {
            session
                .actions
                .push(Action::Emit(VoiceEvent::Status(session.status.clone())));
        }
        session
    }

    pub fn config(&self) -> &VoiceConfig {
        &self.config
    }

    pub fn status(&self) -> &VoiceStatus {
        &self.status
    }

    pub fn is_connecting(&self) -> bool {
        self.link == Link::Connecting
    }

    /// What the connection being opened or open carries; a turn commits by hand, a stream on
    /// voice activity.
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// Whether audio should be handed over now: always-on, or push to talk held, and only
    /// while audio may leave the computer.
    pub fn wants_audio(&self) -> bool {
        self.may_send() && (self.config.mode == Mode::AlwaysOn || self.held)
    }

    pub fn take_actions(&mut self) -> Vec<Action> {
        std::mem::take(&mut self.actions)
    }

    /// When [`Session::tick`] next has something to do, if ever.
    pub fn next_wake_ms(&self) -> Option<u64> {
        let retry = self
            .retry
            .as_ref()
            .filter(|_| self.wants_link())
            .map(|retry| retry.at_ms);
        let commit = match self.turn {
            Turn::Committing { until_ms } => Some(until_ms),
            _ => None,
        };
        let idle_close = (self.idle_push_to_talk() && matches!(self.link, Link::Up { .. }))
            .then(|| self.last_turn_ms + IDLE_CLOSE_MS);
        [retry, commit, idle_close].into_iter().flatten().min()
    }
}
