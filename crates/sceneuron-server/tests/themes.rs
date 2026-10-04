// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises overlay themes through the server: a saved theme reaches the project, the state, and the
//! event stream the overlay draws from; a theme the core does not know is refused without changing
//! anything; and the overlay page draws exactly the themes a project may choose.

mod common;

use std::path::Path;
use std::time::Duration;

use common::TestServer;
use sceneuron_core::Theme;
use serde_json::json;

/// The theme ids the overlay draws, in the order `web/render-themes.js` lists them.
fn overlay_theme_ids() -> Vec<String> {
    let script = include_str!("../../../web/render-themes.js");
    let line = script
        .lines()
        .find(|line| line.contains("const THEME_IDS = "))
        .expect("render-themes.js lists THEME_IDS on one line");
    line.split('"').skip(1).step_by(2).map(str::to_owned).collect()
}

#[test]
fn a_saved_theme_reaches_the_project_the_state_and_the_event_stream() {
    let server = TestServer::start();
    let mut events = server.events();
    let first = events.next_state(Duration::from_secs(5)).unwrap();
    assert_eq!(first["theme"], "none");
    // From the last theme back to none, so every save changes the theme.
    for theme in Theme::ALL.into_iter().rev() {
        let mut project = server.get_json("/api/project");
        project["theme"] = json!(theme.id());
        let reply = server
            .api("PUT", "/api/project")
            .body(project.to_string())
            .send(server.port);
        assert_eq!(reply.status, 200, "{}", reply.text());
        assert_eq!(reply.json()["theme"], theme.id());
        assert_eq!(server.get_json("/api/project")["theme"], theme.id());
        assert_eq!(server.get_json("/api/state")["theme"], theme.id());
        let streamed = (0..10)
            .filter_map(|_| events.next_state(Duration::from_secs(5)))
            .find(|state| state["theme"] == theme.id());
        assert!(streamed.is_some(), "no state with the {} theme", theme.id());
    }
}

#[test]
fn a_theme_the_core_does_not_know_is_refused_and_nothing_changes() {
    let server = TestServer::start();
    let before = server.get_json("/api/project");
    let revision = server.hub.snapshot().revision;
    for theme in [json!("y2k"), json!("Windows 95"), json!(""), json!(null), json!(1)] {
        let mut project = before.clone();
        project["theme"] = theme.clone();
        let reply = server
            .api("PUT", "/api/project")
            .body(project.to_string())
            .send(server.port);
        assert_eq!(reply.status, 400, "{theme}: {}", reply.text());
        assert!(
            reply.error().starts_with("invalid request body: "),
            "{theme}: {}",
            reply.error()
        );
    }
    let mut project = before.clone();
    project["theme"] = json!("y2k");
    let reply = server
        .api("PUT", "/api/project")
        .body(project.to_string())
        .send(server.port);
    assert!(reply.error().contains("unknown variant `y2k`"), "{}", reply.error());
    assert_eq!(server.get_json("/api/project"), before);
    assert_eq!(server.hub.snapshot().revision, revision);
}

#[test]
fn the_overlay_draws_exactly_the_themes_a_project_may_choose() {
    let ids: Vec<&str> = Theme::ALL.iter().map(|theme| theme.id()).collect();
    assert_eq!(overlay_theme_ids(), ids);
    let themes = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/themes");
    for id in ids.iter().filter(|id| **id != "none") {
        assert!(themes.join(format!("{id}.js")).is_file(), "web/themes/{id}.js");
    }
}
