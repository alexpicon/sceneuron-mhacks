// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Records what a running voice input reports, and makes test audio for it.

use std::f32::consts::PI;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use sceneuron_voice::{AudioChunk, Heard, VoiceEvent, VoiceStatus};

pub const WAIT: Duration = Duration::from_secs(10);

#[derive(Clone, Default)]
pub struct Recorder {
    events: Arc<Mutex<Vec<VoiceEvent>>>,
}

impl Recorder {
    /// An `on_event` that records into this recorder.
    pub fn sink(&self) -> impl FnMut(VoiceEvent) + Send + 'static {
        let events = Arc::clone(&self.events);
        move |event| events.lock().unwrap_or_else(PoisonError::into_inner).push(event)
    }

    pub fn events(&self) -> Vec<VoiceEvent> {
        self.events.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Waits until `ready` holds for the events so far, and returns them.
    pub fn wait_for(&self, what: &str, ready: impl Fn(&[VoiceEvent]) -> bool) -> Vec<VoiceEvent> {
        let deadline = Instant::now() + WAIT;
        loop {
            let events = self.events();
            if ready(&events) {
                return events;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {what}; events: {events:#?}"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    pub fn finals(&self) -> Vec<Heard> {
        heard(&self.events())
            .into_iter()
            .filter(|heard| heard.is_final)
            .collect()
    }

    pub fn statuses(&self) -> Vec<VoiceStatus> {
        statuses(&self.events())
    }

    /// Seconds of audio reported as sent.
    pub fn usage(&self) -> f64 {
        self.events()
            .iter()
            .filter_map(|event| match event {
                VoiceEvent::Usage { audio_seconds, .. } => Some(*audio_seconds),
                _ => None,
            })
            .sum()
    }
}

pub fn heard(events: &[VoiceEvent]) -> Vec<Heard> {
    events
        .iter()
        .filter_map(|event| match event {
            VoiceEvent::Heard(heard) => Some(heard.clone()),
            _ => None,
        })
        .collect()
}

pub fn statuses(events: &[VoiceEvent]) -> Vec<VoiceStatus> {
    events
        .iter()
        .filter_map(|event| match event {
            VoiceEvent::Status(status) => Some(status.clone()),
            _ => None,
        })
        .collect()
}

pub fn finals_count(events: &[VoiceEvent]) -> usize {
    heard(events).iter().filter(|heard| heard.is_final).count()
}

pub fn has_status(events: &[VoiceEvent], wanted: impl Fn(&VoiceStatus) -> bool) -> bool {
    statuses(events).iter().any(wanted)
}

/// `seconds` of a 440 Hz tone at half scale, as OBS hands it over: planar float chunks of 1024
/// frames, the same in every channel.
pub fn tone(rate: u32, channels: usize, seconds: f32) -> Vec<AudioChunk> {
    let frames = (rate as f32 * seconds) as usize;
    let mono: Vec<f32> = (0..frames)
        .map(|n| 0.5 * (2.0 * PI * 440.0 * n as f32 / rate as f32).sin())
        .collect();
    mono.chunks(1024)
        .map(|chunk| {
            let planes = vec![chunk; channels];
            AudioChunk::from_planar(&planes, rate)
        })
        .collect()
}
