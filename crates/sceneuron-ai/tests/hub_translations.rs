// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises subtitle translations in a hub against the fake Scribe server and a provider that
//! takes a moment per line: each final line is handed to the translator once, in the order it was
//! said, even while later lines arrive, and every translation reaches the engine; and the spoken
//! language set for subtitles reaches Scribe and the subtitle status, so a target in that language
//! shows the original without a request.

mod common;

use common::TEST_KEY;
use common::hub_subtitles::{fake, hosted, seconds};
use common::sink::wait_until;
use common::speech::EL_KEY;
use sceneuron_core::default_project;
use sceneuron_server::Assistant as _;
use serde_json::{Value, json};

#[test]
fn each_final_line_is_translated_once_in_order_while_later_lines_arrive() {
    let fake = fake(&["One", "Two", "Three"], 9_600);
    let (hub, assistant, service, mock) = hosted(&fake, default_project());
    let update = json!({"elevenlabs_api_key": EL_KEY, "api_key": TEST_KEY,
        "subtitles": {"consent": true, "enabled": true, "targets": ["es"], "lines": 3, "clear_after_ms": 60000}});
    service.update_settings(update).unwrap();
    let input = assistant.voice().input();
    let listening = || input.wants_audio() && hub.read(|engine| engine.subtitle_status().state == "listening");
    assert!(wait_until(listening));
    // Audio goes out in tenths of a second, so a second makes three segments of three tenths each,
    // sooner than one translation takes.
    assert!(input.feed(&seconds(1.0)));
    let translated = || hub.read(|engine| engine.subtitles_view().translations["es"].clone());
    assert!(wait_until(|| translated().len() == 3), "{:?}", translated());
    assert_eq!(translated(), ["ES One", "ES Two", "ES Three"]);
    let lines: Vec<String> = mock
        .requests()
        .iter()
        .map(|request| {
            let input: Value = serde_json::from_str(&request.text).unwrap();
            input["line"].as_str().unwrap().to_owned()
        })
        .collect();
    assert_eq!(lines, ["One", "Two", "Three"], "each line is sent once, in order");
    // A job is handed out when its line is final, so the lines before it go along with whatever
    // translations were known by then.
    let third: Value = serde_json::from_str(&mock.requests()[2].text).unwrap();
    let context: Vec<&str> = third["context"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|earlier| earlier["line"].as_str())
        .collect();
    assert_eq!(context, ["One", "Two"], "the lines before it go along");
    assistant.shutdown();
    hub.shutdown();
}

#[test]
fn the_spoken_language_set_for_subtitles_reaches_scribe_and_a_target_in_it_needs_no_request() {
    let fake = fake(&["Buenos días a todos"], 0);
    let (hub, assistant, service, mock) = hosted(&fake, default_project());
    let update = json!({"elevenlabs_api_key": EL_KEY, "api_key": TEST_KEY, "voice": {"language": "en"},
        "subtitles": {"consent": true, "enabled": true, "language": "es", "targets": ["es"]}});
    service.update_settings(update).unwrap();
    let input = assistant.voice().input();
    let status = || hub.read(|engine| engine.subtitle_status().clone());
    assert!(wait_until(|| input.wants_audio() && status().state == "listening"));
    assert!(!hub.read(|engine| engine.voice_view().enabled), "voice control is off");
    assert_eq!(status().language, "es", "not voice control's language");
    assert_eq!(fake.sessions()[0].query_values("language_code"), ["es"]);
    assert!(input.feed(&seconds(1.0)));
    let spanish = || hub.read(|engine| engine.subtitles_view().translations["es"].clone());
    assert!(wait_until(|| spanish() == ["Buenos días a todos"]), "{:?}", spanish());
    assert_eq!(mock.calls(), 0, "a target in the spoken language shows the original");
    assistant.shutdown();
    hub.shutdown();
}
