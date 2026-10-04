// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises listener delivery: events from changes made on many threads arrive in change
//! order, and a listener that changes the hub has its change delivered after it returns.

mod common;

use std::sync::{Arc, Mutex};
use std::thread;

use common::hold_text;
use sceneuron_core::{Presence, default_project};
use sceneuron_server::{Hub, HubConfig, HubEvent};

#[test]
fn events_from_concurrent_changes_arrive_in_change_order() {
    let hub = Hub::start(default_project(), HubConfig::default()).unwrap();
    let revisions = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&revisions);
    hub.add_listener(Box::new(move |event| {
        if let HubEvent::StateChanged { revision } = event {
            sink.lock().unwrap().push(*revision);
        }
    }));
    let writers: Vec<_> = (0..8)
        .map(|writer| {
            let hub = Arc::clone(&hub);
            thread::spawn(move || {
                for change in 0..50 {
                    hold_text(&hub, &format!("{writer}-{change}"));
                }
            })
        })
        .collect();
    for writer in writers {
        writer.join().unwrap();
    }
    let revisions = revisions.lock().unwrap();
    assert_eq!(revisions.len(), 400);
    assert!(
        revisions.windows(2).all(|pair| pair[0] < pair[1]),
        "out of order: {revisions:?}"
    );
    hub.shutdown();
}

#[test]
fn a_listener_may_change_the_hub_and_its_change_follows() {
    let hub = Hub::start(default_project(), HubConfig::default()).unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let inner = Arc::downgrade(&hub);
    hub.add_listener(Box::new(move |event| {
        sink.lock().unwrap().push(event.clone());
        if *event == HubEvent::PresenceChanged(Presence::Away)
            && let Some(hub) = inner.upgrade()
        {
            hold_text(&hub, "reacted");
        }
    }));
    hub.update(|engine, now| engine.simulate_presence(Presence::Away, now));
    let seen = seen.lock().unwrap();
    let away = seen
        .iter()
        .position(|event| *event == HubEvent::PresenceChanged(Presence::Away))
        .unwrap();
    assert!(
        seen[away + 1..]
            .iter()
            .any(|event| matches!(event, HubEvent::StateChanged { .. }))
    );
    assert_eq!(
        common::element(&hub.snapshot_json().parse().unwrap(), "brb")["props"]["text"],
        "reacted"
    );
    drop(seen);
    hub.shutdown();
}
