// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the speaker inside the assistant against a scripted ElevenLabs server: cached
//! lines cost nothing, new ones count toward the totals and the hourly budget, and nothing is
//! spoken without spoken cards on, a key, or budget.

mod common;

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use common::director::connector;
use common::server::{MockServer, Reply};
use common::sink::{Received, RecordingSink, wait_until};
use common::speech::{EL_KEY, audio_reply, default_voices, mp3};
use common::temp_dir;
use sceneuron_ai::{AiSettingsUpdate, Assistant, Credentials, MockProvider, SpeechCache};
use sceneuron_core::AiPolicy;
use serde_json::json;

fn speaking(server: &MockServer) -> (Assistant, Arc<RecordingSink>) {
    let sink = Arc::new(RecordingSink::default());
    let path = temp_dir("speaker").join("ai.json");
    let mock = Arc::new(MockProvider::new(|_| Ok(json!({}))));
    let (assistant, warning) = Assistant::with_elevenlabs_url(
        path,
        connector(&mock),
        Arc::clone(&sink) as _,
        Credentials::default(),
        &server.url(),
    );
    assert_eq!(warning, None);
    (assistant, sink)
}

fn update(assistant: &Assistant, value: serde_json::Value) {
    let update: AiSettingsUpdate = serde_json::from_value(value).unwrap();
    assistant.update_settings(update).unwrap();
}

#[test]
fn a_line_is_generated_once_cached_and_counted() {
    let server = MockServer::start(vec![default_voices(), audio_reply()]);
    let (assistant, sink) = speaking(&server);
    update(
        &assistant,
        json!({"elevenlabs_api_key": EL_KEY, "voice": {"speak": true}}),
    );
    assistant.speak(7, "Wrap it up");
    assert!(wait_until(|| sink.spoken().len() == 1), "{:?}", sink.voice_events());
    let (nonce, id) = sink.spoken()[0].clone();
    assert_eq!(
        (nonce, id.as_str()),
        (
            7,
            SpeechCache::id("Wrap it up", "JBFqnCBsd6RMkjVDRZzb", "eleven_flash_v2_5").as_str()
        )
    );
    assert_eq!(assistant.speech(&id), Some(mp3()));
    assistant.speak(8, "Wrap it up");
    assert!(wait_until(|| sink.spoken().len() == 2));
    assert_eq!(sink.spoken()[1], (8, id));
    let paths: Vec<String> = server.requests().into_iter().map(|request| request.path).collect();
    assert_eq!(
        paths,
        [
            "/v2/voices?page_size=100",
            "/v1/text-to-speech/JBFqnCBsd6RMkjVDRZzb?output_format=mp3_44100_128"
        ],
        "the voice is checked once, and the second time comes from the cache"
    );
    let counted = wait_until(|| sink.last_status().is_some_and(|status| status.voice_characters == 10));
    assert!(counted, "{:?}", sink.last_status());
    let status = sink.last_status().unwrap();
    assert!(
        (status.voice_cost_usd - 0.0004).abs() < 1e-12,
        "{}",
        status.voice_cost_usd
    );
    assert!((status.cost_usd - 0.0004).abs() < 1e-12 && (status.hour_cost_usd - 0.0004).abs() < 1e-12);
    assert!(sink.speech_failures().is_empty());
    assistant.shutdown();
}

#[test]
fn nothing_is_spoken_while_spoken_cards_are_off() {
    let server = MockServer::start(vec![audio_reply()]);
    let (assistant, sink) = speaking(&server);
    update(&assistant, json!({"elevenlabs_api_key": EL_KEY}));
    assistant.speak(1, "Agree");
    thread::sleep(Duration::from_millis(300));
    assert!(sink.spoken().is_empty() && sink.speech_failures().is_empty());
    assert!(server.requests().is_empty());
    assistant.shutdown();
}

#[test]
fn without_a_key_spoken_cards_say_so_once_per_settings_change() {
    let server = MockServer::start(vec![default_voices(), audio_reply()]);
    let (assistant, sink) = speaking(&server);
    update(&assistant, json!({"voice": {"speak": true}}));
    assistant.speak(1, "Agree");
    assert!(wait_until(|| sink.speech_failures().len() == 1));
    assistant.speak(2, "Agree");
    update(&assistant, json!({"elevenlabs_api_key": EL_KEY}));
    assistant.speak(3, "Agree");
    assert!(wait_until(|| sink.spoken().iter().any(|(nonce, _)| *nonce == 3)));
    assert!(
        sink.spoken().iter().all(|(nonce, _)| [2, 3].contains(nonce)),
        "{:?}",
        sink.spoken()
    );
    assert_eq!(
        sink.speech_failures(),
        ["add an ElevenLabs key in the AI tab to speak cards"]
    );
    assert_eq!(server.requests().len(), 2);
    let statuses: Vec<_> = sink
        .voice_events()
        .into_iter()
        .filter_map(|event| match event {
            Received::Voice(status) => Some(status),
            _ => None,
        })
        .collect();
    assert!(
        statuses.len() >= 3,
        "at the start, from the voice input, and per settings change: {statuses:?}"
    );
    assert_eq!(statuses.last().unwrap().message, "voice control is off");
    assistant.shutdown();
}

#[test]
fn a_spent_budget_stops_new_lines_and_failures_never_show_the_key() {
    let echo = json!({"detail": {"message": format!("bad key {EL_KEY}")}});
    let server = MockServer::start(vec![default_voices(), audio_reply(), Reply::json(401, echo)]);
    let (assistant, sink) = speaking(&server);
    let policy = AiPolicy {
        max_cost_per_hour: 0.0,
        ..AiPolicy::default()
    };
    assistant.director().set_project(Vec::new(), policy, false);
    update(
        &assistant,
        json!({"elevenlabs_api_key": EL_KEY, "voice": {"speak": true}}),
    );
    assistant.speak(1, "Agree");
    assert!(wait_until(|| sink.spoken().len() == 1));
    assert!(wait_until(|| assistant.director().over_budget()));
    assistant.speak(2, "Disagree");
    assert!(wait_until(|| !sink.speech_failures().is_empty()));
    assert_eq!(
        sink.speech_failures(),
        ["a card was not spoken: the hourly budget is spent"]
    );
    assistant.speak(3, "Agree");
    assert!(wait_until(|| sink.spoken().len() == 2), "cached lines are free");
    assistant.director().set_project(Vec::new(), AiPolicy::default(), false);
    assert!(wait_until(|| !assistant.director().over_budget()));
    assistant.speak(4, "Disagree");
    assert!(wait_until(|| sink.speech_failures().len() == 2));
    let failure = &sink.speech_failures()[1];
    assert!(
        failure.starts_with("a card was not spoken: ") && failure.contains("[redacted]"),
        "{failure}"
    );
    assert!(!failure.contains(EL_KEY));
    assistant.shutdown();
}
