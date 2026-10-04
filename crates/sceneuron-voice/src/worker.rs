// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The voice thread: converts audio, carries out the session's actions on a real connection,
//! and reports events. Connections open on a short-lived thread of their own, so push to talk
//! and audio keep flowing while one opens.
//!
//! Audio arrives through the ring, commands through a channel. A press, a release, or new
//! settings carry the ring position at which they happened, and the audio before that position
//! is handed to the session first, so each applies to exactly the audio after it.

mod opening;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, PoisonError};
use std::time::{Duration, Instant};

use crate::audio::Converter;
use crate::config::{Mode, VoiceConfig};
use crate::protocol::audio_message;
use crate::session::{Action, Failure, Session};
pub(crate) use crate::shared::Shared;
use crate::socket::Link;
use crate::status::VoiceEvent;
use crate::transport::Transport;
use opening::Opening;

/// Without a connection or a timer, the thread still wakes this often.
const IDLE_WAIT: Duration = Duration::from_secs(1);
/// While audio is captured, the ring is emptied at least this often.
const DRAIN_WAIT: Duration = Duration::from_millis(20);

pub(crate) enum Command {
    /// Push to talk, at ring position `at`.
    Talk {
        down: bool,
        at: usize,
    },
    /// New settings from ring position `at`; `generation` counts every call.
    Configure {
        config: VoiceConfig,
        generation: u64,
        at: usize,
    },
    Opened {
        attempt: u64,
        result: Result<Link, Failure>,
    },
    Shutdown,
}

pub(crate) struct Worker<F: FnMut(VoiceEvent)> {
    session: Session,
    link: Option<Link>,
    /// Counts connection attempts; a result for an older one is closed unused.
    attempt: u64,
    /// Every opening thread not yet joined, so none outlives the voice thread.
    opening: Vec<Opening>,
    converter: Converter,
    samples: Vec<f32>,
    pcm: Vec<u8>,
    /// The settings generation the session runs.
    applied: u64,
    transport: Transport,
    commands: Sender<Command>,
    shared: Arc<Shared>,
    clock: Instant,
    on_event: F,
}

impl<F: FnMut(VoiceEvent)> Worker<F> {
    pub(crate) fn new(
        config: VoiceConfig,
        transport: Transport,
        commands: Sender<Command>,
        shared: Arc<Shared>,
        on_event: F,
    ) -> Worker<F> {
        Worker {
            session: Session::new(config, 0),
            link: None,
            attempt: 0,
            opening: Vec::new(),
            converter: Converter::new(),
            samples: Vec::new(),
            pcm: Vec::new(),
            applied: 0,
            transport,
            commands,
            shared,
            clock: Instant::now(),
            on_event,
        }
    }

    fn now_ms(&self) -> u64 {
        self.clock.elapsed().as_millis() as u64
    }

    pub(crate) fn run(mut self, inbox: Receiver<Command>) {
        loop {
            self.act();
            let first = match self.link.as_mut() {
                Some(link) => {
                    // The read waits a moment for the server, so commands wait at most as long.
                    let received = link.receive();
                    let now = self.now_ms();
                    match received {
                        Ok(Some(message)) => self.session.message(message, now),
                        Ok(None) => {}
                        Err(failure) => {
                            self.link = None;
                            self.session.failed(failure, now);
                        }
                    }
                    inbox.try_recv().map_err(|error| error == TryRecvError::Disconnected)
                }
                None => {
                    let now = self.now_ms();
                    let most = if self.shared.capturing() { DRAIN_WAIT } else { IDLE_WAIT };
                    let wait = self
                        .session
                        .next_wake_ms()
                        .map_or(most, |at| Duration::from_millis(at.saturating_sub(now)))
                        .min(most);
                    inbox
                        .recv_timeout(wait)
                        .map_err(|error| error == RecvTimeoutError::Disconnected)
                }
            };
            let mut next = first;
            loop {
                match next {
                    Ok(command) => {
                        if !self.handle(command) {
                            return self.stop();
                        }
                    }
                    Err(true) => return self.stop(),
                    Err(false) => break,
                }
                next = inbox.try_recv().map_err(|error| error == TryRecvError::Disconnected);
            }
            self.drain(self.shared.ring.position());
            let now = self.now_ms();
            self.session.tick(now);
        }
    }

    /// Hands the session every block of audio written before ring position `until`.
    fn drain(&mut self, until: usize) {
        loop {
            self.samples.clear();
            let Some(sample_rate) = self.shared.ring.read_before(until, &mut self.samples) else {
                return;
            };
            self.pcm.clear();
            self.converter
                .convert_samples(sample_rate, &self.samples, &mut self.pcm);
            let now = self.now_ms();
            self.session.audio(&self.pcm, now);
        }
    }

    /// Applies one command; false once the thread should end.
    fn handle(&mut self, command: Command) -> bool {
        match command {
            Command::Talk { down, at } => {
                self.drain(at);
                // A push-to-talk turn is a new stretch of audio; always-on streams on.
                if down && self.session.config().mode == Mode::PushToTalk {
                    self.converter.reset();
                }
                let now = self.now_ms();
                self.session.talk(down, now);
            }
            Command::Configure { config, generation, at } => {
                self.drain(at);
                self.applied = generation;
                let now = self.now_ms();
                self.session.configure(config, now);
            }
            Command::Opened { attempt, result } => self.opened(attempt, result),
            Command::Shutdown => return false,
        }
        true
    }

    /// Carries out the session's actions until it asks for nothing more.
    fn act(&mut self) {
        loop {
            let actions = self.session.take_actions();
            if actions.is_empty() {
                break;
            }
            for action in actions {
                match action {
                    Action::Connect => self.connect(),
                    Action::Send { pcm, commit } => {
                        let Some(link) = self.link.as_mut() else {
                            continue;
                        };
                        if let Err(failure) = link.send(audio_message(&pcm, commit)) {
                            self.link = None;
                            let now = self.now_ms();
                            self.session.failed(failure, now);
                        }
                    }
                    Action::Disconnect => {
                        self.attempt += 1;
                        self.abort_openings();
                        if let Some(link) = self.link.take() {
                            link.close();
                        }
                    }
                    Action::Emit(event) => self.emit(event),
                }
            }
        }
        let streaming = self.session.wants_audio() && self.session.config().mode == Mode::AlwaysOn;
        let may_listen = self.session.config().blocker().is_none();
        let shared = &self.shared;
        shared.allow(&shared.streaming, streaming, self.applied);
        shared.allow(&shared.may_listen, may_listen, self.applied);
    }

    fn emit(&mut self, event: VoiceEvent) {
        match &event {
            VoiceEvent::Status(status) => {
                *self.shared.status.lock().unwrap_or_else(PoisonError::into_inner) = status.clone();
                log::info!("voice: {}", status.describe());
            }
            VoiceEvent::Heard(_) | VoiceEvent::Language(_) | VoiceEvent::Usage { .. } => {}
        }
        if catch_unwind(AssertUnwindSafe(|| (self.on_event)(event))).is_err() {
            log::error!("voice: the event handler panicked");
        }
    }

    /// Closes the session and waits for every opening thread, aborting what it was opening.
    fn stop(mut self) {
        self.session.shutdown();
        self.act();
        self.abort_openings();
        self.join_openings();
    }
}
