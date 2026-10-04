// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises refusals: HTTP refusals of the WebSocket handshake against the fake Scribe server,
//! where a refused key stops and a rate limit or an outage retries with backoff, and server
//! messages that quote the key, which never show it, not even in part across the cut of a long
//! message.

mod common;

use std::thread;
use std::time::Duration;

use common::recorder::{Recorder, has_status, tone};
use common::{KEY, config};
use sceneuron_voice::fake::{FakeScribe, FakeScript};
use sceneuron_voice::session::Session;
use sceneuron_voice::{ErrorKind, Mode, ServerMessage, Transport, Voice, VoiceStatus};

/// A fake that refuses the first handshakes as `http_refusals` say, and a held push to talk.
fn held(http_refusals: Vec<Option<(u16, String)>>) -> (FakeScribe, Voice, Recorder) {
    let fake = FakeScribe::start(FakeScript {
        key: KEY.to_owned(),
        http_refusals,
        ..FakeScript::default()
    })
    .unwrap();
    let mut config = config(Mode::PushToTalk);
    config.endpoint = fake.endpoint();
    let recorder = Recorder::default();
    let voice = Voice::start(config, Transport::default(), recorder.sink());
    voice.talk(true);
    for chunk in tone(16_000, 1, 0.2) {
        voice.input().feed(&chunk);
    }
    (fake, voice, recorder)
}

#[test]
fn a_refused_key_or_address_stops_without_retrying() {
    for (status, expected) in [
        (401, "ElevenLabs refused the API key (HTTP 401)"),
        (403, "ElevenLabs refused the API key (HTTP 403)"),
        (404, "ElevenLabs answered HTTP 404 no such path"),
    ] {
        let (fake, voice, recorder) = held(vec![Some((status, "no such path".to_owned()))]);
        let stopped = |status: &VoiceStatus| matches!(status, VoiceStatus::Stopped { .. });
        recorder.wait_for("a stop", |events| has_status(events, stopped));
        assert_eq!(
            voice.status(),
            VoiceStatus::Stopped {
                message: expected.to_owned()
            }
        );
        thread::sleep(Duration::from_millis(1_300));
        assert_eq!(fake.sessions().len(), 1, "HTTP {status} is not retried");
        assert_eq!(fake.sessions()[0].http_refused, Some(status));
    }
}

#[test]
fn a_rate_limit_or_an_outage_retries_after_a_second() {
    for status in [429, 503] {
        let (fake, voice, recorder) = held(vec![Some((status, "try later".to_owned()))]);
        let retrying = |seen: &VoiceStatus| {
            matches!(seen, VoiceStatus::Retrying { message, retry_in_ms: 1_000 }
                if *message == format!("ElevenLabs answered HTTP {status} try later"))
        };
        recorder.wait_for("a retry", |events| has_status(events, retrying));
        recorder.wait_for("listening after the retry", |events| {
            has_status(events, VoiceStatus::is_listening)
        });
        let sessions = fake.sessions();
        assert_eq!(sessions.len(), 2, "HTTP {status}");
        assert_eq!(
            (sessions[0].http_refused, sessions[1].http_refused),
            (Some(status), None)
        );
        voice.talk(false);
    }
}

#[test]
fn a_refusal_that_quotes_the_key_never_shows_it() {
    let body = format!("{} {KEY} is over quota", "x".repeat(190));
    let (_fake, voice, recorder) = held(vec![Some((503, body)), Some((503, format!("bad {KEY}")))]);
    let retrying = |status: &VoiceStatus| matches!(status, VoiceStatus::Retrying { .. });
    recorder.wait_for("a retry", |events| has_status(events, retrying));
    recorder.wait_for("a second retry", |events| {
        common::recorder::statuses(events)
            .iter()
            .any(|status| matches!(status, VoiceStatus::Retrying { message, .. } if message.ends_with("bad [key]")))
    });
    let everything = format!("{:?} {:?}", recorder.events(), voice.status());
    assert!(!everything.contains(&KEY[..6]), "{everything}");
}

fn always_on() -> Session {
    let mut session = Session::new(config(Mode::AlwaysOn), 0);
    session.take_actions();
    session
}

#[test]
fn a_server_quoting_the_key_back_never_reaches_the_status() {
    let mut session = always_on();
    session.connected(100);
    let refusal = ServerMessage::Error {
        kind: ErrorKind::Fatal,
        code: "auth_error".to_owned(),
        message: format!("the key {KEY} is not valid"),
    };
    session.message(refusal, 200);
    let actions = session.take_actions();
    let shown = format!("{:?} {actions:?}", session.status());
    assert!(
        !shown.contains(KEY) && shown.contains("the key [key] is not valid"),
        "{shown}"
    );
}

#[test]
fn a_key_across_the_cut_of_a_long_server_message_leaves_no_part_behind() {
    // The status says "ElevenLabs refused the session (auth_error: ...)" and is cut at 200
    // characters, so every place where the key straddles the cut is tried.
    for at in 130..=200 {
        let mut session = always_on();
        session.connected(100);
        let refusal = ServerMessage::Error {
            kind: ErrorKind::Fatal,
            code: "auth_error".to_owned(),
            message: format!("{}{KEY} is not valid", "x".repeat(at)),
        };
        session.message(refusal, 200);
        let actions = session.take_actions();
        let shown = format!("{:?} {actions:?}", session.status());
        assert!(!shown.contains(&KEY[..6]), "a part of the key at {at}: {shown}");
        let VoiceStatus::Stopped { message } = session.status() else {
            panic!("not stopped: {shown}");
        };
        assert!(message.chars().count() <= 203, "the message is cut to fit: {message}");
    }
}
