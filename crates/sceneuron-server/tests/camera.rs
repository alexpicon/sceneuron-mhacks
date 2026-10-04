// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the Camera & mic endpoints: the camera preview as a JPEG that is never stored, the
//! reason it gives when there is no picture, what on-device analysis found, the sources that
//! carry audio and adding the voice filter to one, that only the editor token reaches any of
//! them, and what the host outside OBS answers.

mod common;

use std::sync::atomic::Ordering;

use common::{FAKE_JPEG, FAKE_MIC, Req, TestServer, start_server};
use sceneuron_core::default_project;
use sceneuron_server::{Hub, HubConfig, PreviewGap, PreviewImage, ServerConfig, generate_token};
use serde_json::json;

const PREVIEW: &str = "/api/camera/preview";
const ANALYSIS: &str = "/api/camera/analysis";
const AUDIO: &str = "/api/audio-sources";
const ATTACH_VOICE: &str = "/api/audio-sources/attach";

/// Every Camera & mic request, with a body the API accepts.
const ROUTES: &[(&str, &str, &str)] = &[
    ("GET", PREVIEW, ""),
    ("GET", ANALYSIS, ""),
    ("GET", AUDIO, ""),
    ("POST", ATTACH_VOICE, r#"{"uuid":"mic-1"}"#),
];

fn overlay_token(server: &TestServer) -> String {
    let url = server.server.as_ref().unwrap().overlay_url();
    url.split("token=").nth(1).unwrap().to_owned()
}

#[test]
fn the_editor_gets_the_preview_as_a_jpeg_that_is_never_stored() {
    let server = TestServer::start();
    let reply = server.api("GET", PREVIEW).send(server.port);
    assert_eq!(reply.status, 200);
    assert_eq!(reply.header("Content-Type"), Some("image/jpeg"));
    assert_eq!(reply.header("Cache-Control"), Some("no-store"));
    assert_eq!(reply.header("X-Content-Type-Options"), Some("nosniff"));
    assert_eq!(reply.header("X-Sceneuron-Simulated"), None);
    assert_eq!(reply.body, FAKE_JPEG);
    assert_eq!(server.host.camera_requests.load(Ordering::SeqCst), 1);
}

#[test]
fn a_test_picture_is_marked_simulated() {
    let server = TestServer::start();
    *server.host.preview.lock().unwrap() = Ok(PreviewImage {
        jpeg: FAKE_JPEG.to_vec(),
        simulated: true,
    });
    let reply = server.api("GET", PREVIEW).send(server.port);
    assert_eq!(reply.header("Content-Type"), Some("image/jpeg"));
    assert_eq!(reply.header("X-Sceneuron-Simulated"), Some("true"));
}

#[test]
fn without_a_picture_the_preview_says_why_without_failing() {
    let server = TestServer::start();
    for (gap, reason) in [
        (PreviewGap::NoSource, "no_source"),
        (PreviewGap::Waiting, "waiting"),
        (PreviewGap::Stale, "stale"),
        (PreviewGap::NotInObs, "not_in_obs"),
    ] {
        *server.host.preview.lock().unwrap() = Err(gap);
        let reply = server.api("GET", PREVIEW).send(server.port);
        assert_eq!(reply.status, 200, "{reason}");
        assert_eq!(reply.header("Content-Type"), Some("application/json"));
        assert_eq!(reply.header("Cache-Control"), Some("no-store"));
        assert_eq!(reply.json(), json!({ "reason": reason }));
    }
}

#[test]
fn the_analysis_lists_what_was_seen_and_the_face_boxes() {
    let server = TestServer::start();
    let analysis = server.get_json(ANALYSIS);
    assert_eq!(analysis["seen"], json!(["face in view (0.98)"]));
    assert_eq!(
        analysis["faces"],
        json!([{"x1": 0.25, "y1": 0.125, "x2": 0.5, "y2": 0.5, "score": 0.98}])
    );
    assert_eq!(analysis["age_ms"], 40);
}

#[test]
fn audio_sources_are_listed_and_the_voice_filter_is_added_through_the_host() {
    let server = TestServer::start();
    let listed = server.get_json(AUDIO);
    let mic = json!({
        "uuid": FAKE_MIC,
        "name": "Mic/Aux",
        "kind": "coreaudio_input_capture",
        "global": true,
        "voice": false,
        "in_use": false,
    });
    assert_eq!(listed, json!({ "available": true, "sources": [mic] }));
    let reply = server.post_json(ATTACH_VOICE, &json!({ "uuid": FAKE_MIC }).to_string());
    assert_eq!(reply.status, 200, "{}", reply.text());
    assert_eq!(reply.json(), json!({ "ok": true }));
    assert_eq!(*server.host.voice_attached.lock().unwrap(), vec![FAKE_MIC.to_owned()]);
    let source = &server.get_json(AUDIO)["sources"][0];
    assert_eq!(
        (source["voice"].clone(), source["in_use"].clone()),
        (json!(true), json!(true))
    );
    let reply = server.post_json(ATTACH_VOICE, r#"{"uuid":"other"}"#);
    assert_eq!(reply.status, 400);
    assert_eq!(reply.error(), "no audio source other");
    assert_eq!(server.post_json(ATTACH_VOICE, "{}").status, 400);
    assert_eq!(server.post_json(ATTACH_VOICE, r#"{"uuid":"mic-1","x":1}"#).status, 400);
    assert_eq!(server.api("GET", ATTACH_VOICE).send(server.port).status, 405);
    assert_eq!(server.api("POST", PREVIEW).send(server.port).status, 405);
}

#[test]
fn only_the_editor_token_reaches_the_camera_and_the_microphones() {
    let server = TestServer::start();
    let overlay = overlay_token(&server);
    for (method, target, body) in ROUTES {
        let with = |request: Req| {
            let request = if body.is_empty() { request } else { request.body(*body) };
            request.send(server.port)
        };
        let reply = with(server.with_token(&overlay, method, target));
        assert_eq!(reply.status, 403, "overlay {method} {target}: {}", reply.text());
        let reply = with(server.control(method, target));
        assert_eq!(reply.status, 403, "control {method} {target}: {}", reply.text());
        let reply = with(Req::new(method, &format!("{target}?token={overlay}")));
        assert_eq!(reply.status, 403, "overlay query {method} {target}: {}", reply.text());
        let reply = with(Req::new(method, target));
        assert_eq!(reply.status, 401, "no token {method} {target}: {}", reply.text());
    }
    assert_eq!(
        server.host.camera_requests.load(Ordering::SeqCst),
        0,
        "no picture was taken"
    );
    assert!(server.host.voice_attached.lock().unwrap().is_empty());
    let preview = Req::new("GET", &format!("{PREVIEW}?token={}", server.token)).send(server.port);
    assert_eq!(preview.header("Content-Type"), Some("image/jpeg"));
}

#[test]
fn outside_obs_there_is_no_camera_picture_and_no_microphone() {
    let hub = Hub::start(default_project(), HubConfig::default()).unwrap();
    let token = generate_token();
    let config = ServerConfig {
        port: 0,
        token: token.clone(),
        ..ServerConfig::default()
    };
    let server = start_server(hub, config).unwrap();
    let request = |method: &str, path: &str| Req::new(method, path).header("X-Sceneuron-Token", &token);
    let reply = request("GET", PREVIEW).send(server.port());
    assert_eq!((reply.status, reply.json()), (200, json!({ "reason": "not_in_obs" })));
    let reply = request("GET", ANALYSIS).send(server.port());
    assert_eq!(reply.json(), json!({ "seen": [], "faces": [], "age_ms": null }));
    let reply = request("GET", AUDIO).send(server.port());
    assert_eq!(reply.json(), json!({ "available": false, "sources": [] }));
    let reply = request("POST", ATTACH_VOICE)
        .body(r#"{"uuid":"x"}"#)
        .send(server.port());
    assert_eq!(reply.status, 400);
    assert_eq!(reply.error(), "Sceneuron is not running inside OBS");
}
