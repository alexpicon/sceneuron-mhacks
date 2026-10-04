// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the control event stream: the current view comes first, later views arrive only
//! when what they show changed, streams are pinged while nothing they show changes, and
//! revoking the control token ends the streams opened with it while other streams stay open.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use common::{Item, TestServer, server_with_decks};
use sceneuron_core::Presence;
use serde_json::json;

const WAIT: Duration = Duration::from_secs(3);

/// Long enough for a change to reach a stream that would send it.
const QUIET: Duration = Duration::from_millis(400);

fn raise(server: &TestServer, card: &str) {
    raise_with(server, &server.control_token, card);
}

fn raise_with(server: &TestServer, token: &str, card: &str) {
    let body = json!({"deck": "poker", "card": card, "action": "raise"}).to_string();
    let reply = server
        .with_token(token, "POST", "/api/card")
        .body(body)
        .send(server.port);
    assert_eq!(reply.status, 200, "{}", reply.text());
}

#[test]
fn the_first_event_is_the_current_view() {
    let server = server_with_decks();
    raise(&server, "p3");
    let mut events = server.control_events(&server.control_token);
    assert_eq!(events.status, 200);
    assert_eq!(events.header("Content-Type"), Some("text/event-stream"));
    assert_eq!(events.header("Cache-Control"), Some("no-store"));
    assert_eq!(events.header("Access-Control-Allow-Origin"), None);
    let first = events.next_control(WAIT).expect("no first view");
    assert_eq!(first["revision"], server.hub.snapshot().revision);
    assert_eq!(first["decks"][1]["raised"], "p3");
    assert!(first.get("events").is_none());
}

#[test]
fn only_changes_to_the_view_are_sent() {
    let server = server_with_decks();
    let mut events = server.control_events(&server.control_token);
    let first = events.next_control(WAIT).expect("no first view");
    server
        .hub
        .update(|engine, now| engine.note("system", "saved the project again", now));
    let scene = sceneuron_core::SceneObservation {
        caption: "a person at a desk".to_owned(),
        verdicts: Vec::new(),
    };
    server.hub.update(|engine, now| engine.observe_scene(scene, now));
    assert!(server.hub.snapshot().revision > first["revision"].as_u64().unwrap());
    assert_eq!(events.next(QUIET), None, "changes outside the view are not sent");

    raise(&server, "p8");
    let raised = events.next_control(WAIT).expect("no view after a raise");
    assert_eq!(raised["decks"][1]["raised"], "p8");
    assert_eq!(raised["revision"], server.hub.snapshot().revision);
    raise(&server, "p8");
    assert_eq!(
        events.next(QUIET),
        None,
        "raising the raised card again changes nothing"
    );

    let reply = server.post_json("/api/simulate", r#"{"presence":"away"}"#);
    assert_eq!(reply.status, 200);
    let shown = events
        .next_control(WAIT)
        .expect("no view after a rule changed an element");
    assert_eq!(server.hub.snapshot().presence.state, Presence::Away);
    assert_ne!(shown["elements"], raised["elements"]);
}

#[test]
fn a_stream_is_pinged_while_only_other_things_change() {
    let server = TestServer::start();
    let mut events = server.control_events(&server.control_token);
    events.next_control(WAIT).expect("no first view");
    let started = Instant::now();
    let stop = Arc::new(AtomicBool::new(false));
    let notes = {
        let (hub, stop) = (Arc::clone(&server.hub), Arc::clone(&stop));
        thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                hub.update(|engine, now| engine.note("system", "a note", now));
                thread::sleep(Duration::from_millis(50));
            }
        })
    };
    let item = events.next(Duration::from_secs(20));
    stop.store(true, Ordering::SeqCst);
    notes.join().unwrap();
    assert_eq!(item, Some(Item::Comment("ping".to_owned())));
    assert!(
        started.elapsed() >= Duration::from_secs(14),
        "pinged after {:?}",
        started.elapsed()
    );
}

#[test]
fn revoking_ends_the_streams_opened_with_the_control_token() {
    let server = server_with_decks();
    let mut control = server.control_events(&server.control_token);
    let mut second = server.control_events(&server.control_token);
    let mut editor_control = server.control_events(&server.token);
    let mut editor_state = server.events();
    for stream in [&mut control, &mut second, &mut editor_control] {
        assert!(stream.next_control(WAIT).is_some());
    }
    assert!(editor_state.next_state(WAIT).is_some());

    let reply = server.api("POST", "/api/control/revoke").send(server.port);
    assert_eq!(reply.status, 200, "{}", reply.text());
    assert!(control.ends_within(WAIT), "a revoked client's stream stayed open");
    assert!(second.ends_within(WAIT));

    let new_token = server.host.saved_control_token().expect("the new token was not saved");
    let mut paired_again = server.control_events(&new_token);
    assert!(paired_again.next_control(WAIT).is_some());
    raise_with(&server, &new_token, "p1");
    assert_eq!(
        editor_control.next_control(WAIT).expect("the editor's stream ended")["decks"][1]["raised"],
        "p1"
    );
    assert_eq!(
        paired_again.next_control(WAIT).expect("the new stream ended")["decks"][1]["raised"],
        "p1"
    );
    assert_eq!(
        editor_state.next_state(WAIT).expect("the snapshot stream ended")["decks"][1]["raised"],
        "p1"
    );
}

#[test]
fn the_old_control_token_cannot_open_a_stream_after_a_revoke() {
    let server = TestServer::start();
    assert_eq!(server.api("POST", "/api/control/revoke").send(server.port).status, 200);
    let refused = server.control_events(&server.control_token);
    assert_eq!(refused.status, 401);
}
