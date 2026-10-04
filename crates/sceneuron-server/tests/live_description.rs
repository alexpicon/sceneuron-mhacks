// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the live description through the hub: frames reported as they would be by camera
//! analysis reach subscribers as the bar's text, a flickering camera publishes at most a few
//! changes a second, the background tick clears the line once frames stop, and a project without
//! the bar gets the line through throttled telemetry only.

mod common;

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use common::{drain, element, parse, wait_until};
use sceneuron_core::{DetectionSettings, Project, SEEN_CHANGE_MS, Sighting, default_project, pack_for_canvas};
use sceneuron_server::{Hub, HubConfig};

const BAR: &str = "live-description";

/// The default project with the live description bar, frames analyzed 20 times a second, and
/// presence unknown 200 ms after they stop.
fn bar_project() -> Project {
    let base = Project {
        detection: DetectionSettings {
            sample_fps: 20.0,
            stale_after_ms: 200,
            ..DetectionSettings::default()
        },
        ..default_project()
    };
    let pack = pack_for_canvas(BAR, &base.canvas).unwrap();
    base.add_pack(&pack).unwrap()
}

fn start(project: Project) -> Arc<Hub> {
    let config = HubConfig {
        tick_interval: Duration::from_millis(20),
        telemetry_interval: Duration::from_millis(100),
        ..HubConfig::default()
    };
    Hub::start(project, config).unwrap()
}

fn people(people: usize) -> Sighting {
    Sighting {
        people,
        ..Sighting::default()
    }
}

/// Reports `sighting` for `frames` frames 50 ms apart, as camera analysis would.
fn feed(hub: &Hub, sighting: &Sighting, frames: usize) {
    for _ in 0..frames {
        hub.update(|engine, now| engine.observe_sighting(sighting, now));
        thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn frames_reach_subscribers_as_the_bar_text_and_the_tick_clears_it_once_they_stop() {
    let hub = start(bar_project());
    let receiver = hub.subscribe();
    let first = parse(&receiver.recv_timeout(Duration::from_secs(1)).unwrap());
    assert_eq!(first["seen"], "");
    assert_eq!(element(&first, BAR)["props"]["visible"], false);
    feed(&hub, &people(2), 3);
    let (messages, _) = drain(&receiver);
    let shown = messages
        .iter()
        .find(|snapshot| snapshot["seen"] == "2 people")
        .expect("a snapshot with the line");
    assert_eq!(element(shown, BAR)["props"]["text"], "2 people");
    assert_eq!(element(shown, BAR)["props"]["visible"], true);
    assert!(shown["revision"].as_u64() > first["revision"].as_u64());
    let cleared = wait_until(Duration::from_secs(2), || hub.snapshot().seen.is_empty());
    assert!(cleared, "the line stayed after frames stopped");
    let last = wait_until(Duration::from_secs(1), || {
        drain(&receiver)
            .0
            .iter()
            .any(|snapshot| snapshot["seen"] == "" && element(snapshot, BAR)["props"]["visible"] == false)
    });
    assert!(last, "no snapshot hid the bar");
}

#[test]
fn a_flickering_camera_publishes_at_most_a_few_changes_a_second() {
    let hub = start(bar_project());
    let receiver = hub.subscribe();
    receiver.recv_timeout(Duration::from_secs(1)).unwrap();
    let started = Instant::now();
    // Two frames of each count in turn, so each count settles.
    for round in 0..12 {
        feed(&hub, &people(1 + round % 2), 2);
    }
    let elapsed = started.elapsed();
    let (messages, _) = drain(&receiver);
    let mut lines: Vec<&str> = messages
        .iter()
        .filter_map(|snapshot| snapshot["seen"].as_str())
        .collect();
    lines.dedup();
    let most = 1 + usize::try_from(elapsed.as_millis() / u128::from(SEEN_CHANGE_MS)).unwrap();
    assert!(lines.len() > 2, "the line followed the camera: {lines:?}");
    assert!(
        lines.len() <= most,
        "{} lines in {elapsed:?}, at most {most}: {lines:?}",
        lines.len()
    );
}

#[test]
fn without_the_bar_the_line_arrives_as_telemetry_without_a_new_revision() {
    let hub = start(Project {
        detection: bar_project().detection,
        ..default_project()
    });
    let receiver = hub.subscribe();
    let first = parse(&receiver.recv_timeout(Duration::from_secs(1)).unwrap());
    let outcome = hub.update(|engine, now| engine.observe_sighting(&people(1), now));
    assert!(!outcome.changed);
    thread::sleep(Duration::from_millis(30));
    let outcome = hub.update(|engine, now| engine.observe_sighting(&people(1), now));
    assert!(outcome.telemetry && !outcome.changed, "{outcome:?}");
    let telemetry = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
    let telemetry = parse(&telemetry);
    assert_eq!(telemetry["seen"], "1 person");
    assert_eq!(telemetry["revision"], first["revision"]);
}
