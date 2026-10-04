// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises how the hub saves the project: after the debounce, never for live-only
//! changes, on flush, on shutdown, immediately after shutdown, not at all without a
//! project path, and failures that reach listeners and the event log once per streak.

mod common;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use common::{drain, hold_text, temp_dir, wait_until};
use sceneuron_core::{Project, default_project, load_project};
use sceneuron_server::{Hub, HubConfig, HubEvent};

fn saving_hub(debounce: Duration) -> (Arc<Hub>, PathBuf) {
    let path = temp_dir("hub-save").join("project.json");
    let config = HubConfig {
        project_path: Some(path.clone()),
        save_debounce: debounce,
        tick_interval: Duration::from_millis(20),
        ..HubConfig::default()
    };
    (Hub::start(default_project(), config).unwrap(), path)
}

fn saved(path: &Path) -> Option<Project> {
    load_project(path).ok().flatten()
}

#[test]
fn the_project_is_saved_after_the_debounce() {
    let (hub, path) = saving_hub(Duration::from_millis(300));
    hub.update(|engine, now| engine.set_automation(false, now));
    assert!(!path.exists(), "saved before the debounce passed");
    assert!(wait_until(Duration::from_secs(3), || path.exists()));
    assert!(!saved(&path).unwrap().automation_enabled);
}

#[test]
fn each_change_restarts_the_debounce() {
    let (hub, path) = saving_hub(Duration::from_millis(1_500));
    hub.update(|engine, now| engine.set_automation(false, now));
    thread::sleep(Duration::from_millis(750));
    hub.update(|engine, now| engine.set_automation(true, now));
    thread::sleep(Duration::from_millis(1_100));
    assert!(!path.exists(), "saved before the debounce of the last change passed");
    assert!(wait_until(Duration::from_secs(3), || path.exists()));
    assert!(saved(&path).unwrap().automation_enabled);
}

#[test]
fn telemetry_and_overrides_do_not_save() {
    let (hub, path) = saving_hub(Duration::from_millis(10));
    hold_text(&hub, "held");
    hub.update(|engine, now| engine.observe(0.2, now));
    thread::sleep(Duration::from_millis(200));
    assert!(!path.exists());
}

#[test]
fn flush_saves_immediately() {
    let (hub, path) = saving_hub(Duration::from_secs(60));
    hub.update(|engine, now| engine.set_automation(false, now));
    assert_eq!(hub.flush(), Ok(()));
    assert!(!saved(&path).unwrap().automation_enabled);
}

#[test]
fn shutdown_saves_and_disconnects_subscribers() {
    let (hub, path) = saving_hub(Duration::from_secs(60));
    let receiver = hub.subscribe();
    hub.update(|engine, now| engine.set_automation(false, now));
    hub.shutdown();
    assert!(!saved(&path).unwrap().automation_enabled);
    let (messages, disconnected) = drain(&receiver);
    assert_eq!(messages.len(), 2);
    assert!(disconnected);
    hub.shutdown();
}

#[test]
fn changes_after_shutdown_are_saved_immediately() {
    let (hub, path) = saving_hub(Duration::from_secs(60));
    hub.shutdown();
    hub.update(|engine, now| engine.set_automation(false, now));
    assert!(!saved(&path).unwrap().automation_enabled);
}

#[test]
fn flush_without_a_project_path_does_nothing() {
    let hub = Hub::start(default_project(), HubConfig::default()).unwrap();
    hub.update(|engine, now| engine.set_automation(false, now));
    assert_eq!(hub.flush(), Ok(()));
    hub.shutdown();
    assert!(!hub.snapshot().automation_enabled);
}

/// A hub whose project path cannot be written: its parent is a file.
fn failing_hub() -> (Arc<Hub>, Arc<Mutex<Vec<String>>>) {
    let blocker = temp_dir("hub-save-fail").join("not-a-directory");
    std::fs::write(&blocker, "file").unwrap();
    let config = HubConfig {
        project_path: Some(blocker.join("project.json")),
        save_debounce: Duration::from_secs(60),
        ..HubConfig::default()
    };
    let hub = Hub::start(default_project(), config).unwrap();
    let failures = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&failures);
    hub.add_listener(Box::new(move |event| {
        if let HubEvent::SaveFailed(message) = event {
            seen.lock().unwrap().push(message.clone());
        }
    }));
    (hub, failures)
}

#[test]
fn a_failing_save_is_reported_once_per_streak_and_logged_for_the_editor() {
    let (hub, failures) = failing_hub();
    hub.update(|engine, now| engine.set_automation(false, now));
    assert!(hub.flush().unwrap_err().contains("could not save the project"));
    hub.update(|engine, now| engine.set_automation(true, now));
    assert!(hub.flush().is_err());
    assert_eq!(failures.lock().unwrap().len(), 1);
    let events = hub.snapshot().events;
    assert!(
        events
            .iter()
            .any(|e| e.source == "system" && e.detail.contains("could not save the project"))
    );
}

#[test]
fn a_failing_final_save_tells_listeners_the_changes_are_lost() {
    let (hub, failures) = failing_hub();
    hub.update(|engine, now| engine.set_automation(false, now));
    hub.shutdown();
    let failures = failures.lock().unwrap();
    assert_eq!(failures.len(), 2, "{failures:?}");
    assert!(failures[1].ends_with("the last changes are lost"));
}
