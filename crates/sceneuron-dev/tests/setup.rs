// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises "Set up OBS for me" against the sceneuron-dev binary's simulated OBS: it adds the
//! overlay, analyzes the simulated webcam, and with demo presence gives the simulated Mic/Aux the
//! voice filter; a second click finds it all in place, and without demo presence there is no
//! microphone to give it to. Without `--stream-deck-links` the Stream Deck plugin is not
//! installed, so no link goes out; with it, the link lands in that file, never on the terminal,
//! and once its code is traded the next click finds the Stream Deck paired.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use common::Running;
use serde_json::{Value, json};

fn step(outcome: &str, source: &str) -> Value {
    json!({ "outcome": outcome, "source": source, "reason": "" })
}

fn not_installed(report: &Value) -> bool {
    let reason = report["stream_deck"]["reason"].as_str().unwrap_or_default();
    report["stream_deck"]["outcome"] == "skipped" && reason.contains("not installed")
}

fn set_up(running: &Running) -> Value {
    let (status, report) = running.request("POST", "/api/setup", "");
    assert_eq!(status, 200, "{report}");
    report
}

#[test]
fn with_demo_presence_everything_is_added_then_found_in_place() {
    let mut running = Running::start(&["--port", "0", "--demo-presence"]);
    let added = json!({
        "overlay": step("added", "Sceneuron overlay"),
        "camera": step("added", "Webcam (simulated)"),
        "microphone": step("added", "Mic/Aux (simulated)"),
    });
    let mut report = set_up(&running);
    assert!(not_installed(&report), "{report}");
    report.as_object_mut().unwrap().remove("stream_deck");
    assert_eq!(report, added);
    running.line_starting("Added Sceneuron overlay to the current scene");
    running.line_starting("Analyzing Webcam (simulated)");
    running.line_starting("Voice control and subtitles listen to Mic/Aux (simulated)");
    let (_, sources) = running.request("GET", "/api/sources", "");
    assert_eq!(
        (&sources[0]["analyzed"], &sources[1]["analyzed"]),
        (&json!(true), &json!(false))
    );
    let (_, audio) = running.request("GET", "/api/audio-sources", "");
    assert_eq!(audio["sources"][0]["in_use"], true);

    let present = json!({
        "overlay": step("present", "Sceneuron overlay"),
        "camera": step("present", "Webcam (simulated)"),
        "microphone": step("present", "Mic/Aux (simulated)"),
    });
    let mut again = set_up(&running);
    assert!(not_installed(&again), "{again}");
    again.as_object_mut().unwrap().remove("stream_deck");
    assert_eq!(again, present);
    let (_, reply) = running.request("POST", "/api/overlay", "");
    assert_eq!(
        reply,
        json!({"name": "Sceneuron overlay 2"}),
        "the setup added only one overlay"
    );
    assert!(running.quit_with_q().success());
}

#[test]
fn without_demo_presence_the_microphone_is_skipped() {
    let mut running = Running::start(&["--port", "0"]);
    let report = set_up(&running);
    assert_eq!(report["overlay"], step("added", "Sceneuron overlay"));
    assert_eq!(report["camera"], step("added", "Webcam (simulated)"));
    let skipped = json!({ "outcome": "skipped", "source": "", "reason": "Sceneuron is not running inside OBS" });
    assert_eq!(report["microphone"], skipped);
    assert_eq!(set_up(&running)["microphone"], skipped);
    assert!(running.quit_with_q().success());
}

/// A fresh path for pairing links under the cargo target dir.
fn links_file(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir.join("stream-deck-links.txt")
}

#[test]
fn with_the_simulated_stream_deck_plugin_the_link_goes_to_a_file_and_its_code_pairs() {
    let links = links_file("stream-deck-links");
    let mut running = Running::start(&["--port", "0", "--stream-deck-links", links.to_str().unwrap()]);
    running.line_starting("Stream Deck: the plugin counts as installed");
    assert_eq!(set_up(&running)["stream_deck"], step("added", "Stream Deck"));
    let said = running.line_starting("Sent a Stream Deck pairing link to ");
    let text = fs::read_to_string(&links).unwrap();
    let prefix = format!(
        "streamdeck://plugins/message/ai.chaski.sceneuron/pair?port={}&code=",
        running.port
    );
    let code = text
        .trim_end()
        .strip_prefix(&prefix)
        .and_then(|rest| rest.strip_suffix("&streamdeck=hidden"))
        .unwrap_or_else(|| panic!("not one pairing link for this port: {text}"));
    assert!(
        code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit()),
        "{text}"
    );
    assert!(!said.contains(code), "the terminal shows the code: {said}");
    assert_eq!(fs::metadata(&links).unwrap().permissions().mode() & 0o777, 0o600);

    let (status, paired) = running.request_as("", "POST", "/api/control/pair", &json!({ "code": code }).to_string());
    assert_eq!(status, 200, "{paired}");
    assert_eq!(set_up(&running)["stream_deck"], step("present", "Stream Deck"));
    assert_eq!(fs::read_to_string(&links).unwrap(), text, "no second link");
    assert!(running.quit_with_q().success());
}
