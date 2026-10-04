// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises push to talk in always-on mode against the fake Scribe server: the audio is split
//! at the press and the release, wherever the voice thread happens to be, so the turn's session
//! gets exactly the held audio and its transcript is addressed, while the streams before and
//! after get the rest.

mod common;

use common::recorder::{Recorder, WAIT, finals_count, has_status};
use common::{KEY, config, final_text};
use sceneuron_voice::fake::{FakeScribe, FakeScript};
use sceneuron_voice::{AudioChunk, Mode, Transport, Voice, VoiceStatus};

/// `ms` of audio at 16 kHz in 10 ms pieces, at `level`.
fn speech(ms: usize, level: f32) -> Vec<AudioChunk> {
    (0..ms / 10)
        .map(|_| AudioChunk {
            sample_rate: 16_000,
            samples: vec![level; 160],
        })
        .collect()
}

#[test]
fn held_audio_goes_to_a_turn_of_its_own_split_exactly_at_the_press_and_release() {
    let fake = FakeScribe::start(FakeScript {
        key: KEY.to_owned(),
        transcripts: vec!["wrap it up".to_owned()],
        // Voice activity never ends a segment here, so only the turn's commit is answered.
        vad_bytes: 100_000_000,
        ..FakeScript::default()
    })
    .unwrap();
    let mut config = config(Mode::AlwaysOn);
    config.endpoint = fake.endpoint();
    let recorder = Recorder::default();
    let voice = Voice::start(config, Transport::default(), recorder.sink());
    recorder.wait_for("listening", |events| has_status(events, VoiceStatus::is_listening));
    let input = voice.input();
    let feed = |chunks: Vec<AudioChunk>| {
        for chunk in chunks {
            assert!(input.feed(&chunk));
        }
    };
    feed(speech(1_000, 0.1));
    voice.talk(true);
    feed(speech(500, 0.5));
    voice.talk(false);
    feed(speech(500, 0.2));
    recorder.wait_for("the turn's transcript", |events| finals_count(events) == 1);
    assert_eq!(recorder.finals(), [final_text("wrap it up", true)]);
    let sessions = fake
        .wait(WAIT, |sessions| {
            sessions.len() == 3 && sessions[2].audio.len() == 16_000
        })
        .unwrap_or_else(|sessions| panic!("{} sessions", sessions.len()));
    let strategies: Vec<&str> = sessions
        .iter()
        .map(|session| session.query_values("commit_strategy")[0])
        .collect();
    assert_eq!(strategies, ["vad", "manual", "vad"]);
    let turn = &sessions[1];
    assert_eq!(
        (turn.audio.len(), turn.commits),
        (16_000, 1),
        "exactly the half second held, committed once"
    );
    let held = i16::from_le_bytes([turn.audio[0], turn.audio[1]]);
    assert_eq!(
        held,
        (0.5f32 * 32_767.0).round() as i16,
        "no audio from before the press"
    );
    let after = i16::from_le_bytes([sessions[2].audio[0], sessions[2].audio[1]]);
    assert_eq!(
        after,
        (0.2f32 * 32_767.0).round() as i16,
        "the stream resumes with what followed"
    );
    assert_eq!(
        sessions[0].audio.len(),
        32_000,
        "the second before the press went to the stream"
    );
    assert_eq!(sessions[0].commits + sessions[2].commits, 0);
    voice.shutdown();
}
