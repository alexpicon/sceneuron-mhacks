// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises spoken cards in a hub: a card raised on a deck that speaks gets its audio from a
//! scripted ElevenLabs server and is served by the server's speech endpoint, and a failure lands
//! in the event log without the key. Voice control in a hub is in `hub_voice.rs`.

mod common;

use std::sync::Arc;

use common::director::connector;
use common::server::{MockServer, Reply};
use common::sink::wait_until;
use common::speech::{EL_KEY, default_voices, mp3};
use common::temp_dir;
use sceneuron_ai::{AiService, Assistant, Credentials, HubSink, MockProvider, follow_hub};
use sceneuron_core::{CardAction, default_project, pack_for_canvas};
use sceneuron_server::{Assistant as _, Hub, HubConfig};
use serde_json::json;

fn hosted(server: &MockServer) -> (Arc<Hub>, Arc<Assistant>, AiService) {
    let base = default_project();
    let mut project = base
        .add_pack(&pack_for_canvas("meeting", &base.canvas).unwrap())
        .unwrap();
    project.decks[0].speak = true;
    let hub = Hub::start(project, HubConfig::default()).unwrap();
    let sink = Arc::new(HubSink::new(Arc::clone(&hub)));
    let path = temp_dir("hub-speech").join("ai.json");
    let mock = Arc::new(MockProvider::new(|_| Ok(json!({}))));
    let (assistant, warning) =
        Assistant::with_elevenlabs_url(path, connector(&mock), sink, Credentials::default(), &server.url());
    assert_eq!(warning, None);
    let assistant = Arc::new(assistant);
    follow_hub(&hub, &assistant);
    let service = AiService::new(Arc::clone(&assistant));
    (hub, assistant, service)
}

fn raise(hub: &Hub, card: &str) {
    hub.try_update(|engine, now| engine.card("meeting", Some(card), CardAction::Raise, now))
        .unwrap();
}

#[test]
fn a_raised_card_is_spoken_and_served() {
    let echo = json!({"detail": {"message": format!("nope {EL_KEY}")}});
    let server = MockServer::start(vec![
        default_voices(),
        Reply::bytes(200, "audio/mpeg", mp3()),
        Reply::json(401, echo),
    ]);
    let (hub, assistant, service) = hosted(&server);
    service
        .update_settings(json!({"elevenlabs_api_key": EL_KEY, "voice": {"speak": true}}))
        .unwrap();
    raise(&hub, "agree");
    let audio = || hub.read(|engine| engine.speech().and_then(|cue| cue.audio.clone()));
    assert!(
        wait_until(|| audio().is_some()),
        "{:?}",
        hub.read(|engine| engine.speech().cloned())
    );
    let id = audio().unwrap();
    let speech = service.speech(&id).unwrap();
    assert_eq!((speech.audio, speech.content_type), (mp3(), "audio/mpeg"));
    assert_eq!(service.speech(&"0".repeat(64)), None);
    assert_eq!(server.requests()[1].json()["text"], "Agree");
    raise(&hub, "disagree");
    let noted = || {
        hub.snapshot()
            .events
            .into_iter()
            .find(|event| event.source == "voice" && event.detail.starts_with("a card was not spoken"))
    };
    assert!(wait_until(|| noted().is_some()));
    assert!(!noted().unwrap().detail.contains(EL_KEY));
    assert_eq!(hub.read(|engine| engine.speech().unwrap().audio.clone()), None);
    assistant.shutdown();
    hub.shutdown();
}
