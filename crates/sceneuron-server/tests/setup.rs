// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises "Set up OBS for me": the route answers what the host added and what was already in
//! place, and skips the Stream Deck when its plugin is not installed, only the editor token
//! reaches it, the host outside OBS refuses it, and the camera chosen for analysis is a camera
//! device, the likeliest kind first, and never a source of another kind. Pairing the Stream Deck
//! is exercised in `setup_stream_deck.rs`.

mod common;

use std::sync::atomic::Ordering;

use common::{FAKE_MIC, FAKE_SOURCE, Req, TestServer, start_server};
use sceneuron_core::default_project;
use sceneuron_server::{Hub, HubConfig, ServerConfig, SourceInfo, camera_to_analyze, generate_token};
use serde_json::{Value, json};

const SETUP: &str = "/api/setup";

fn step(outcome: &str, source: &str) -> Value {
    json!({ "outcome": outcome, "source": source, "reason": "" })
}

/// The Stream Deck step on a Mac without Sceneuron's Stream Deck plugin.
fn no_stream_deck_plugin(report: &Value) -> bool {
    let reason = report["stream_deck"]["reason"].as_str().unwrap_or_default();
    report["stream_deck"]["outcome"] == "skipped" && reason.contains("not installed") && reason.contains("double-click")
}

fn source(name: &str, kind: &str) -> SourceInfo {
    SourceInfo {
        uuid: format!("uuid-{name}"),
        name: name.to_owned(),
        kind: kind.to_owned(),
        analyzed: false,
    }
}

#[test]
fn the_editor_sets_up_what_is_missing_and_a_second_click_finds_it_present() {
    let server = TestServer::start();
    let reply = server.post_json(SETUP, "");
    assert_eq!(reply.status, 200, "{}", reply.text());
    let mut report = reply.json();
    assert!(no_stream_deck_plugin(&report), "{report}");
    report.as_object_mut().unwrap().remove("stream_deck");
    let added = json!({
        "overlay": step("added", "Sceneuron overlay"),
        "camera": step("added", "Camera"),
        "microphone": step("added", "Mic/Aux"),
    });
    assert_eq!(report, added);
    assert_eq!(*server.host.attached.lock().unwrap(), [FAKE_SOURCE]);
    assert_eq!(*server.host.voice_attached.lock().unwrap(), [FAKE_MIC]);

    let mut again = server.post_json(SETUP, "").json();
    assert!(no_stream_deck_plugin(&again), "{again}");
    again.as_object_mut().unwrap().remove("stream_deck");
    let present = json!({
        "overlay": step("present", "Sceneuron overlay"),
        "camera": step("present", "Camera"),
        "microphone": step("present", "Mic/Aux"),
    });
    assert_eq!(again, present);
    assert!(server.host.stream_deck_links.lock().unwrap().is_empty());
    assert_eq!(server.host.overlays.load(Ordering::SeqCst), 1);
    assert_eq!(server.host.attached.lock().unwrap().len(), 1);
    assert_eq!(server.host.voice_attached.lock().unwrap().len(), 1);
}

#[test]
fn only_the_editor_token_sets_up_obs_and_only_with_post() {
    let server = TestServer::start();
    server.host.stream_deck_installed.store(true, Ordering::SeqCst);
    let url = server.server.as_ref().unwrap().overlay_url();
    let overlay = url.split("token=").nth(1).unwrap().to_owned();
    let reply = server.with_token(&overlay, "POST", SETUP).send(server.port);
    assert_eq!(reply.status, 403, "overlay: {}", reply.text());
    let reply = server.control("POST", SETUP).send(server.port);
    assert_eq!(reply.status, 403, "control: {}", reply.text());
    let reply = Req::new("POST", &format!("{SETUP}?token={overlay}")).send(server.port);
    assert_eq!(reply.status, 403, "overlay query: {}", reply.text());
    assert_eq!(Req::new("POST", SETUP).send(server.port).status, 401);
    let reply = server.api("GET", SETUP).send(server.port);
    assert_eq!((reply.status, reply.header("Allow")), (405, Some("POST")));
    assert_eq!(server.host.overlays.load(Ordering::SeqCst), 0);
    assert!(server.host.attached.lock().unwrap().is_empty());
    assert!(server.host.voice_attached.lock().unwrap().is_empty());
    assert!(server.host.stream_deck_links.lock().unwrap().is_empty());
}

#[test]
fn outside_obs_setting_up_is_refused() {
    let hub = Hub::start(default_project(), HubConfig::default()).unwrap();
    let token = generate_token();
    let config = ServerConfig {
        port: 0,
        token: token.clone(),
        ..ServerConfig::default()
    };
    let server = start_server(hub, config).unwrap();
    let reply = Req::new("POST", SETUP)
        .header("X-Sceneuron-Token", &token)
        .send(server.port());
    assert_eq!(reply.status, 400);
    assert_eq!(reply.error(), "Sceneuron is not running inside OBS");
}

/// The name of the source chosen for camera analysis among `sources`.
fn chosen(sources: &[SourceInfo]) -> Option<&str> {
    camera_to_analyze(sources).map(|found| found.name.as_str())
}

#[test]
fn the_camera_to_analyze_is_the_first_camera_device_of_its_kind() {
    let intro = source("Intro video", "ffmpeg_source");
    for kind in ["macos-avcapture", "av_capture_input", "macos-avcapture-fast"] {
        let sources = [intro.clone(), source("Webcam", kind), source("Second webcam", kind)];
        assert_eq!(chosen(&sources), Some("Webcam"), "{kind}");
    }
}

#[test]
fn a_capture_source_of_another_system_is_no_camera_here() {
    let sources = [source("Webcam", "dshow_input"), source("Webcam", "v4l2_input")];
    assert_eq!(chosen(&sources), None);
}

#[test]
fn a_video_capture_device_wins_over_a_capture_card_added_before_it() {
    let card = source("Game capture", "macos-avcapture-fast");
    let sources = [card.clone(), source("Webcam", "macos-avcapture")];
    assert_eq!(chosen(&sources), Some("Webcam"));
    let sources = [card.clone(), source("Old webcam", "av_capture_input")];
    assert_eq!(chosen(&sources), Some("Old webcam"));
    assert_eq!(
        chosen(&[card]),
        Some("Game capture"),
        "a capture card is still a camera device"
    );
}

#[test]
fn without_a_camera_device_no_source_is_chosen() {
    let sources = [
        source("Starting soon", "ffmpeg_source"),
        source("Playlist", "vlc_source"),
        source("Studio feed", "ndi_source"),
    ];
    assert_eq!(chosen(&sources), None);
    assert_eq!(chosen(&sources[..1]), None, "not even the only video source");
    assert_eq!(chosen(&[]), None);
}
