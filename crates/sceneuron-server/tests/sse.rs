// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the event stream: the first event is the current snapshot, later changes
//! arrive, idle pings, HTTP/1.0 streams, the stream limit, and ending open streams when
//! the server stops or the hub shuts down.

mod common;

use std::time::{Duration, Instant};

use common::{EventStream, Item, TestServer};
use sceneuron_core::Presence;

const WAIT: Duration = Duration::from_secs(3);

#[test]
fn the_first_event_is_the_current_state_and_updates_follow() {
    let server = TestServer::start();
    let mut events = server.events();
    assert_eq!(events.status, 200);
    assert_eq!(events.header("Content-Type"), Some("text/event-stream"));
    assert_eq!(events.header("Cache-Control"), Some("no-store"));
    assert_eq!(events.header("X-Content-Type-Options"), Some("nosniff"));
    assert_eq!(events.header("Transfer-Encoding"), Some("chunked"));
    let first = events.next_state(WAIT).expect("no first event");
    assert_eq!(first["revision"], server.hub.snapshot().revision);
    assert_eq!(first["presence"]["state"], "unknown");

    server
        .hub
        .update(|engine, now| engine.simulate_presence(Presence::Away, now));
    let next = events.next_state(WAIT).expect("no update event");
    assert_eq!(next["presence"]["state"], "away");
    assert!(next["revision"].as_u64() > first["revision"].as_u64());

    let reply = server.post_json("/api/simulate", r#"{"presence":"present"}"#);
    assert_eq!(reply.status, 200);
    let latest = events.next_state(WAIT).expect("no event for an API change");
    assert_eq!(latest["presence"]["state"], "present");
}

#[test]
fn an_idle_stream_is_pinged() {
    let server = TestServer::start();
    let mut events = server.events();
    events.next_state(WAIT).expect("no first event");
    let started = Instant::now();
    let item = events.next(Duration::from_secs(20));
    assert_eq!(item, Some(Item::Comment("ping".to_owned())));
    assert!(
        started.elapsed() >= Duration::from_secs(14),
        "pinged after {:?}",
        started.elapsed()
    );
}

#[test]
fn http_1_0_clients_get_an_unframed_stream() {
    let mut server = TestServer::start();
    let target = format!("/api/events?token={}", server.token);
    let mut events = EventStream::open(server.port, &target, "1.0");
    assert_eq!(events.status, 200);
    assert_eq!(events.header("Transfer-Encoding"), None);
    assert!(events.next_state(WAIT).is_some());
    server.stop();
    assert!(events.ends_within(WAIT));
}

#[test]
fn streams_beyond_the_limit_are_refused() {
    let server = TestServer::start();
    let mut open: Vec<EventStream> = (0..32).map(|_| server.events()).collect();
    for events in &mut open {
        assert_eq!(events.status, 200);
        assert!(events.next_state(WAIT).is_some());
    }
    let mut refused = server.events();
    assert_eq!(refused.status, 503);
    assert!(refused.ends_within(WAIT));
    // The server notices a closed stream when it next writes to it.
    drop(open.pop());
    let mut away = true;
    assert!(common::wait_until(WAIT, || {
        let presence = if away { Presence::Away } else { Presence::Present };
        away = !away;
        server.hub.update(|engine, now| engine.simulate_presence(presence, now));
        server.events().status == 200
    }));
}

#[test]
fn stopping_the_server_ends_open_streams_quickly() {
    let mut server = TestServer::start();
    let mut streams: Vec<EventStream> = (0..3).map(|_| server.events()).collect();
    for events in &mut streams {
        assert!(events.next_state(WAIT).is_some());
    }
    let took = server.stop();
    assert!(took < Duration::from_secs(1), "stop took {took:?}");
    for events in &mut streams {
        assert!(events.ends_within(WAIT), "a stream stayed open after stop");
    }
}

#[test]
fn hub_shutdown_ends_open_streams() {
    let server = TestServer::start();
    let mut events = server.events();
    assert!(events.next_state(WAIT).is_some());
    server.hub.shutdown();
    assert!(events.ends_within(WAIT));
}
