// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises `--voice-wav` with `--ai` against the in-process fake Scribe server: the file goes
//! to the assistant's voice input under the AI settings, its command raises a card, and the
//! seconds sent show in the AI status; with voice control off in the settings, nothing is sent.
//! No test here reaches ElevenLabs.

mod common;

use std::thread;
use std::time::{Duration, Instant};

use common::voice::{KEY, KEY_VAR, NO_ENV_FILE, fake, meeting_project, wav_file};
use common::{Running, WAIT};
use serde_json::json;

fn run(name: &str, voice: serde_json::Value, url: &str) -> Running {
    let wav = wav_file(name, 0.5);
    let project = meeting_project(&wav);
    let settings = wav.with_file_name("ai.json");
    std::fs::write(&settings, json!({ "voice": voice }).to_string()).unwrap();
    let args = [
        "--port",
        "0",
        "--project",
        project.to_str().unwrap(),
        "--ai",
        "--ai-settings",
        settings.to_str().unwrap(),
        "--env-file",
        "missing.env",
        "--voice-wav",
        wav.to_str().unwrap(),
        "--voice-url",
        url,
    ];
    Running::start_with_env(&args, &[(KEY_VAR, KEY), NO_ENV_FILE])
}

#[test]
fn the_assistants_voice_input_hears_the_file_and_counts_its_seconds() {
    let fake = fake();
    let running = run("voice-ai", json!({"enabled": true, "consent": true}), &fake.endpoint());
    let heard = running.line_starting("Voice heard: \"Sceneuron, wrap it up\"");
    assert!(heard.ends_with("raised Meeting signals: Wrap it up"), "{heard}");
    let deadline = Instant::now() + WAIT;
    let ai = loop {
        let (_, state) = running.request("GET", "/api/state", "");
        if state["ai"]["voice_seconds"].as_f64().unwrap_or(0.0) > 0.0 {
            break state["ai"].clone();
        }
        assert!(Instant::now() < deadline, "no voice spend in {}", state["ai"]);
        thread::sleep(Duration::from_millis(20));
    };
    let seconds = ai["voice_seconds"].as_f64().unwrap();
    assert!((seconds - 0.5).abs() < 0.01, "{seconds}");
    let cost = ai["voice_cost_usd"].as_f64().unwrap();
    assert!((cost - seconds * (0.39 + 0.05) / 3_600.0).abs() < 1e-9, "{cost}");
}

#[test]
fn with_voice_control_off_in_the_settings_nothing_is_sent() {
    let fake = fake();
    let running = run("voice-ai-off", json!({}), &fake.endpoint());
    running.line_starting("Voice: voice control is off");
    thread::sleep(Duration::from_millis(1_000));
    assert!(fake.sessions().is_empty(), "no session without voice control on");
}
