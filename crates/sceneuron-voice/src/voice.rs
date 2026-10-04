// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The voice input's handles: [`Voice`] for settings, push to talk, and status, and
//! [`VoiceInput`] for the audio thread.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};

use crate::audio::AudioChunk;
use crate::config::VoiceConfig;
use crate::ring::AudioRing;
use crate::status::{VoiceEvent, VoiceStatus};
use crate::transport::Transport;
use crate::worker::{Command, Shared, Worker};

/// Samples the ring between the audio thread and the voice thread holds, headers included:
/// about five seconds of 48 kHz audio, far more than the voice thread falls behind.
const RING_SLOTS: usize = 1 << 18;

/// The running voice input. Dropping it closes any session and ends its thread.
pub struct Voice {
    commands: Sender<Command>,
    shared: Arc<Shared>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Voice {
    /// Starts the voice thread. `on_event` receives every transcript, status change, and usage
    /// report, in order, on that thread; it should return quickly.
    pub fn start(
        config: VoiceConfig,
        transport: Transport,
        on_event: impl FnMut(VoiceEvent) + Send + 'static,
    ) -> Voice {
        let (commands, inbox) = mpsc::channel();
        let shared = Arc::new(Shared {
            streaming: AtomicBool::new(false),
            held: AtomicBool::new(false),
            may_listen: AtomicBool::new(config.blocker().is_none()),
            generation: AtomicU64::new(0),
            ring: AudioRing::new(RING_SLOTS),
            status: Mutex::new(config.blocker().unwrap_or(VoiceStatus::Ready)),
        });
        let worker = Worker::new(config, transport, commands.clone(), Arc::clone(&shared), on_event);
        let thread = thread::Builder::new()
            .name("sceneuron-voice".to_owned())
            .spawn(move || worker.run(inbox))
            .map_err(|error| log::error!("voice: cannot start its thread: {error}"))
            .ok();
        Voice {
            commands,
            shared,
            thread: Mutex::new(thread),
        }
    }

    /// A handle for the audio thread.
    pub fn input(&self) -> VoiceInput {
        VoiceInput {
            shared: Arc::clone(&self.shared),
        }
    }

    /// Push to talk pressed or released, at the point in the audio handed over so far. Capture
    /// starts and stops at once, before the voice thread sees it; a press captures only when
    /// the settings allow audio to leave the computer.
    pub fn talk(&self, down: bool) {
        let at = self.shared.ring.position();
        let _ = self.commands.send(Command::Talk { down, at });
        self.shared.held.store(down, Ordering::Release);
    }

    /// New settings, which apply to the audio handed over from now on. Settings that forbid
    /// sending stop capture at once.
    pub fn configure(&self, config: VoiceConfig) {
        let generation = self.shared.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let at = self.shared.ring.position();
        if config.blocker().is_some() {
            self.shared.may_listen.store(false, Ordering::Release);
            self.shared.streaming.store(false, Ordering::Release);
        }
        let _ = self.commands.send(Command::Configure { config, generation, at });
    }

    pub fn status(&self) -> VoiceStatus {
        self.shared
            .status
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Closes any session and waits for the voice thread to end. Safe to call more than once.
    pub fn shutdown(&self) {
        self.shared.streaming.store(false, Ordering::Release);
        self.shared.may_listen.store(false, Ordering::Release);
        let thread = self.thread.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(thread) = thread {
            let _ = self.commands.send(Command::Shutdown);
            let _ = thread.join();
        }
    }
}

impl Drop for Voice {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Hands audio to the voice input without ever waiting: it reads atomic flags and copies the
/// samples into a ring allocated once, so it never locks, allocates, or makes a system call,
/// and is safe on a real-time audio thread. One thread at a time may feed it; a block that
/// arrives while another thread writes, or that does not fit, is dropped.
#[derive(Clone)]
pub struct VoiceInput {
    shared: Arc<Shared>,
}

impl VoiceInput {
    /// Whether audio is wanted now; check it before copying anything.
    pub fn wants_audio(&self) -> bool {
        self.shared.capturing()
    }

    /// Hands over mono `chunk`; false when it was not wanted or did not fit, and was dropped.
    pub fn feed(&self, chunk: &AudioChunk) -> bool {
        self.wants_audio()
            && self
                .shared
                .ring
                .write(chunk.sample_rate, chunk.samples.len(), |n| chunk.samples[n])
    }

    /// Hands over planar float channels, OBS's layout, mixed down to mono as they are copied.
    /// Channels shorter than the first cut the block to their length.
    pub fn feed_planar(&self, planes: &[&[f32]], sample_rate: u32) -> bool {
        if planes.is_empty() || !self.wants_audio() {
            return false;
        }
        let frames = planes.iter().map(|plane| plane.len()).min().unwrap_or(0);
        let scale = 1.0 / planes.len() as f32;
        self.shared.ring.write(sample_rate, frames, |n| {
            planes.iter().map(|plane| finite(plane[n])).sum::<f32>() * scale
        })
    }
}

fn finite(sample: f32) -> f32 {
    if sample.is_finite() { sample } else { 0.0 }
}
