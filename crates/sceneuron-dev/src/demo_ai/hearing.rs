// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Simulated voice in the development server: letting go of push-to-talk hears the next line of
//! a script, as if ElevenLabs had transcribed it, and each card to speak gets a short chime
//! instead of a voice. Nothing leaves the computer.

use std::collections::VecDeque;
use std::f64::consts::TAU;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use sceneuron_core::{Heard, VoiceMode};
use sceneuron_server::{Hub, HubEvent};

use super::voice::DemoVoice;

/// What the simulated microphone hears, one line per push-to-talk, in turn.
const SCRIPT: [&str; 4] = [
    "Sceneuron, wrap it up",
    "Sceneuron, my vote is eight",
    "Sceneuron, lower everything",
    "Sceneuron, next topic",
];
const SAMPLE_RATE: u32 = 16_000;
/// Simulated audio kept for the overlay, newest last.
const KEPT_AUDIO: usize = 8;

/// The simulated voice settings and the chimes made for spoken cards.
#[derive(Debug, Default)]
pub(crate) struct DemoVoiceState {
    settings: Mutex<DemoVoice>,
    audio: Mutex<VecDeque<(String, Vec<u8>)>>,
    next_line: AtomicUsize,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl DemoVoiceState {
    pub(crate) fn settings(&self) -> MutexGuard<'_, DemoVoice> {
        lock(&self.settings)
    }

    /// The chime made for a spoken card, by its id.
    pub(crate) fn audio(&self, id: &str) -> Option<Vec<u8>> {
        let audio = lock(&self.audio);
        audio.iter().find(|(known, _)| known == id).map(|(_, wav)| wav.clone())
    }

    fn keep(&self, id: String, wav: Vec<u8>) {
        let mut audio = lock(&self.audio);
        if audio.len() == KEPT_AUDIO {
            audio.pop_front();
        }
        audio.push_back((id, wav));
    }
}

/// Reports the simulated voice status, and from then on hears a scripted line whenever
/// push-to-talk is let go and makes a chime for each new card to speak.
pub(crate) fn follow(hub: &Arc<Hub>, state: &Arc<DemoVoiceState>) {
    let status = state.settings().status();
    hub.update(|engine, now| engine.set_voice_status(status, now));
    let (hub_ref, state) = (Arc::downgrade(hub), Arc::clone(state));
    let talking = AtomicBool::new(false);
    let spoken = AtomicU64::new(0);
    hub.add_listener(Box::new(move |event| {
        let Some(hub) = hub_ref
            .upgrade()
            .filter(|_| matches!(event, HubEvent::StateChanged { .. }))
        else {
            return;
        };
        let (now_talking, cue) = hub.read(|engine| {
            let cue = engine.speech().filter(|cue| cue.audio.is_none()).map(|cue| cue.nonce);
            (engine.talking(), cue)
        });
        let voice = state.settings().clone();
        if talking.swap(now_talking, Ordering::SeqCst) && !now_talking && voice.enabled {
            let line = SCRIPT[state.next_line.fetch_add(1, Ordering::SeqCst) % SCRIPT.len()];
            let heard = Heard {
                text: line.to_owned(),
                is_final: true,
                addressed: voice.mode == VoiceMode::PushToTalk,
            };
            hub.update(|engine, now| engine.hear(&heard, now));
        }
        // A cue is handled once, when it is new, like the real speaker: turning spoken cards on later
        // does not speak a card raised before.
        if let Some(nonce) = cue
            && spoken.fetch_max(nonce, Ordering::SeqCst) < nonce
            && voice.speak
            && voice.key_hint.is_some()
        {
            let id = format!("{nonce:064x}");
            state.keep(id.clone(), chime());
            hub.update(|engine, now| engine.speech_ready(nonce, &id, now));
        }
    }));
}

/// A short rising two-note chime as a 16-bit mono WAV file.
fn chime() -> Vec<u8> {
    let notes = [(660.0, 180), (880.0, 220)];
    let mut samples: Vec<i16> = Vec::new();
    for (frequency, ms) in notes {
        let count = (SAMPLE_RATE * ms / 1_000) as usize;
        samples.extend((0..count).map(|n| {
            let t = n as f64 / f64::from(SAMPLE_RATE);
            let fade = (1.0 - n as f64 / count as f64).min(n as f64 / 160.0).min(1.0);
            ((TAU * frequency * t).sin() * fade * 9_000.0) as i16
        }));
    }
    let data_bytes = u32::try_from(samples.len() * 2).unwrap_or(u32::MAX);
    let mut wav = Vec::with_capacity(44 + samples.len() * 2);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chime_is_a_well_formed_wav_file() {
        let wav = chime();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        let size = u32::from_le_bytes(wav[4..8].try_into().unwrap()) as usize;
        assert_eq!(size + 8, wav.len());
        let data = u32::from_le_bytes(wav[40..44].try_into().unwrap()) as usize;
        assert_eq!(data, wav.len() - 44);
        assert_eq!(data, 2 * (16_000 * 18 / 100 + 16_000 * 22 / 100));
    }

    #[test]
    fn only_the_newest_chimes_are_kept() {
        let state = DemoVoiceState::default();
        for n in 0..10u8 {
            state.keep(format!("{n:064x}"), vec![n]);
        }
        assert_eq!(state.audio(&format!("{:064x}", 1)), None);
        assert_eq!(state.audio(&format!("{:064x}", 9)), Some(vec![9]));
    }
}
