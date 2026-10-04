// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the hub: snapshot publishing to subscribers, listener events, presence
//! timeouts from the background tick, telemetry throttling, and what shutdown does to
//! subscribers and listeners.

mod common;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use common::{drain, hold_text, parse, wait_until};
use sceneuron_core::{DetectionSettings, EngineError, Presence, Project, default_project};
use sceneuron_server::{Hub, HubConfig, HubEvent, Server};

fn hub() -> Arc<Hub> {
    Hub::start(default_project(), HubConfig::default()).unwrap()
}

fn record_events(hub: &Hub) -> Arc<Mutex<Vec<HubEvent>>> {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);
    hub.add_listener(Box::new(move |event| sink.lock().unwrap().push(event.clone())));
    events
}

fn quick_detection() -> Project {
    Project {
        detection: DetectionSettings {
            present_after_ms: 0,
            stale_after_ms: 100,
            ..DetectionSettings::default()
        },
        ..default_project()
    }
}

#[test]
fn start_rejects_an_invalid_project() {
    let project = Project {
        version: 99,
        ..default_project()
    };
    assert!(Hub::start(project, HubConfig::default()).is_err());
}

#[test]
fn clock_is_monotonic_and_starts_near_zero() {
    let hub = hub();
    let first = hub.now_ms();
    assert!(first < 1_000);
    thread::sleep(Duration::from_millis(20));
    assert!(hub.now_ms() >= first + 20);
}

#[test]
fn subscribers_start_with_the_current_snapshot_and_receive_changes() {
    let hub = hub();
    let receiver = hub.subscribe();
    let first = parse(&receiver.recv_timeout(Duration::from_secs(1)).unwrap());
    assert_eq!(first["revision"], hub.snapshot().revision);
    hold_text(&hub, "back soon");
    let next = parse(&receiver.recv_timeout(Duration::from_secs(1)).unwrap());
    assert!(next["revision"].as_u64() > first["revision"].as_u64());
    let brb = common::element(&next, "brb");
    assert_eq!(brb["props"]["text"], "back soon");
    assert_eq!(parse(&hub.snapshot_json())["revision"], next["revision"]);
}

#[test]
fn refused_and_unchanged_updates_publish_nothing() {
    let hub = hub();
    let receiver = hub.subscribe();
    receiver.recv_timeout(Duration::from_secs(1)).unwrap();
    let refused = hub.try_update(|engine, now| engine.trigger_effect("missing", sceneuron_core::Effect::Pop, now));
    assert_eq!(refused, Err(EngineError::UnknownElement("missing".to_owned())));
    let unchanged = hub
        .try_update(|engine, now| engine.release_overrides(None, now))
        .unwrap();
    assert!(!unchanged.changed);
    thread::sleep(Duration::from_millis(100));
    assert!(drain(&receiver).0.is_empty());
}

#[test]
fn read_sees_the_engine() {
    let hub = hub();
    let count = hub.read(|engine| engine.project().elements.len());
    assert_eq!(count, default_project().elements.len());
}

#[test]
fn listeners_hear_state_presence_and_project_changes() {
    let hub = hub();
    let events = record_events(&hub);
    hub.update(|engine, now| engine.simulate_presence(Presence::Away, now));
    hub.update(|engine, now| engine.set_automation(false, now));
    let revision = hub.snapshot().revision;
    let events = events.lock().unwrap().clone();
    assert_eq!(
        events,
        vec![
            HubEvent::StateChanged { revision: revision - 1 },
            HubEvent::PresenceChanged(Presence::Away),
            HubEvent::StateChanged { revision },
            HubEvent::ProjectChanged,
        ]
    );
}

#[test]
fn a_panicking_listener_does_not_break_the_hub_or_other_listeners() {
    let hub = hub();
    hub.add_listener(Box::new(|_| panic!("listener bug")));
    let events = record_events(&hub);
    hub.update(|engine, now| engine.simulate_presence(Presence::Present, now));
    assert!(
        events
            .lock()
            .unwrap()
            .contains(&HubEvent::PresenceChanged(Presence::Present))
    );
    hold_text(&hub, "still works");
    assert_eq!(events.lock().unwrap().len(), 3);
}

#[test]
fn a_stalled_subscriber_is_dropped_and_others_continue() {
    let hub = hub();
    let stalled = hub.subscribe();
    for index in 0..100 {
        hold_text(&hub, &format!("change {index}"));
    }
    let (messages, disconnected) = drain(&stalled);
    assert!(disconnected, "a full channel must drop the subscriber");
    assert!(messages.len() < 100, "got {} messages", messages.len());
    let fresh = hub.subscribe();
    let first = parse(&fresh.recv_timeout(Duration::from_secs(1)).unwrap());
    assert_eq!(common::element(&first, "brb")["props"]["text"], "change 99");
    hold_text(&hub, "after");
    assert!(fresh.recv_timeout(Duration::from_secs(1)).is_ok());
}

#[test]
fn the_tick_turns_stale_presence_unknown() {
    let config = HubConfig {
        tick_interval: Duration::from_millis(20),
        ..HubConfig::default()
    };
    let hub = Hub::start(quick_detection(), config).unwrap();
    let events = record_events(&hub);
    hub.update(|engine, now| engine.observe(0.95, now));
    assert_eq!(hub.read(|engine| engine.presence()), Presence::Present);
    let unknown = HubEvent::PresenceChanged(Presence::Unknown);
    assert!(wait_until(Duration::from_secs(3), || events
        .lock()
        .unwrap()
        .contains(&unknown)));
    assert_eq!(hub.read(|engine| engine.presence()), Presence::Unknown);
    let presence = hub.snapshot().presence;
    assert_eq!(presence.state, Presence::Unknown);
}

#[test]
fn telemetry_snapshots_are_throttled_and_carry_the_latest_score() {
    let config = HubConfig {
        telemetry_interval: Duration::from_millis(200),
        tick_interval: Duration::from_secs(10),
        ..HubConfig::default()
    };
    let hub = Hub::start(default_project(), config).unwrap();
    let receiver = hub.subscribe();
    receiver.recv_timeout(Duration::from_secs(1)).unwrap();
    let mut last_score = 0.0;
    let started = Instant::now();
    for index in 0..40 {
        last_score = 0.01 * f64::from(index);
        let outcome = hub.update(|engine, now| engine.observe(last_score, now));
        assert!(outcome.telemetry && !outcome.changed);
        thread::sleep(Duration::from_millis(10));
    }
    let observing = started.elapsed();
    thread::sleep(Duration::from_millis(700));
    let (messages, _) = drain(&receiver);
    assert!(!messages.is_empty(), "telemetry was never published");
    // The first change goes out at once, then one per interval, then the last pending one.
    let allowed = 2 + usize::try_from(observing.as_millis() / 200).unwrap();
    assert!(
        messages.len() <= allowed.min(20),
        "{} telemetry snapshots for 40 observations over {observing:?}",
        messages.len()
    );
    let latest = messages.last().unwrap();
    assert_eq!(latest["presence"]["last_score"].as_f64(), Some(last_score));
}

#[test]
fn subscribing_after_shutdown_yields_one_snapshot() {
    let hub = hub();
    hub.shutdown();
    let receiver = hub.subscribe();
    let (messages, disconnected) = drain(&receiver);
    assert_eq!(messages.len(), 1);
    assert!(disconnected);
}

#[test]
fn shutdown_drops_listeners() {
    let hub = hub();
    let events = record_events(&hub);
    hub.shutdown();
    hold_text(&hub, "late");
    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn hub_and_server_can_be_shared_across_threads() {
    fn shared<T: Send + Sync>() {}
    shared::<Hub>();
    shared::<Server>();
}
