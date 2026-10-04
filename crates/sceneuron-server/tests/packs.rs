// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the packs request: every pack, scaled to the saved project's canvas, and only for
//! the editor token.

mod common;

use common::{Req, TestServer};
use sceneuron_core::{Canvas, default_project, pack_for_canvas, packs};
use sceneuron_server::HubConfig;
use serde_json::Value;

fn ids(list: &Value) -> Vec<&str> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|pack| pack["id"].as_str().unwrap())
        .collect()
}

#[test]
fn packs_are_scaled_to_the_saved_canvas() {
    let mut project = default_project();
    project.canvas = Canvas {
        width: 1280,
        height: 720,
    };
    let canvas = project.canvas;
    let server = TestServer::with(project, HubConfig::default());
    let list = server.get_json("/api/packs");
    let expected: Vec<String> = packs().into_iter().map(|pack| pack.id).collect();
    assert_eq!(ids(&list), expected);
    assert_eq!(
        ids(&list),
        [
            "meeting",
            "poker",
            "classroom",
            "status",
            "subtitles",
            "live-description"
        ]
    );
    for pack in list.as_array().unwrap() {
        let id = pack["id"].as_str().unwrap();
        let scaled = serde_json::to_value(pack_for_canvas(id, &canvas).unwrap()).unwrap();
        assert_eq!(pack, &scaled, "{id}");
        assert!(
            ["name", "description", "elements", "decks"]
                .iter()
                .all(|key| pack.get(key).is_some())
        );
    }
    let meeting = &list[0]["elements"][0]["props"];
    assert_eq!(meeting["width"], (1160.0_f64 * 1280.0 / 1920.0).round());
}

#[test]
fn packs_follow_a_saved_canvas_change() {
    let server = TestServer::start();
    let before = server.get_json("/api/packs")[1]["elements"][0]["props"]["width"].clone();
    let mut project = server.hub.read(|engine| engine.project().clone());
    project.canvas = Canvas {
        width: 3840,
        height: 2160,
    };
    let saved = server
        .api("PUT", "/api/project")
        .body(serde_json::to_string(&project).unwrap());
    assert_eq!(saved.send(server.port).status, 200);
    let after = server.get_json("/api/packs")[1]["elements"][0]["props"]["width"].clone();
    assert_eq!(after.as_f64().unwrap(), before.as_f64().unwrap() * 2.0);
}

#[test]
fn only_the_editor_token_lists_packs() {
    let server = TestServer::start();
    assert_eq!(server.control("GET", "/api/packs").send(server.port).status, 403);
    assert_eq!(Req::new("GET", "/api/packs").send(server.port).status, 401);
    let post = server.api("POST", "/api/packs").send(server.port);
    assert_eq!((post.status, post.header("Allow")), (405, Some("GET")));
}
