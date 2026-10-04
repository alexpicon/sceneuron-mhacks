// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the simulated voice of sceneuron-dev --demo-ai over HTTP: voice settings need
//! consent, letting go of push-to-talk hears a scripted command that raises a card on a deck with
//! voice on, and a card raised on a deck that speaks gets a chime the overlay may play.

mod common;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};

use common::{Running, WAIT};
use serde_json::{Value, json};

/// Polls `path` until `ready` accepts its JSON.
fn when(running: &Running, path: &str, what: &str, ready: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + WAIT;
    loop {
        let (_, value) = running.request("GET", path, "");
        if ready(&value) {
            return value;
        }
        assert!(Instant::now() < deadline, "{what} never happened: {value}");
        thread::sleep(Duration::from_millis(50));
    }
}

/// Saves the project with the meeting pack, its voice and speech on.
fn add_meeting_pack(running: &Running) {
    let (_, packs) = running.request("GET", "/api/packs", "");
    let pack = packs
        .as_array()
        .unwrap()
        .iter()
        .find(|pack| pack["id"] == "meeting")
        .unwrap()
        .clone();
    let (_, mut saved) = running.request("GET", "/api/project", "");
    let elements = saved["elements"].as_array_mut().unwrap();
    elements.extend(pack["elements"].as_array().unwrap().iter().cloned());
    let mut deck = pack["decks"][0].clone();
    deck["voice"] = json!(true);
    deck["speak"] = json!(true);
    saved["decks"] = json!([deck]);
    let (status, reply) = running.request("PUT", "/api/project", &saved.to_string());
    assert_eq!(status, 200, "{reply}");
}

/// The status line, content type, and body of a GET with the editor token.
fn get_raw(running: &Running, path: &str) -> (String, Vec<u8>) {
    let mut stream = TcpStream::connect(("127.0.0.1", running.port)).unwrap();
    let head = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\nX-Sceneuron-Token: {}\r\n\r\n",
        running.port, running.token
    );
    stream.write_all(head.as_bytes()).unwrap();
    let mut reply = Vec::new();
    stream.read_to_end(&mut reply).unwrap();
    let end = reply.windows(4).position(|window| window == b"\r\n\r\n").unwrap();
    (
        String::from_utf8_lossy(&reply[..end]).into_owned(),
        reply[end + 4..].to_vec(),
    )
}

#[test]
fn letting_go_of_push_to_talk_hears_a_scripted_command() {
    let running = Running::start(&["--port", "0", "--demo-ai"]);
    let (_, settings) = running.request("GET", "/api/ai", "");
    assert_eq!(settings["voice"]["enabled"], false);
    assert_eq!(
        (
            settings["has_elevenlabs_key"].clone(),
            settings["elevenlabs_key_hint"].clone()
        ),
        (json!(true), json!("demo"))
    );
    let (status, _) = running.request("PUT", "/api/ai", r#"{"voice":{"enabled":true}}"#);
    assert_eq!(status, 400, "listening needs consent");
    add_meeting_pack(&running);
    let (status, settings) = running.request("PUT", "/api/ai", r#"{"voice":{"consent":true,"enabled":true}}"#);
    assert_eq!((status, &settings["voice"]["enabled"]), (200, &json!(true)));
    let voice = when(&running, "/api/voice", "voice on", |voice| voice["enabled"] == true);
    assert!(voice["message"].as_str().unwrap().starts_with("Simulated voice"));
    assert_eq!(running.request("POST", "/api/voice/talk", r#"{"down":true}"#).0, 200);
    when(&running, "/api/voice", "talking", |voice| voice["talking"] == true);
    assert_eq!(running.request("POST", "/api/voice/talk", r#"{"down":false}"#).0, 200);
    let voice = when(&running, "/api/voice", "a heard command", |voice| {
        !voice["heard"].is_null()
    });
    assert_eq!(voice["heard"]["text"], "Sceneuron, wrap it up");
    assert_eq!(voice["heard"]["result"], "raised Meeting signals: Wrap it up");
    let (_, state) = running.request("GET", "/api/state", "");
    let deck = state["decks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|deck| deck["id"] == "meeting")
        .unwrap()
        .clone();
    assert_eq!(
        (deck["raised"].clone(), deck["raised_by"].clone()),
        (json!("wrap-up"), json!("voice"))
    );
}

#[test]
fn a_card_on_a_speaking_deck_gets_a_chime() {
    let running = Running::start(&["--port", "0", "--demo-ai"]);
    add_meeting_pack(&running);
    let (status, _) = running.request("PUT", "/api/ai", r#"{"voice":{"speak":true}}"#);
    assert_eq!(status, 200);
    let (status, _) = running.request(
        "POST",
        "/api/card",
        r#"{"deck":"meeting","card":"agree","action":"raise"}"#,
    );
    assert_eq!(status, 200);
    let state = when(&running, "/api/state", "the chime", |state| {
        state["speech"]["audio"].is_string()
    });
    assert_eq!(state["speech"]["text"], "Agree");
    let id = state["speech"]["audio"].as_str().unwrap();
    let (head, body) = get_raw(&running, &format!("/api/speech/{id}"));
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(head.contains("Content-Type: audio/wav"), "{head}");
    assert_eq!(&body[..4], b"RIFF");
}
