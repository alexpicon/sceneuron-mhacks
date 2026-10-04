// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Voice fixtures: a WAV file of a tone, the fake Scribe server that hears a command in it, and
//! a project whose meeting deck voice can raise.

use std::f32::consts::PI;
use std::path::{Path, PathBuf};

use sceneuron_core::{default_project, pack_for_canvas, save_project};
use sceneuron_voice::fake::{FakeScribe, FakeScript};
use sceneuron_voice::pcm16_wav;

use super::project_path;

pub const KEY_VAR: &str = "SCENEURON_ELEVENLABS_API_KEY";
pub const KEY: &str = "sk_dev_voice_test_0123456789";
/// Keeps a developer's own env file setting out of the tests.
pub const NO_ENV_FILE: (&str, &str) = ("SCENEURON_ENV_FILE", "");

/// A file of `seconds` of a 440 Hz tone, 48 kHz stereo, next to a fresh project path.
pub fn wav_file(name: &str, seconds: f32) -> PathBuf {
    let frames = (48_000.0 * seconds) as usize;
    let samples: Vec<i16> = (0..frames)
        .flat_map(|n| {
            let sample = (12_000.0 * (2.0 * PI * 440.0 * n as f32 / 48_000.0).sin()) as i16;
            [sample, sample]
        })
        .collect();
    let path = project_path(name).with_file_name("speech.wav");
    std::fs::write(&path, pcm16_wav(48_000, 2, &samples)).unwrap();
    path
}

pub fn fake() -> FakeScribe {
    FakeScribe::start(FakeScript {
        key: KEY.to_owned(),
        transcripts: vec!["Sceneuron, wrap it up".to_owned()],
        ..FakeScript::default()
    })
    .unwrap()
}

/// A project next to `wav` with the meeting pack, whose voice is on, and the poker pack, whose
/// voice is off.
pub fn meeting_project(wav: &Path) -> PathBuf {
    let path = wav.with_file_name("project.json");
    let base = default_project();
    let mut project = base
        .add_pack(&pack_for_canvas("meeting", &base.canvas).unwrap())
        .unwrap()
        .add_pack(&pack_for_canvas("poker", &base.canvas).unwrap())
        .unwrap();
    project.decks[0].voice = true;
    save_project(&path, &project).unwrap();
    path
}
