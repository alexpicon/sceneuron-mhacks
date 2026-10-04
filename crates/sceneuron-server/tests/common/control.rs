// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Control API helpers: projects with packs, and requests and views of the control token.

use std::collections::BTreeSet;

use sceneuron_core::{Project, default_project, pack_for_canvas};
use sceneuron_server::HubConfig;
use serde_json::Value;

use super::{Reply, TestServer};

/// The default project with these packs added, scaled to its canvas.
pub fn project_with_packs(ids: &[&str]) -> Project {
    let mut project = default_project();
    for id in ids {
        let pack = pack_for_canvas(id, &project.canvas).unwrap_or_else(|| panic!("no pack {id}"));
        project = project.add_pack(&pack).unwrap();
    }
    project
}

/// A server whose project has the meeting and poker packs.
pub fn server_with_decks() -> TestServer {
    TestServer::with(project_with_packs(&["meeting", "poker"]), HubConfig::default())
}

/// The keys of a JSON object.
pub fn keys(value: &Value) -> BTreeSet<&str> {
    value.as_object().unwrap().keys().map(String::as_str).collect()
}

/// The deck with this id from a control view or a snapshot.
pub fn deck<'a>(view: &'a Value, id: &str) -> &'a Value {
    view["decks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|deck| deck["id"] == id)
        .unwrap()
}

/// Posts `body` with the control token.
pub fn control_post(server: &TestServer, target: &str, body: Value) -> Reply {
    server.control("POST", target).body(body.to_string()).send(server.port)
}

/// Posts `body` with the control token and returns the control view it answers with.
pub fn view_after(server: &TestServer, target: &str, body: Value) -> Value {
    let reply = control_post(server, target, body);
    assert_eq!(reply.status, 200, "{}", reply.text());
    reply.json()
}
