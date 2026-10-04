// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the spoken language as speech to text detects it, end to end through a hub and the
//! fake Scribe server: a stream asks for detection only while the language is left to it, the
//! language reported reaches the subtitle status and names the Subtitles pack's spoken bar, and a
//! language set, or subtitles turned off, leaves the detected one behind.

mod common;

use common::hub_subtitles::{hosted, seconds, with_packs};
use common::sink::wait_until;
use common::speech::EL_KEY;
use sceneuron_core::SubtitleBar;
use sceneuron_server::{Assistant as _, Hub};
use sceneuron_voice::fake::{FakeScribe, FakeScript};
use serde_json::json;

const LINE: &str = "Good morning everyone";

/// A fake Scribe server that hears `LINE` once a second and reports it in English.
fn english() -> FakeScribe {
    FakeScribe::start(FakeScript {
        key: EL_KEY.to_owned(),
        transcripts: vec![LINE.to_owned(); 4],
        vad_bytes: 16_000,
        language: "eng".to_owned(),
        ..FakeScript::default()
    })
    .unwrap()
}

fn detected(hub: &Hub) -> String {
    hub.read(|engine| engine.subtitle_status().detected.clone())
}

fn spoken_bar(hub: &Hub) -> SubtitleBar {
    let snapshot = hub.read(|engine| engine.snapshot(0));
    let bar = snapshot
        .elements
        .into_iter()
        .find(|element| element.id == "subtitles-spoken");
    bar.and_then(|element| element.subtitle_bar).expect("the spoken bar")
}

#[test]
fn the_detected_language_names_the_spoken_bar_until_a_language_is_set() {
    let fake = english();
    let (hub, assistant, service, _mock) = hosted(&fake, with_packs(&["subtitles"]));
    let on = json!({"elevenlabs_api_key": EL_KEY, "subtitles": {"consent": true, "enabled": true}});
    service.update_settings(on).unwrap();
    let input = assistant.voice().input();
    assert!(wait_until(|| input.wants_audio() && !fake.sessions().is_empty()));
    let session = &fake.sessions()[0];
    assert_eq!(session.query_values("include_language_detection"), ["true"]);
    assert!(session.query_values("language_code").is_empty());
    assert_eq!(
        spoken_bar(&hub).label,
        "",
        "nothing is named before speech to text says"
    );
    assert!(input.feed(&seconds(0.5)) && input.feed(&seconds(0.5)));
    assert!(wait_until(|| detected(&hub) == "eng"), "{:?}", detected(&hub));
    assert_eq!(spoken_bar(&hub).label, "English (detected)");

    service
        .update_settings(json!({"subtitles": {"language": "es"}}))
        .unwrap();
    assert!(
        wait_until(|| fake.sessions().len() == 2),
        "the language is fixed when a session opens"
    );
    let session = &fake.sessions()[1];
    assert_eq!(session.query_values("language_code"), ["es"]);
    assert!(session.query_values("include_language_detection").is_empty());
    assert_eq!(detected(&hub), "", "a language set leaves the detected one behind");
    assert_eq!(spoken_bar(&hub).label, "Spanish");
    assistant.shutdown();
    hub.shutdown();
}

#[test]
fn turning_subtitles_off_forgets_the_detected_language() {
    let fake = english();
    let (hub, assistant, service, _mock) = hosted(&fake, with_packs(&["subtitles"]));
    let on = json!({"elevenlabs_api_key": EL_KEY, "subtitles": {"consent": true, "enabled": true}});
    service.update_settings(on).unwrap();
    let input = assistant.voice().input();
    assert!(wait_until(|| input.wants_audio()));
    assert!(input.feed(&seconds(0.5)) && input.feed(&seconds(0.5)));
    assert!(wait_until(|| detected(&hub) == "eng"));
    service
        .update_settings(json!({"subtitles": {"enabled": false}}))
        .unwrap();
    assert_eq!(detected(&hub), "");
    service
        .update_settings(json!({"subtitles": {"enabled": true}}))
        .unwrap();
    assert_eq!(detected(&hub), "", "until speech to text says again");
    assistant.shutdown();
    hub.shutdown();
}
