// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises `--voice-wav` end to end against the in-process fake Scribe server: the file is
//! heard as push to talk and the audio sent and its cost are printed, the engine reads its
//! transcript and raises a card of a deck whose voice is on, keyterms come from such decks only,
//! the key comes from the environment or the env file, and a missing key, an unreadable file,
//! or the simulated AI is an error. No test here reaches ElevenLabs.

mod common;

use std::process::{Command, Stdio};
use std::time::Duration;

use common::voice::{KEY, KEY_VAR, NO_ENV_FILE, fake, meeting_project, wav_file};
use common::{BINARY, Running};

fn heard_in_event_log(running: &Running, text: &str) -> bool {
    let (_, state) = running.request("GET", "/api/state", "");
    let events = state["events"].as_array().cloned().unwrap_or_default();
    events
        .iter()
        .any(|event| event["source"] == "voice" && event["detail"] == format!("heard: {text:?}"))
}

#[test]
fn a_wav_file_is_heard_as_push_to_talk() {
    let fake = fake();
    let (wav, url) = (wav_file("voice-heard", 0.6), fake.endpoint());
    let args = [
        "--port",
        "0",
        "--env-file",
        "missing.env",
        "--voice-wav",
        wav.to_str().unwrap(),
        "--voice-url",
        &url,
    ];
    let running = Running::start_with_env(&args, &[(KEY_VAR, KEY), NO_ENV_FILE]);
    let speaking = running.line_starting("Voice: speaking");
    assert!(speaking.contains("(0.6 s)") && speaking.contains(&url), "{speaking}");
    // 0.6 s at $0.39 an hour, without keyterms; printed before the transcript can arrive.
    let spent = running.line_starting("Voice: sent ");
    assert_eq!(spent, "Voice: sent 0.6 s of audio so far, about $0.0001");
    let heard = running.line_starting("Voice heard: \"Sceneuron, wrap it up\"");
    let unmatched = "no command matched, and Sceneuron's AI is not running to take others";
    assert!(heard.ends_with(unmatched), "{heard}");
    assert!(heard_in_event_log(&running, "Sceneuron, wrap it up"));
    let sessions = fake
        .wait(Duration::from_secs(5), |sessions| {
            sessions.len() == 1 && sessions[0].commits == 1
        })
        .unwrap();
    let session = &sessions[0];
    assert_eq!(session.key.as_deref(), Some(KEY));
    assert_eq!(session.query_values("commit_strategy"), ["manual"]);
    assert!(
        session.query_values("keyterms").is_empty(),
        "push to talk with no deck that voice can raise needs no keyterms"
    );
    let samples = session.audio.len() / 2;
    assert!((9_500..=9_600).contains(&samples), "{samples} samples for 0.6 s");
}

#[test]
fn the_key_can_come_from_the_env_file() {
    let fake = fake();
    let wav = wav_file("voice-env-file", 0.3);
    let env_file = wav.with_file_name("keys.env");
    std::fs::write(&env_file, format!("{KEY_VAR}={KEY}\n")).unwrap();
    let url = fake.endpoint();
    let args = [
        "--port",
        "0",
        "--env-file",
        env_file.to_str().unwrap(),
        "--voice-wav",
        wav.to_str().unwrap(),
        "--voice-url",
        &url,
    ];
    let running = Running::start_with_env(&args, &[(KEY_VAR, ""), NO_ENV_FILE]);
    running.line_starting("Voice heard: \"Sceneuron, wrap it up\"");
}

#[test]
fn a_command_in_the_file_raises_a_card_of_a_deck_whose_voice_is_on() {
    let fake = fake();
    let wav = wav_file("voice-raise", 0.4);
    let project = meeting_project(&wav);
    let url = fake.endpoint();
    let args = [
        "--port",
        "0",
        "--project",
        project.to_str().unwrap(),
        "--env-file",
        "missing.env",
        "--voice-wav",
        wav.to_str().unwrap(),
        "--voice-url",
        &url,
    ];
    let running = Running::start_with_env(&args, &[(KEY_VAR, KEY), NO_ENV_FILE]);
    let heard = running.line_starting("Voice heard: \"Sceneuron, wrap it up\"");
    assert!(heard.ends_with("raised Meeting signals: Wrap it up"), "{heard}");
    let (_, state) = running.request("GET", "/api/state", "");
    let deck = state["decks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|deck| deck["id"] == "meeting")
        .unwrap()
        .clone();
    assert_eq!(
        (&deck["raised"], &deck["raised_by"]),
        (&"wrap-up".into(), &"voice".into())
    );
    let keyterms = fake.sessions()[0].query_values("keyterms").join("|");
    assert!(keyterms.starts_with("Sceneuron|Wrap it up|"), "{keyterms}");
    assert!(!keyterms.contains("13"), "the poker deck's voice is off: {keyterms}");
}

#[test]
fn a_missing_key_or_an_unreadable_file_is_an_error() {
    let wav = wav_file("voice-errors", 0.1);
    let not_wav = wav.with_file_name("notes.wav");
    std::fs::write(&not_wav, "not audio").unwrap();
    let cases = [
        (
            wav.to_str().unwrap(),
            "needs an ElevenLabs API key in SCENEURON_ELEVENLABS_API_KEY",
        ),
        (not_wav.to_str().unwrap(), "not a WAV file"),
        ("missing.wav", "cannot read missing.wav"),
    ];
    let simulated = Command::new(BINARY)
        .args(["--port", "0", "--demo-ai", "--voice-wav", "missing.wav"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(simulated.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&simulated.stderr).contains("--voice-wav cannot be used with --demo-ai"));
    for (path, expected) in cases {
        let result = Command::new(BINARY)
            .args(["--port", "0", "--env-file", "missing.env", "--voice-wav", path])
            .args(["--voice-url", "ws://127.0.0.1:9/v1/speech-to-text/realtime"])
            .envs([(KEY_VAR, ""), NO_ENV_FILE])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1), "{path}");
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(stderr.contains(expected), "{path}: {stderr}");
    }
}
