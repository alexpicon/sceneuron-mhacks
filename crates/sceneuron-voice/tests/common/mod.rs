// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Shared helpers for the voice tests: configs, made-up audio, and what a session asked for.

#![allow(dead_code)]

pub mod recorder;

use sceneuron_voice::session::Action;
use sceneuron_voice::{ApiKey, Heard, Mode, VoiceConfig, VoiceEvent, VoiceStatus};

pub const KEY: &str = "sk_test_0123456789abcdef";

/// A config that may send audio.
pub fn config(mode: Mode) -> VoiceConfig {
    VoiceConfig {
        enabled: true,
        consent: true,
        mode,
        keyterms: vec!["Sceneuron".to_owned(), "Wrap it up".to_owned()],
        api_key: ApiKey::new(KEY),
        ..VoiceConfig::default()
    }
}

/// `ms` milliseconds of converted 16 kHz audio.
pub fn pcm(ms: usize) -> Vec<u8> {
    vec![7; ms * 32]
}

/// The sends among `actions`, as (bytes, commit).
pub fn sends(actions: &[Action]) -> Vec<(usize, bool)> {
    actions
        .iter()
        .filter_map(|action| match action {
            Action::Send { pcm, commit } => Some((pcm.len(), *commit)),
            _ => None,
        })
        .collect()
}

pub fn count(actions: &[Action], wanted: &Action) -> usize {
    actions.iter().filter(|action| *action == wanted).count()
}

pub fn heard(actions: &[Action]) -> Vec<Heard> {
    actions
        .iter()
        .filter_map(|action| match action {
            Action::Emit(VoiceEvent::Heard(heard)) => Some(heard.clone()),
            _ => None,
        })
        .collect()
}

pub fn statuses(actions: &[Action]) -> Vec<VoiceStatus> {
    actions
        .iter()
        .filter_map(|action| match action {
            Action::Emit(VoiceEvent::Status(status)) => Some(status.clone()),
            _ => None,
        })
        .collect()
}

/// Seconds of audio reported as used.
pub fn usage(actions: &[Action]) -> f64 {
    actions
        .iter()
        .filter_map(|action| match action {
            Action::Emit(VoiceEvent::Usage { audio_seconds, .. }) => Some(*audio_seconds),
            _ => None,
        })
        .sum()
}

pub fn final_text(text: &str, addressed: bool) -> Heard {
    Heard {
        text: text.to_owned(),
        is_final: true,
        addressed,
    }
}
