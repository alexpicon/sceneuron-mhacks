// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises OBS's program through the server: a host's report reaches the state and the event
//! stream the overlay draws from, with the on-air pill's text and the elements that show the scene
//! or the time on air marked for the overlay; and the starter project's pill and timer, and its
//! cards' raise times, as the overlay page reads them.

mod common;

use std::time::Duration;

use common::{TestServer, element};
use sceneuron_core::{CardAction, starter_project};
use sceneuron_server::HubConfig;
use serde_json::json;

fn starter_server() -> TestServer {
    TestServer::with(starter_project().unwrap(), HubConfig::default())
}

#[test]
fn a_report_reaches_the_state_and_the_event_stream_with_the_pill_and_timer_marked() {
    let server = starter_server();
    let mut events = server.events();
    let first = events.next_state(Duration::from_secs(5)).unwrap();
    assert_eq!(first["theme"], "graphite");
    assert_eq!(first["on_air"]["since_ms"], json!(null));
    let pill = element(&first, "on-air");
    assert_eq!((&pill["on_air"], &pill["clock"]), (&json!(true), &json!(false)));
    let timer = element(&first, "on-air-time");
    assert_eq!((&timer["on_air"], &timer["clock"]), (&json!(false), &json!(true)));
    assert_eq!(
        timer["props"]["text"], "{{on-air-time}}",
        "the overlay fills in the time"
    );
    assert_eq!(element(&first, "name-label")["on_air"], false);

    server
        .hub
        .update(|engine, now| engine.set_on_air("Scene 01", true, false, now));
    let live = (0..10)
        .filter_map(|_| events.next_state(Duration::from_secs(5)))
        .find(|state| state["on_air"]["streaming"] == true)
        .expect("a state on air");
    let since = live["on_air"]["since_ms"].as_u64().unwrap();
    assert!(since <= live["now_ms"].as_u64().unwrap());
    assert_eq!(element(&live, "on-air")["props"]["text"], "Scene 01 \u{b7} LIVE");
    assert_eq!(server.get_json("/api/state")["on_air"]["scene"], "Scene 01");

    server
        .hub
        .update(|engine, now| engine.set_on_air("Scene 01", false, false, now));
    let state = server.get_json("/api/state");
    assert_eq!(
        state["on_air"],
        json!({"scene": "Scene 01", "streaming": false, "recording": false, "since_ms": null})
    );
    assert_eq!(element(&state, "on-air")["props"]["text"], "Scene 01");
}

#[test]
fn the_control_view_leaves_the_program_out() {
    let server = starter_server();
    server
        .hub
        .update(|engine, now| engine.set_on_air("Scene 01", true, true, now));
    let view = server.control("GET", "/api/control/state").send(server.port).json();
    assert!(view.get("on_air").is_none(), "{view}");
}

#[test]
fn decks_say_when_their_cards_went_up_so_the_newest_can_be_marked() {
    let server = starter_server();
    for (deck, card) in [("meeting", "wrap-up"), ("status", "muted")] {
        server
            .hub
            .update(|engine, now| engine.card(deck, Some(card), CardAction::Raise, now).unwrap());
        std::thread::sleep(Duration::from_millis(5));
    }
    let state = server.get_json("/api/state");
    let raised_at = |id: &str| {
        let decks = state["decks"].as_array().unwrap();
        decks.iter().find(|deck| deck["id"] == id).unwrap()["raised_at_ms"].clone()
    };
    let (meeting, status) = (raised_at("meeting"), raised_at("status"));
    assert!(
        meeting.as_u64().unwrap() < status.as_u64().unwrap(),
        "{meeting} {status}"
    );
    assert_eq!(raised_at("poker"), json!(null));
}
