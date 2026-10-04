// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the audio thread's hand-off against the fake Scribe server: a long run of small
//! pieces all arrives, a full ring refuses audio until the voice thread catches up and then
//! takes it again, and planar channels are mixed to mono as they are copied.

mod common;

use std::mem::ManuallyDrop;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use common::recorder::{Recorder, WAIT};
use common::{KEY, config};
use sceneuron_voice::fake::{FakeScribe, FakeScript};
use sceneuron_voice::{AudioChunk, Mode, Transport, Voice, VoiceEvent};

fn fake() -> FakeScribe {
    FakeScribe::start(FakeScript {
        key: KEY.to_owned(),
        ..FakeScript::default()
    })
    .unwrap()
}

fn start(fake: &FakeScribe, on_event: impl FnMut(VoiceEvent) + Send + 'static) -> Voice {
    let mut config = config(Mode::PushToTalk);
    config.endpoint = fake.endpoint();
    Voice::start(config, Transport::default(), on_event)
}

fn samples(session_audio: &[u8]) -> Vec<i16> {
    session_audio
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| i16::from_le_bytes(*pair))
        .collect()
}

#[test]
fn two_thousand_small_pieces_held_all_reach_the_server() {
    let fake = fake();
    let voice = start(&fake, Recorder::default().sink());
    let input = voice.input();
    let piece = AudioChunk {
        sample_rate: 16_000,
        samples: vec![0.25; 160],
    };
    voice.talk(true);
    for index in 0..2_000 {
        assert!(input.feed(&piece), "piece {index} was dropped");
        if index % 40 == 39 {
            thread::sleep(Duration::from_millis(2));
        }
    }
    voice.talk(false);
    let bytes = 2_000 * 160 * 2;
    fake.wait(WAIT, |sessions| {
        sessions.iter().map(|session| session.audio.len()).sum::<usize>() == bytes
    })
    .expect("every piece reaches the server");
}

#[test]
fn a_full_ring_refuses_audio_until_the_voice_thread_catches_up() {
    let fake = fake();
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let waiting = Arc::clone(&gate);
    // The voice thread stops at its first report until the gate opens, so nothing is drained.
    // A failing assertion leaves the voice running instead of waiting for its thread, which a
    // broken ring can hand a block it takes minutes to read.
    let voice = ManuallyDrop::new(start(&fake, move |_| {
        let (open, wake) = &*waiting;
        let mut open = open.lock().unwrap();
        while !*open {
            open = wake.wait(open).unwrap();
        }
    }));
    let input = voice.input();
    let block = AudioChunk {
        sample_rate: 48_000,
        samples: vec![0.1; 1024],
    };
    voice.talk(true);
    let accepted = (0..1_000).take_while(|_| input.feed(&block)).count();
    let refused_again = !input.feed(&block);
    // Opened before any assertion, so a failing one does not leave the voice thread parked.
    *gate.0.lock().unwrap() = true;
    gate.1.notify_all();
    assert!(
        (200..=300).contains(&accepted),
        "about five seconds of 48 kHz audio fit, not {accepted} blocks"
    );
    assert!(refused_again, "a full ring keeps refusing");
    let deadline = Instant::now() + WAIT;
    while !input.feed(&block) {
        assert!(Instant::now() < deadline, "the ring never emptied");
        thread::sleep(Duration::from_millis(5));
    }
    voice.talk(false);
    drop(ManuallyDrop::into_inner(voice));
}

#[test]
fn planar_channels_are_mixed_to_mono_as_they_are_copied() {
    let fake = fake();
    let voice = start(&fake, Recorder::default().sink());
    let input = voice.input();
    assert!(
        !input.feed_planar(&[&[0.5][..]], 16_000),
        "nothing is wanted before a press"
    );
    voice.talk(true);
    let left = [1.0, 0.5, 0.0, f32::NAN, 0.25];
    let right = [0.0, 0.5, -1.0, 0.5];
    assert!(input.feed_planar(&[&left[..], &right[..]], 16_000));
    assert!(!input.feed_planar(&[], 16_000), "no channels, no audio");
    voice.talk(false);
    let sessions = fake
        .wait(WAIT, |sessions| {
            sessions.first().is_some_and(|session| session.commits == 1)
        })
        .unwrap();
    assert_eq!(
        samples(&sessions[0].audio),
        [16_384, 16_384, -16_384, 8_192],
        "averaged, a bad number is silence, and the shorter channel cuts the block"
    );
}
