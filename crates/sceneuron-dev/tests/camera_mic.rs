// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the sceneuron-dev binary's Camera & mic tab: with demo presence, a simulated camera
//! picture once a simulated camera is analyzed, marked simulated and different each time, the
//! simulated microphones and choosing one, and without demo presence, neither.

mod common;

use std::io::{Read, Write};
use std::net::TcpStream;

use common::{Running, WAIT};
use serde_json::json;

/// The status, the headers, and the body of a GET with the editor token.
fn get(running: &Running, path: &str) -> (u16, String, Vec<u8>) {
    let mut stream = TcpStream::connect(("127.0.0.1", running.port)).unwrap();
    stream.set_read_timeout(Some(WAIT)).unwrap();
    let head = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\nX-Sceneuron-Token: {}\r\n\r\n",
        running.port, running.token
    );
    stream.write_all(head.as_bytes()).unwrap();
    let mut reply = Vec::new();
    stream.read_to_end(&mut reply).unwrap();
    let end = reply.windows(4).position(|window| window == b"\r\n\r\n").unwrap();
    let head = String::from_utf8(reply[..end].to_vec()).unwrap();
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    (status, head, reply[end + 4..].to_vec())
}

#[test]
fn demo_presence_offers_a_simulated_picture_and_microphones() {
    let running = Running::start(&["--port", "0", "--demo-presence"]);
    assert_eq!(
        running.request("GET", "/api/camera/preview", ""),
        (200, json!({"reason": "no_source"}))
    );
    let (_, sources) = running.request("GET", "/api/sources", "");
    let uuid = sources[0]["uuid"].as_str().unwrap().to_owned();
    running.request("POST", "/api/sources/attach", &json!({ "uuid": uuid }).to_string());

    let (status, head, first) = get(&running, "/api/camera/preview");
    assert_eq!(status, 200);
    assert!(head.contains("Content-Type: image/jpeg"), "{head}");
    assert!(head.contains("X-Sceneuron-Simulated: true"), "{head}");
    assert!(head.contains("Cache-Control: no-store"), "{head}");
    assert!(first.starts_with(&[0xFF, 0xD8, 0xFF]) && first.ends_with(&[0xFF, 0xD9]));
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert_ne!(get(&running, "/api/camera/preview").2, first, "the sweeping band moved");
    let (_, analysis) = running.request("GET", "/api/camera/analysis", "");
    assert_eq!(analysis["age_ms"], 0, "{analysis}");

    let (_, audio) = running.request("GET", "/api/audio-sources", "");
    assert_eq!(audio["available"], true);
    let names: Vec<&str> = audio["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|source| source["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Mic/Aux (simulated)", "Desktop Audio (simulated)"]);
    let mic = audio["sources"][0]["uuid"].as_str().unwrap().to_owned();
    let desktop = audio["sources"][1]["uuid"].as_str().unwrap().to_owned();
    let attach = |uuid: &str| {
        running.request(
            "POST",
            "/api/audio-sources/attach",
            &json!({ "uuid": uuid }).to_string(),
        )
    };
    assert_eq!(attach(&desktop), (200, json!({"ok": true})));
    assert_eq!(attach(&mic), (200, json!({"ok": true})));
    let (_, audio) = running.request("GET", "/api/audio-sources", "");
    let marks = |index: usize| {
        (
            audio["sources"][index]["voice"].clone(),
            audio["sources"][index]["in_use"].clone(),
        )
    };
    assert_eq!(marks(0), (json!(true), json!(true)));
    assert_eq!(marks(1), (json!(false), json!(false)), "the filter moved");
    assert_eq!(attach("nope").0, 400);
    running.line_starting("Voice control and subtitles listen to Mic/Aux (simulated)");
}

#[test]
fn without_demo_presence_the_tab_is_told_this_is_not_obs() {
    let running = Running::start(&["--port", "0"]);
    assert_eq!(
        running.request("GET", "/api/camera/preview", ""),
        (200, json!({"reason": "not_in_obs"}))
    );
    let (_, sources) = running.request("GET", "/api/sources", "");
    let uuid = sources[0]["uuid"].as_str().unwrap().to_owned();
    running.request("POST", "/api/sources/attach", &json!({ "uuid": uuid }).to_string());
    assert_eq!(
        running.request("GET", "/api/camera/preview", ""),
        (200, json!({"reason": "not_in_obs"}))
    );
    let analysis = json!({ "seen": [], "faces": [], "age_ms": null });
    assert_eq!(running.request("GET", "/api/camera/analysis", ""), (200, analysis));
    let audio = json!({ "available": false, "sources": [] });
    assert_eq!(running.request("GET", "/api/audio-sources", ""), (200, audio));
    let (status, reply) = running.request("POST", "/api/audio-sources/attach", r#"{"uuid":"x"}"#);
    assert_eq!(
        (status, reply["error"].as_str()),
        (400, Some("Sceneuron is not running inside OBS"))
    );
}
