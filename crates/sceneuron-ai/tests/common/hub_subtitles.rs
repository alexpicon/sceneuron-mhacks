// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Subtitles in a hub: an assistant whose speech to text is the fake Scribe server and whose
//! provider translates each line into "ES " and its words, a little slowly, as a real one would;
//! projects with packs; and seconds of audio.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use sceneuron_ai::{AiService, Assistant, Credentials, ElevenLabs, HubSink, MockProvider, ModelRequest, follow_hub};
use sceneuron_core::{Project, default_project, pack_for_canvas};
use sceneuron_server::{Hub, HubConfig};
use sceneuron_voice::AudioChunk;
use sceneuron_voice::fake::{FakeScribe, FakeScript};
use serde_json::{Value, json};

use super::director::connector;
use super::speech::EL_KEY;
use super::temp_dir;

/// How long the stand-in provider takes to translate a line.
pub const TRANSLATION_TIME: Duration = Duration::from_millis(150);

/// A hub for `project` and an assistant whose speech to text is `fake`.
pub fn hosted(fake: &FakeScribe, project: Project) -> (Arc<Hub>, Arc<Assistant>, AiService, Arc<MockProvider>) {
    let hub = Hub::start(project, HubConfig::default()).unwrap();
    let sink = Arc::new(HubSink::new(Arc::clone(&hub)));
    let mock = Arc::new(MockProvider::new(|request: &ModelRequest| {
        thread::sleep(TRANSLATION_TIME);
        let input: Value = serde_json::from_str(&request.text).unwrap();
        Ok(json!({"translations": {"es": format!("ES {}", input["line"].as_str().unwrap())}}))
    }));
    let elevenlabs = ElevenLabs {
        realtime_url: fake.endpoint(),
        ..ElevenLabs::at("http://127.0.0.1:9")
    };
    let path = temp_dir("hub-subtitles").join("ai.json");
    let (assistant, warning) =
        Assistant::with_elevenlabs(path, connector(&mock), sink, Credentials::default(), &elevenlabs);
    assert_eq!(warning, None);
    let assistant = Arc::new(assistant);
    follow_hub(&hub, &assistant);
    let service = AiService::new(Arc::clone(&assistant));
    (hub, assistant, service, mock)
}

/// The default project with the packs `ids`.
pub fn with_packs(ids: &[&str]) -> Project {
    let base = default_project();
    let mut project = base.clone();
    for id in ids {
        project = project.add_pack(&pack_for_canvas(id, &base.canvas).unwrap()).unwrap();
    }
    project
}

/// A fake Scribe server that hears `transcripts` in turn, one per `vad_bytes` of streamed audio.
pub fn fake(transcripts: &[&str], vad_bytes: usize) -> FakeScribe {
    FakeScribe::start(FakeScript {
        key: EL_KEY.to_owned(),
        transcripts: transcripts.iter().map(|text| (*text).to_owned()).collect(),
        vad_bytes,
        ..FakeScript::default()
    })
    .unwrap()
}

/// `count` seconds of 16 kHz audio.
pub fn seconds(count: f64) -> AudioChunk {
    AudioChunk {
        sample_rate: 16_000,
        samples: vec![0.2; (16_000.0 * count) as usize],
    }
}

/// What an element shows now.
pub fn text(hub: &Hub, element: &str) -> String {
    hub.read(|engine| engine.effective_props(element).unwrap().text)
}
