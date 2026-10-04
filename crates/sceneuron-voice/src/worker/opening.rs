// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Opening connections on threads of their own, so push to talk and audio keep flowing while
//! one opens, and making sure none of those threads outlives the voice thread.

use std::thread::{self, JoinHandle};

use super::{Command, Worker};
use crate::protocol::session_url;
use crate::session::Failure;
use crate::socket::Link;
use crate::status::VoiceEvent;
use crate::transport::Abort;

/// A connection being opened on its own thread.
pub(super) struct Opening {
    attempt: u64,
    abort: Abort,
    thread: JoinHandle<()>,
}

impl<F: FnMut(VoiceEvent)> Worker<F> {
    pub(super) fn connect(&mut self) {
        self.attempt += 1;
        let attempt = self.attempt;
        let config = self.session.config();
        let (url, key) = (session_url(config, self.session.kind()), config.api_key.clone());
        let (transport, commands) = (self.transport.clone(), self.commands.clone());
        let abort = Abort::default();
        let watched = abort.clone();
        let spawned = thread::Builder::new()
            .name("sceneuron-voice-connect".to_owned())
            .spawn(move || {
                let result = Link::open(&url, &key, &transport, &watched);
                let _ = commands.send(Command::Opened { attempt, result });
            });
        match spawned {
            Ok(thread) => self.opening.push(Opening { attempt, abort, thread }),
            Err(error) => {
                let now = self.now_ms();
                self.session
                    .failed(Failure::Retry(format!("cannot start a connection: {error}")), now);
            }
        }
    }

    /// An attempt ended. Its link serves the session if the session still waits for it, and is
    /// closed otherwise.
    pub(super) fn opened(&mut self, attempt: u64, result: Result<Link, Failure>) {
        // The thread sent its result last, so it ends at once.
        if let Some(index) = self.opening.iter().position(|opening| opening.attempt == attempt) {
            let _ = self.opening.swap_remove(index).thread.join();
        }
        let current = attempt == self.attempt && self.session.is_connecting();
        let now = self.now_ms();
        match result {
            Ok(link) if current => {
                self.link = Some(link);
                self.session.connected(now);
            }
            Ok(link) => link.close(),
            Err(failure) if current => self.session.failed(failure, now),
            Err(_) => {}
        }
    }

    pub(super) fn abort_openings(&self) {
        self.opening.iter().for_each(|opening| opening.abort.abort());
    }

    pub(super) fn join_openings(&mut self) {
        for opening in std::mem::take(&mut self.opening) {
            let _ = opening.thread.join();
        }
    }
}
