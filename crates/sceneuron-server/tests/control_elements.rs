// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises element requests: show, hide, toggle, and release, and a toggle that reads and
//! writes the visibility in one change.

mod common;

use std::sync::Arc;
use std::thread;

use common::{TestServer, control_post, element, view_after};
use serde_json::json;

#[test]
fn elements_show_hide_toggle_and_release() {
    let server = TestServer::start();
    let act = |action: &str| {
        view_after(
            &server,
            "/api/control/element",
            json!({"element": "brb", "action": action}),
        )
    };
    let start = server.control("GET", "/api/control/state").send(server.port).json();
    let base = element(&start, "brb")["visible"].as_bool().unwrap();
    let hidden = act("hide");
    assert_eq!(
        (&element(&hidden, "brb")["visible"], &element(&hidden, "brb")["held"]),
        (&json!(false), &json!(true))
    );
    let shown = act("show");
    assert_eq!(element(&shown, "brb")["visible"], true);
    let toggled = act("toggle");
    assert_eq!(element(&toggled, "brb")["visible"], false);
    assert_eq!(
        toggled["revision"].as_u64(),
        shown["revision"].as_u64().map(|revision| revision + 1)
    );
    let released = act("release");
    assert_eq!(
        (
            &element(&released, "brb")["visible"],
            &element(&released, "brb")["held"]
        ),
        (&json!(base), &json!(false))
    );
    for (body, status) in [
        (json!({"element": "nope", "action": "toggle"}), 404),
        (json!({"element": "nope", "action": "release"}), 404),
        (json!({"element": "brb", "action": "flip"}), 400),
        (json!({"element": "brb"}), 400),
    ] {
        let reply = control_post(&server, "/api/control/element", body.clone());
        assert_eq!(reply.status, status, "{body}: {}", reply.text());
        assert!(!reply.error().is_empty());
    }
}

#[test]
fn a_toggle_reads_and_writes_in_one_change() {
    let server = Arc::new(TestServer::start());
    let before = server.hub.snapshot();
    let visible = |snapshot: &sceneuron_core::Snapshot| {
        snapshot
            .elements
            .iter()
            .find(|element| element.id == "brb")
            .unwrap()
            .props
            .visible
    };
    let toggles: Vec<_> = (0..16)
        .map(|_| {
            let server = Arc::clone(&server);
            thread::spawn(move || {
                control_post(
                    &server,
                    "/api/control/element",
                    json!({"element": "brb", "action": "toggle"}),
                )
                .status
            })
        })
        .collect();
    for toggle in toggles {
        assert_eq!(toggle.join().unwrap(), 200);
    }
    let after = server.hub.snapshot();
    assert_eq!(
        visible(&after),
        visible(&before),
        "an even number of toggles cancels out"
    );
    assert_eq!(after.revision, before.revision + 16);
}
