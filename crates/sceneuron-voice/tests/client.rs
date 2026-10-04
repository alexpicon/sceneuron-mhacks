// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the running voice input against the in-process fake Scribe server: a push-to-talk
//! turn end to end, always-on streaming with the wake word and a reconnect, refusals, consent,
//! and that the key travels only in its header.

mod common;

use std::thread;
use std::time::Duration;

use common::recorder::{Recorder, WAIT, finals_count, has_status, tone};
use common::{KEY, config, final_text};
use sceneuron_voice::fake::{FakeScribe, FakeScript, FakeSession};
use sceneuron_voice::{AudioChunk, Heard, Mode, Transport, Voice, VoiceConfig, VoiceEvent, VoiceStatus};

fn script(transcripts: &[&str]) -> FakeScript {
    FakeScript {
        key: KEY.to_owned(),
        transcripts: transcripts.iter().map(|text| (*text).to_owned()).collect(),
        ..FakeScript::default()
    }
}

fn start(fake: &FakeScribe, mut config: VoiceConfig) -> (Voice, Recorder) {
    config.endpoint = fake.endpoint();
    let recorder = Recorder::default();
    let voice = Voice::start(config, Transport::default(), recorder.sink());
    (voice, recorder)
}

fn feed(voice: &Voice, chunks: Vec<AudioChunk>) {
    let input = voice.input();
    for chunk in chunks {
        assert!(input.feed(&chunk), "the voice input refused audio it wants");
    }
}

fn decoded(session: &FakeSession) -> Vec<i16> {
    session
        .audio
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| i16::from_le_bytes(*pair))
        .collect()
}

#[test]
fn a_push_to_talk_turn_goes_from_microphone_audio_to_an_addressed_transcript() {
    let fake = FakeScribe::start(script(&["Sceneuron, wrap it up"])).unwrap();
    let (voice, recorder) = start(&fake, config(Mode::PushToTalk));
    let input = voice.input();
    assert!(!input.wants_audio(), "nothing is captured before a press");
    voice.talk(true);
    assert!(input.wants_audio());
    feed(&voice, tone(48_000, 2, 1.0));
    recorder.wait_for("listening", |events| has_status(events, VoiceStatus::is_listening));
    voice.talk(false);
    recorder.wait_for("the final transcript", |events| finals_count(events) == 1);
    let partial = Heard {
        text: "Sceneuron,".to_owned(),
        is_final: false,
        addressed: true,
    };
    let heard = common::recorder::heard(&recorder.events());
    assert_eq!(heard, vec![partial, final_text("Sceneuron, wrap it up", true)]);

    let sessions = fake
        .wait(WAIT, |sessions| sessions.len() == 1 && sessions[0].commits == 1)
        .unwrap();
    let session = &sessions[0];
    assert_eq!(session.path, "/v1/speech-to-text/realtime");
    assert_eq!(session.key.as_deref(), Some(KEY));
    assert_eq!(session.query_values("model_id"), ["scribe_v2_realtime"]);
    assert_eq!(session.query_values("audio_format"), ["pcm_16000"]);
    assert_eq!(session.query_values("commit_strategy"), ["manual"]);
    assert_eq!(session.query_values("keyterms"), ["Sceneuron", "Wrap it up"]);
    assert_eq!(session.sample_rates.iter().copied().collect::<Vec<_>>(), [16_000]);
    let samples = decoded(session);
    assert!(
        (15_900..=16_000).contains(&samples.len()),
        "{} samples for one second",
        samples.len()
    );
    let middle = &samples[4_000..12_000];
    let rms = (middle.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / middle.len() as f64).sqrt();
    assert!(
        (rms - 0.5 / 2f64.sqrt() * 32767.0).abs() < 200.0,
        "the tone arrived at {rms}"
    );
    assert!((recorder.usage() - samples.len() as f64 / 16_000.0).abs() < 1e-6);

    let statuses = recorder.statuses();
    let expected = [
        VoiceStatus::Ready,
        VoiceStatus::Connecting,
        VoiceStatus::Listening,
        VoiceStatus::Ready,
    ];
    assert_eq!(statuses, expected, "events: {:#?}", recorder.events());
    assert!(!input.wants_audio(), "nothing is captured after the release");
    voice.shutdown();
    voice.shutdown();
    fake.wait(WAIT, |sessions| sessions[0].ended)
        .expect("shutting down closes the session");
}

#[test]
fn always_on_streams_leaves_the_wake_word_to_the_engine_and_reconnects_after_the_server_closes() {
    let mut fake_script = script(&["Sceneuron, lower", "let's take this offline"]);
    fake_script.vad_bytes = 16_000;
    fake_script.close_after = vec![Some(8)];
    let fake = FakeScribe::start(fake_script).unwrap();
    let (voice, recorder) = start(&fake, config(Mode::AlwaysOn));
    let listening = |events: &[VoiceEvent]| has_status(events, VoiceStatus::is_listening);
    recorder.wait_for("listening", listening);
    assert!(voice.input().wants_audio(), "always-on captures without a press");
    feed(&voice, tone(16_000, 1, 1.0));
    recorder.wait_for("the first transcript", |events| finals_count(events) == 1);
    let retrying = |status: &VoiceStatus| matches!(status, VoiceStatus::Retrying { retry_in_ms: 1_000, .. });
    recorder.wait_for("a retry", |events| has_status(events, retrying));
    recorder.wait_for("listening again", |events| {
        common::recorder::statuses(events)
            .iter()
            .filter(|status| status.is_listening())
            .count()
            == 2
    });
    feed(&voice, tone(16_000, 1, 1.0));
    recorder.wait_for("the second transcript", |events| finals_count(events) == 2);
    let expected = vec![
        final_text("Sceneuron, lower", false),
        final_text("let's take this offline", false),
    ];
    assert_eq!(recorder.finals(), expected);
    let sessions = fake.sessions();
    assert_eq!(sessions.len(), 2);
    assert!(
        sessions
            .iter()
            .all(|session| session.query_values("commit_strategy") == ["vad"])
    );
    assert_eq!(sessions[0].audio_messages, 8);
    assert_eq!(sessions[0].commits, 0, "voice activity decides where segments end");
}

#[test]
fn a_refused_key_stops_without_retrying_and_without_echoing_the_key() {
    let fake = FakeScribe::start(FakeScript {
        key: "sk_the_right_key".to_owned(),
        ..FakeScript::default()
    })
    .unwrap();
    let (voice, recorder) = start(&fake, config(Mode::PushToTalk));
    voice.talk(true);
    feed(&voice, tone(48_000, 1, 0.3));
    voice.talk(false);
    let stopped = |status: &VoiceStatus| matches!(status, VoiceStatus::Stopped { .. });
    recorder.wait_for("a stop", |events| has_status(events, stopped));
    let VoiceStatus::Stopped { message } = voice.status() else {
        panic!("not stopped: {:?}", voice.status());
    };
    assert!(message.contains("auth_error"), "{message}");
    thread::sleep(Duration::from_millis(1_500));
    let sessions = fake.sessions();
    assert_eq!(sessions.len(), 1, "a refused key is not retried");
    assert!(sessions[0].audio.is_empty(), "no audio follows a refusal");
    let everything = format!("{:?}", recorder.events());
    assert!(!everything.contains(KEY), "{everything}");
    assert!(!sessions[0].path.contains(KEY) && sessions[0].query.iter().all(|(_, value)| !value.contains(KEY)));
}

#[test]
fn without_consent_nothing_is_captured_or_sent() {
    let fake = FakeScribe::start(script(&["never"])).unwrap();
    for mode in [Mode::PushToTalk, Mode::AlwaysOn] {
        let config = VoiceConfig {
            consent: false,
            ..config(mode)
        };
        let (voice, _recorder) = start(&fake, config);
        voice.talk(true);
        let input = voice.input();
        assert!(!input.wants_audio());
        assert!(tone(48_000, 2, 0.2).iter().all(|chunk| !input.feed(chunk)));
        voice.talk(false);
        thread::sleep(Duration::from_millis(300));
        assert_eq!(voice.status(), VoiceStatus::NeedsConsent);
    }
    assert!(fake.sessions().is_empty());
}

#[test]
fn withdrawing_consent_ends_an_always_on_session() {
    let fake = FakeScribe::start(script(&[])).unwrap();
    let (voice, recorder) = start(&fake, config(Mode::AlwaysOn));
    recorder.wait_for("listening", |events| has_status(events, VoiceStatus::is_listening));
    let mut withdrawn = config(Mode::AlwaysOn);
    withdrawn.consent = false;
    withdrawn.endpoint = fake.endpoint();
    voice.configure(withdrawn);
    assert!(
        !voice.input().wants_audio(),
        "capture stops before the voice thread catches up"
    );
    fake.wait(WAIT, |sessions| sessions.len() == 1 && sessions[0].ended)
        .expect("the session closes");
    recorder.wait_for("the consent status", |events| {
        has_status(events, |status| *status == VoiceStatus::NeedsConsent)
    });
}

#[test]
fn audio_said_while_held_is_never_dropped_however_fast_the_presses() {
    let fake = FakeScribe::start(script(&[])).unwrap();
    let (voice, _recorder) = start(&fake, config(Mode::PushToTalk));
    let input = voice.input();
    let chunk = AudioChunk {
        sample_rate: 16_000,
        samples: vec![0.25; 160],
    };
    let (presses, chunks) = (30, 10);
    for press in 0..presses {
        voice.talk(true);
        for piece in 0..chunks {
            assert!(input.feed(&chunk), "press {press} dropped piece {piece}");
        }
        voice.talk(false);
        assert!(!input.wants_audio(), "capture stops with the release");
    }
    let bytes = presses * chunks * 160 * 2;
    fake.wait(WAIT, |sessions| {
        sessions.iter().map(|session| session.audio.len()).sum::<usize>() == bytes
    })
    .expect("every piece reaches the server");
}

#[test]
fn plain_websockets_are_refused_for_other_computers() {
    let recorder = Recorder::default();
    let config = VoiceConfig {
        endpoint: "ws://speech.example.com/v1/speech-to-text/realtime".to_owned(),
        ..config(Mode::PushToTalk)
    };
    let voice = Voice::start(config, Transport::default(), recorder.sink());
    voice.talk(true);
    let stopped =
        |status: &VoiceStatus| matches!(status, VoiceStatus::Stopped { message } if message.contains("wss://"));
    recorder.wait_for("a refusal", |events| has_status(events, stopped));
}
