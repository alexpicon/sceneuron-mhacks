// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises which voice speaks cards, against a scripted ElevenLabs server: the account's
//! voices are read once a key is set, the default voice is replaced by one the account has, a
//! chosen voice the account lacks is reported instead of spoken, and a voice lost later is
//! checked again.

mod common;

use std::sync::Arc;

use common::director::connector;
use common::server::{MockServer, Reply};
use common::sink::{RecordingSink, wait_until};
use common::speech::{EL_KEY, audio_reply, voices_reply};
use common::temp_dir;
use sceneuron_ai::{AiSettingsUpdate, Assistant, Credentials, MockProvider};
use serde_json::json;

const SPEAK: &str = "/v1/text-to-speech/";

fn speaking(server: &MockServer, voice: serde_json::Value) -> (Assistant, Arc<RecordingSink>) {
    let sink = Arc::new(RecordingSink::default());
    let path = temp_dir("speaker-voices").join("ai.json");
    let mock = Arc::new(MockProvider::new(|_| Ok(json!({}))));
    let (assistant, _) = Assistant::with_elevenlabs_url(
        path,
        connector(&mock),
        Arc::clone(&sink) as _,
        Credentials::default(),
        &server.url(),
    );
    let mut voice = voice;
    voice["speak"] = json!(true);
    let update: AiSettingsUpdate =
        serde_json::from_value(json!({"elevenlabs_api_key": EL_KEY, "voice": voice})).unwrap();
    assistant.update_settings(update).unwrap();
    (assistant, sink)
}

fn paths(server: &MockServer) -> Vec<String> {
    server.requests().into_iter().map(|request| request.path).collect()
}

#[test]
fn the_default_voice_is_replaced_by_one_the_account_has() {
    let server = MockServer::start(vec![
        voices_reply(&[("own1", "Mine", "cloned"), ("prem1", "Rachel", "premade")], false),
        audio_reply(),
    ]);
    let (assistant, sink) = speaking(&server, json!({}));
    assistant.speak(1, "Agree");
    assert!(wait_until(|| sink.spoken().len() == 1), "{:?}", sink.received());
    assert_eq!(
        paths(&server)[1],
        format!("{SPEAK}prem1?output_format=mp3_44100_128"),
        "the account's premade voice speaks"
    );
    let notes = sink.voice_notes();
    assert_eq!(notes.len(), 1);
    assert!(notes[0].contains("Rachel (prem1)"), "{notes:?}");
    assert!(sink.speech_failures().is_empty());
    assistant.shutdown();
}

#[test]
fn a_chosen_voice_the_account_lacks_is_reported_and_never_requested() {
    let server = MockServer::start(vec![voices_reply(&[("prem1", "Rachel", "premade")], false)]);
    let (assistant, sink) = speaking(&server, json!({"voice_id": "gone42"}));
    assistant.speak(1, "Agree");
    assistant.speak(2, "Disagree");
    assert!(wait_until(|| !sink.speech_failures().is_empty()));
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert_eq!(
        sink.speech_failures(),
        [
            "cards are not spoken: the voice id gone42 is not in your ElevenLabs account; paste one from My Voices \
          into the AI tab"
        ],
        "said once until the settings change"
    );
    assert_eq!(paths(&server), ["/v2/voices?page_size=100"]);
    assert!(sink.spoken().is_empty());
    assistant.shutdown();
}

#[test]
fn a_voice_on_a_later_page_is_used_as_chosen() {
    let server = MockServer::start(vec![
        voices_reply(&[("other", "Other", "premade")], true),
        audio_reply(),
    ]);
    let (assistant, sink) = speaking(&server, json!({"voice_id": "later7"}));
    assistant.speak(1, "Agree");
    assert!(wait_until(|| sink.spoken().len() == 1));
    assert!(paths(&server)[1].starts_with(&format!("{SPEAK}later7?")));
    assistant.shutdown();
}

#[test]
fn a_voice_not_found_when_speaking_is_checked_again_and_replaced() {
    let missing = json!({"detail": {"code": "voice_not_found", "message": "not found"}});
    let server = MockServer::start(vec![
        Reply::json(503, json!({"detail": {"message": "busy"}})),
        Reply::json(404, missing),
        voices_reply(&[("prem1", "Rachel", "premade")], false),
        audio_reply(),
    ]);
    let (assistant, sink) = speaking(&server, json!({}));
    assistant.speak(1, "Agree");
    assert!(wait_until(|| sink.spoken().len() == 1), "{:?}", sink.received());
    let paths = paths(&server);
    assert_eq!(paths.len(), 4, "{paths:?}");
    assert!(paths[1].starts_with(&format!("{SPEAK}JBFqnCBsd6RMkjVDRZzb?")));
    assert!(paths[3].starts_with(&format!("{SPEAK}prem1?")));
    assert!(sink.speech_failures().is_empty());
    assistant.shutdown();
}
