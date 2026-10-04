// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises language detection against the fake Scribe server: a stream whose language is left to
//! detection asks for it and reports each segment's language after its transcript, while a stream
//! with a language set does not ask and hears none.

mod common;

use common::recorder::{Recorder, finals_count, has_status, tone};
use common::{KEY, config};
use sceneuron_voice::fake::{FakeScribe, FakeScript};
use sceneuron_voice::{Mode, Transport, Voice, VoiceEvent, VoiceStatus};

fn english() -> FakeScribe {
    FakeScribe::start(FakeScript {
        key: KEY.to_owned(),
        transcripts: vec!["Good morning".to_owned(), "and welcome".to_owned()],
        vad_bytes: 16_000,
        language: "eng".to_owned(),
        ..FakeScript::default()
    })
    .unwrap()
}

/// A stream to `fake` in `language`, fed two seconds of audio; what it reported once it heard
/// both segments.
fn stream(fake: &FakeScribe, language: &str) -> Vec<VoiceEvent> {
    let mut config = config(Mode::AlwaysOn);
    config.endpoint = fake.endpoint();
    language.clone_into(&mut config.language);
    let recorder = Recorder::default();
    let voice = Voice::start(config, Transport::default(), recorder.sink());
    recorder.wait_for("listening", |events| has_status(events, VoiceStatus::is_listening));
    let input = voice.input();
    for chunk in tone(16_000, 1, 2.0) {
        assert!(input.feed(&chunk), "the stream refused audio it wants");
    }
    let events = recorder.wait_for("two segments", |events| finals_count(events) == 2);
    voice.shutdown();
    events
}

fn languages(events: &[VoiceEvent]) -> Vec<&str> {
    let reported = events.iter().filter_map(|event| match event {
        VoiceEvent::Language(code) => Some(code.as_str()),
        _ => None,
    });
    reported.collect()
}

#[test]
fn a_stream_left_to_detection_reports_each_segments_language_after_it() {
    let fake = english();
    let events = stream(&fake, "");
    assert_eq!(fake.sessions()[0].query_values("include_language_detection"), ["true"]);
    let reported = languages(&events);
    assert!(
        !reported.is_empty() && reported.iter().all(|code| *code == "eng"),
        "{reported:?}"
    );
    let first_final = events
        .iter()
        .position(|event| matches!(event, VoiceEvent::Heard(heard) if heard.is_final))
        .unwrap();
    let first_language = events
        .iter()
        .position(|event| matches!(event, VoiceEvent::Language(_)))
        .unwrap();
    assert!(
        first_final < first_language,
        "the language comes after its segment's text"
    );
}

#[test]
fn a_stream_with_a_language_set_does_not_ask() {
    let fake = english();
    let events = stream(&fake, "en");
    let session = &fake.sessions()[0];
    assert_eq!(session.query_values("language_code"), ["en"]);
    assert!(session.query_values("include_language_detection").is_empty());
    assert!(languages(&events).is_empty());
}
