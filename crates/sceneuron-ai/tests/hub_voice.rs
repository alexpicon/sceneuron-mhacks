// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises voice control in a hub against the in-process fake Scribe server: the status follows
//! the settings and the voice input, push to talk held in the engine holds the microphone, the
//! words heard raise a card as a voice raise, the seconds sent and their keyterm surcharge join
//! the AI status, a spent hourly budget stops listening until it allows more, and keyterms come
//! only from decks whose voice is on.

mod common;

use std::sync::Arc;

use common::director::connector;
use common::sink::wait_until;
use common::speech::EL_KEY;
use common::temp_dir;
use sceneuron_ai::{AiService, Assistant, Credentials, ElevenLabs, HubSink, MockProvider, follow_hub};
use sceneuron_core::{RaisedBy, default_project, pack_for_canvas};
use sceneuron_server::{Assistant as _, Hub, HubConfig};
use sceneuron_voice::AudioChunk;
use sceneuron_voice::fake::{FakeScribe, FakeScript};
use serde_json::json;

/// A hub with the meeting pack, whose voice is on, and the poker pack, whose voice is off, and
/// an assistant whose speech to text is `fake`.
fn hosted(fake: &FakeScribe) -> (Arc<Hub>, Arc<Assistant>, AiService) {
    let base = default_project();
    let mut project = base
        .add_pack(&pack_for_canvas("meeting", &base.canvas).unwrap())
        .unwrap()
        .add_pack(&pack_for_canvas("poker", &base.canvas).unwrap())
        .unwrap();
    project.decks[0].voice = true;
    let hub = Hub::start(project, HubConfig::default()).unwrap();
    let sink = Arc::new(HubSink::new(Arc::clone(&hub)));
    let path = temp_dir("hub-voice").join("ai.json");
    let mock = Arc::new(MockProvider::new(|_| Ok(json!({}))));
    let elevenlabs = ElevenLabs {
        realtime_url: fake.endpoint(),
        ..ElevenLabs::at("http://127.0.0.1:9")
    };
    let (assistant, warning) =
        Assistant::with_elevenlabs(path, connector(&mock), sink, Credentials::default(), &elevenlabs);
    assert_eq!(warning, None);
    let assistant = Arc::new(assistant);
    follow_hub(&hub, &assistant);
    let service = AiService::new(Arc::clone(&assistant));
    (hub, assistant, service)
}

fn fake(transcripts: &[&str]) -> FakeScribe {
    FakeScribe::start(FakeScript {
        key: EL_KEY.to_owned(),
        transcripts: transcripts.iter().map(|text| (*text).to_owned()).collect(),
        ..FakeScript::default()
    })
    .unwrap()
}

#[test]
fn the_voice_status_follows_the_settings_and_the_input() {
    let fake = fake(&[]);
    let (hub, assistant, service) = hosted(&fake);
    let message = || hub.read(|engine| engine.voice_view().message);
    assert!(wait_until(|| message() == "voice control is off"));
    service
        .update_settings(json!({"voice": {"consent": true, "enabled": true}}))
        .unwrap();
    assert!(wait_until(|| message() == "add an ElevenLabs key to use voice control"));
    service.update_settings(json!({"elevenlabs_api_key": EL_KEY})).unwrap();
    assert!(wait_until(
        || message() == "voice control is ready; hold push to talk to speak"
    ));
    let view = hub.read(|engine| engine.voice_view());
    assert!(view.enabled && !view.listening);
    hub.update(|engine, now| engine.set_talking(true, now));
    assert!(wait_until(|| hub.read(|engine| engine.voice_view().listening)));
    hub.update(|engine, now| engine.set_talking(false, now));
    assert!(wait_until(|| !hub.read(|engine| engine.voice_view().listening)));
    let settings = service.settings();
    assert_eq!(settings["has_elevenlabs_key"], true);
    assert!(!settings.to_string().contains(EL_KEY));
    assistant.shutdown();
    hub.shutdown();
}

#[test]
fn words_heard_while_push_to_talk_is_held_raise_a_card_and_count_toward_spend() {
    let fake = fake(&["Sceneuron, wrap it up"]);
    let (hub, assistant, service) = hosted(&fake);
    let update = json!({"elevenlabs_api_key": EL_KEY, "voice": {"consent": true, "enabled": true}});
    service.update_settings(update).unwrap();
    hub.update(|engine, now| engine.set_talking(true, now));
    let input = assistant.voice().input();
    assert!(
        wait_until(|| input.wants_audio()),
        "the engine's push to talk holds the microphone"
    );
    let second = AudioChunk {
        sample_rate: 16_000,
        samples: vec![0.2; 16_000],
    };
    assert!(input.feed(&second));
    hub.update(|engine, now| engine.set_talking(false, now));
    let raised = || hub.read(|engine| engine.snapshot(0).decks.into_iter().find(|deck| deck.id == "meeting"));
    assert!(wait_until(
        || raised().is_some_and(|deck| deck.raised.as_deref() == Some("wrap-up"))
    ));
    assert_eq!(raised().unwrap().raised_by, Some(RaisedBy::Voice));
    let heard = hub.read(|engine| engine.voice_view().heard).unwrap();
    assert_eq!(heard.result, "raised Meeting signals: Wrap it up");
    let spent = || hub.read(|engine| engine.snapshot(0).ai);
    assert!(wait_until(|| spent().voice_seconds > 0.0), "{:?}", spent());
    let status = spent();
    assert!((status.voice_seconds - 1.0).abs() < 1e-9, "{}", status.voice_seconds);
    let expected = (0.39 + 0.05) / 3_600.0;
    assert!(
        (status.voice_cost_usd - expected).abs() < 1e-12 && (status.hour_cost_usd - expected).abs() < 1e-12,
        "listening and its keyterm surcharge count: {status:?}"
    );
    let session = &fake.sessions()[0];
    let keyterms = session.query_values("keyterms");
    assert!(
        keyterms.contains(&"Sceneuron") && keyterms.contains(&"Wrap it up"),
        "{keyterms:?}"
    );
    assert!(
        keyterms.contains(&"I can't hear you"),
        "trigger phrases too: {keyterms:?}"
    );
    assert!(
        !keyterms.contains(&"13"),
        "a deck whose voice is off adds none: {keyterms:?}"
    );
    assistant.shutdown();
    hub.shutdown();
}

#[test]
fn a_spent_hourly_budget_stops_listening_until_the_budget_allows() {
    let fake = fake(&[]);
    let (hub, assistant, service) = hosted(&fake);
    let budget = |max_cost_per_hour| {
        let mut project = hub.read(|engine| engine.project().clone());
        project.ai.max_cost_per_hour = max_cost_per_hour;
        hub.try_update(|engine, now| engine.replace_project(project, now))
            .unwrap();
    };
    budget(0.0);
    let update =
        json!({"elevenlabs_api_key": EL_KEY, "voice": {"consent": true, "enabled": true, "mode": "always_on"}});
    service.update_settings(update).unwrap();
    let view = || hub.read(|engine| engine.voice_view());
    let input = assistant.voice().input();
    assert!(wait_until(|| view().listening && input.wants_audio()), "{:?}", view());
    // Always-on reports its spend every ten seconds of audio.
    let second = AudioChunk {
        sample_rate: 16_000,
        samples: vec![0.2; 16_000],
    };
    assert!((0..10).all(|_| input.feed(&second)));
    let spent = "the hourly budget is spent; voice control resumes when the budget allows";
    assert!(wait_until(|| view().message == spent), "{:?}", view());
    assert!(!view().listening && view().enabled, "{:?}", view());
    assert!(!input.wants_audio(), "no audio is taken while the budget is spent");
    // The director tells budget watchers before it publishes the new cost, so wait for it.
    assert!(wait_until(
        || hub.read(|engine| engine.snapshot(0).ai.hour_cost_usd) > 0.0
    ));
    budget(2.0);
    assert!(wait_until(|| view().listening && input.wants_audio()), "{:?}", view());
    assert_eq!(view().message, "listening");
    assert_eq!(fake.sessions().len(), 2, "the stream closed and opened again");
    assistant.shutdown();
    hub.shutdown();
}

#[test]
fn keyterms_follow_the_project_and_the_mode() {
    let fake = fake(&[]);
    let (hub, assistant, service) = hosted(&fake);
    let keyterms = || assistant.voice().keyterms();
    assert!(keyterms().contains(&"Wrap it up".to_owned()));
    let mut project = hub.read(|engine| engine.project().clone());
    project.decks[0].voice = false;
    project.decks[1].voice = true;
    hub.try_update(|engine, now| engine.replace_project(project.clone(), now))
        .unwrap();
    assert!(wait_until(|| keyterms().contains(&"13".to_owned())));
    assert!(!keyterms().contains(&"Wrap it up".to_owned()));
    project.decks[1].voice = false;
    hub.try_update(|engine, now| engine.replace_project(project, now))
        .unwrap();
    assert!(
        wait_until(|| keyterms().is_empty()),
        "push to talk needs no wake word without cards: {:?}",
        keyterms()
    );
    service
        .update_settings(json!({"voice": {"mode": "always_on"}}))
        .unwrap();
    assert_eq!(keyterms(), ["Sceneuron"], "always-on listens for the wake word");
    assistant.shutdown();
    hub.shutdown();
}
