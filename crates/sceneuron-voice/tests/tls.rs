// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the TLS path against the fake Scribe server with a test certificate authority: a
//! turn over `wss://` with that authority trusted, and a refusal of the same server with the
//! bundled web PKI roots, which do not include it.

mod common;

use common::recorder::{Recorder, finals_count, has_status, tone};
use common::{KEY, config, final_text};
use sceneuron_voice::fake::{FakeScribe, FakeScript, tls_config};
use sceneuron_voice::{Mode, Transport, Voice, VoiceStatus};

const TEST_CA: &[u8] = include_bytes!("fixtures/test-ca.der");
const LOCALHOST: &[u8] = include_bytes!("fixtures/localhost.der");
const LOCALHOST_KEY: &[u8] = include_bytes!("fixtures/localhost-key.der");

fn fake() -> FakeScribe {
    let script = FakeScript {
        key: KEY.to_owned(),
        transcripts: vec!["vote eight".to_owned()],
        ..FakeScript::default()
    };
    FakeScribe::start_tls(script, tls_config(LOCALHOST, LOCALHOST_KEY).unwrap()).unwrap()
}

#[test]
fn a_turn_works_over_tls_with_the_server_trusted() {
    let fake = fake();
    assert!(fake.endpoint().starts_with("wss://localhost:"));
    let mut config = config(Mode::PushToTalk);
    config.endpoint = fake.endpoint();
    let recorder = Recorder::default();
    let voice = Voice::start(config, Transport::trusting(TEST_CA).unwrap(), recorder.sink());
    voice.talk(true);
    let input = voice.input();
    for chunk in tone(44_100, 1, 0.5) {
        assert!(input.feed(&chunk));
    }
    recorder.wait_for("listening", |events| has_status(events, VoiceStatus::is_listening));
    voice.talk(false);
    recorder.wait_for("the transcript", |events| finals_count(events) == 1);
    assert_eq!(recorder.finals(), vec![final_text("vote eight", true)]);
    let sessions = fake.sessions();
    assert_eq!(sessions[0].key.as_deref(), Some(KEY));
    assert!((7_900..=8_000).contains(&(sessions[0].audio.len() / 2)));
}

#[test]
fn an_untrusted_certificate_is_refused_and_retried_later() {
    let fake = fake();
    let mut config = config(Mode::AlwaysOn);
    config.endpoint = fake.endpoint();
    let recorder = Recorder::default();
    let _voice = Voice::start(config, Transport::default(), recorder.sink());
    let refused = |status: &VoiceStatus| matches!(status, VoiceStatus::Retrying { message, .. } if message.contains("certificate"));
    recorder.wait_for("a certificate refusal", |events| has_status(events, refused));
    assert!(fake.sessions().is_empty(), "no WebSocket session started");
}

#[test]
fn a_certificate_that_is_not_der_cannot_be_trusted() {
    assert!(Transport::trusting(b"not a certificate").is_err());
}
