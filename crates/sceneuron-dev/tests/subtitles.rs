// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises subtitles in sceneuron-dev: the simulated ones of `--demo-ai`, which need consent and
//! then show a scripted speaker with canned translations, and `--voice-wav` with `--ai` and
//! subtitles on against the in-process fake Scribe server, whose words become a subtitle line
//! with voice control off, while a translation that cannot reach the default AI provider,
//! Gemini, on a closed local address, is logged without the words. No test here reaches
//! ElevenLabs or a model.

mod common;

use std::thread;
use std::time::{Duration, Instant};

use common::voice::{KEY, KEY_VAR, NO_ENV_FILE, wav_file};
use common::{Running, WAIT, project_path};
use sceneuron_voice::fake::{FakeScribe, FakeScript};
use serde_json::{Value, json};

/// Polls `path` every 20 ms until `ready` accepts its JSON.
fn when(running: &Running, path: &str, what: &str, ready: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + WAIT;
    loop {
        let (_, value) = running.request("GET", path, "");
        if ready(&value) {
            return value;
        }
        assert!(Instant::now() < deadline, "{what} never happened: {value}");
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn the_simulated_subtitles_need_consent_and_show_a_scripted_speaker_and_translations() {
    let running = Running::start(&["--port", "0", "--demo-ai"]);
    let (_, settings) = running.request("GET", "/api/ai", "");
    assert_eq!(settings["subtitles"]["enabled"], false);
    assert_eq!(settings["subtitles"]["translation_model"], "gemini-3.5-flash-lite");
    let (status, refused) = running.request("PUT", "/api/ai", r#"{"subtitles": {"enabled": true}}"#);
    assert_eq!(status, 400, "{refused}");
    let on = json!({"subtitles": {"consent": true, "enabled": true, "targets": ["es", "fr"]}});
    let (status, saved) = running.request("PUT", "/api/ai", &on.to_string());
    assert_eq!(
        (status, saved["subtitles"]["targets"].clone()),
        (200, json!(["es", "fr"])),
        "{saved}"
    );
    let english = "Good morning everyone, and welcome to the stream.";
    let view = when(&running, "/api/subtitles", "a translated line", |view| {
        view["translations"]["fr"]
            .as_array()
            .is_some_and(|lines| !lines.is_empty())
    });
    assert_eq!(joined(&view["lines"]), english);
    assert_eq!(
        joined(&view["translations"]["es"]),
        "Buenos días a todos y bienvenidos a la transmisión."
    );
    assert!(
        view["status"]["message"]
            .as_str()
            .unwrap()
            .starts_with("Simulated subtitles")
    );
    let (_, off) = running.request("PUT", "/api/ai", r#"{"subtitles": {"enabled": false}}"#);
    assert_eq!(off["subtitles"]["enabled"], false);
    when(&running, "/api/subtitles", "subtitles turned off", |view| {
        view["status"]["state"] == "off"
    });
}

/// The lines of a JSON array of strings, joined by spaces.
fn joined(lines: &Value) -> String {
    let lines: Vec<&str> = lines.as_array().unwrap().iter().filter_map(Value::as_str).collect();
    lines.join(" ")
}

#[test]
fn a_spoken_file_becomes_a_subtitle_line_and_a_failed_translation_is_logged_without_it() {
    let fake = FakeScribe::start(FakeScript {
        key: KEY.to_owned(),
        transcripts: vec!["Good morning everyone".to_owned()],
        ..FakeScript::default()
    })
    .unwrap();
    let wav = wav_file("subtitles-wav", 0.5);
    let settings = project_path("subtitles-wav-ai").with_file_name("ai.json");
    let ai = json!({"base_url": "http://127.0.0.1:9",
        "subtitles": {"enabled": true, "consent": true, "targets": ["es"], "clear_after_ms": 60000}});
    std::fs::write(&settings, ai.to_string()).unwrap();
    let args = [
        "--port",
        "0",
        "--ai",
        "--ai-settings",
        settings.to_str().unwrap(),
        "--env-file",
        "missing.env",
        "--voice-wav",
        wav.to_str().unwrap(),
        "--voice-url",
        &fake.endpoint(),
    ];
    let env = [
        (KEY_VAR, KEY),
        ("SCENEURON_GEMINI_API_KEY", "test-gemini-key-0123456789"),
        NO_ENV_FILE,
    ];
    let running = Running::start_with_env(&args, &env);
    let view = when(&running, "/api/subtitles", "the subtitle line", |view| {
        view["lines"] == json!(["Good morning everyone"])
    });
    assert_eq!(view["status"]["enabled"], true);
    let failed = "translation failed: the AI provider is unavailable";
    let state = when(
        &running,
        "/api/state",
        "the failed translation in the event log",
        |state| {
            state["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|event| event["detail"] == failed)
        },
    );
    let details: Vec<&str> = state["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|event| event["detail"].as_str())
        .collect();
    assert!(details.iter().all(|detail| !detail.contains("morning")), "{details:?}");
    assert_eq!(state["voice"]["enabled"], false, "voice control stays off");
}
