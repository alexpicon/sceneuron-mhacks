// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises project persistence: round trips, missing files, unreadable files,
//! files from newer versions, and atomic replacement.

use std::fs;
use std::path::PathBuf;

use sceneuron_core::{
    Canvas, PersistError, default_project, load_or_default, load_project, pack_for_canvas, save_project,
};

/// A fresh directory under the system temp dir, unique per test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sceneuron-core-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn round_trips_a_project() {
    let path = scratch("roundtrip").join("nested/project.json");
    let mut project = default_project();
    project.automation_enabled = false;
    project.elements[1].props.text = "Saved text".into();
    save_project(&path, &project).unwrap();
    assert_eq!(load_project(&path).unwrap(), Some(project));
    assert!(!path.with_file_name("project.json.tmp").exists());
}

#[test]
fn missing_file_loads_as_none_and_defaults() {
    let path = scratch("missing").join("project.json");
    assert!(load_project(&path).unwrap().is_none());
    let loaded = load_or_default(&path);
    assert_eq!(loaded.project, default_project());
    assert!(loaded.warning.is_none());
}

#[test]
fn unreadable_file_is_kept_aside_and_defaults_load() {
    let dir = scratch("corrupt");
    let path = dir.join("project.json");
    fs::write(&path, "{ not json").unwrap();
    assert!(matches!(load_project(&path), Err(PersistError::Parse(_))));
    let loaded = load_or_default(&path);
    assert_eq!(loaded.project, default_project());
    assert!(loaded.warning.unwrap().contains("unreadable"));
    assert_eq!(
        fs::read_to_string(dir.join("project.json.unreadable")).unwrap(),
        "{ not json"
    );
}

#[test]
fn invalid_project_is_rejected() {
    let path = scratch("invalid").join("project.json");
    let mut project = default_project();
    project.canvas.width = 1;
    fs::write(&path, serde_json::to_string(&project).unwrap()).unwrap();
    assert!(matches!(load_project(&path), Err(PersistError::Invalid(_))));
}

#[test]
fn newer_version_is_rejected_not_downgraded() {
    let path = scratch("newer").join("project.json");
    fs::write(&path, r#"{"version": 99}"#).unwrap();
    assert!(matches!(load_project(&path), Err(PersistError::NewerVersion(99))));
}

#[test]
fn file_without_version_loads_as_current() {
    let path = scratch("unversioned").join("project.json");
    fs::write(&path, r#"{"elements": []}"#).unwrap();
    assert!(load_project(&path).unwrap().unwrap().elements.is_empty());
}

#[test]
fn saving_replaces_existing_content() {
    let path = scratch("replace").join("project.json");
    save_project(&path, &default_project()).unwrap();
    let mut project = default_project();
    project.rules.clear();
    save_project(&path, &project).unwrap();
    assert!(load_project(&path).unwrap().unwrap().rules.is_empty());
}

#[test]
fn version_one_file_upgrades_and_keeps_a_copy() {
    let dir = scratch("upgrade");
    let path = dir.join("project.json");
    let old = r#"{"version": 1, "elements": [], "rules": [], "automation_enabled": false}"#;
    fs::write(&path, old).unwrap();
    let loaded = load_or_default(&path);
    assert!(loaded.warning.is_none());
    assert_eq!(loaded.project.version, sceneuron_core::SCHEMA_VERSION);
    assert!(loaded.project.conditions.is_empty() && !loaded.project.automation_enabled);
    assert_eq!(fs::read_to_string(dir.join("project.json.v1")).unwrap(), old);
    save_project(&path, &loaded.project).unwrap();
    load_or_default(&path);
    assert_eq!(fs::read_to_string(dir.join("project.json.v1")).unwrap(), old);
}

#[test]
fn version_two_file_upgrades_with_no_decks_and_keeps_a_copy() {
    let dir = scratch("upgrade-v2");
    let path = dir.join("project.json");
    let mut old = serde_json::to_value(default_project()).unwrap();
    old["version"] = serde_json::json!(2);
    old.as_object_mut().unwrap().remove("decks");
    let old = serde_json::to_string_pretty(&old).unwrap();
    fs::write(&path, &old).unwrap();
    let loaded = load_or_default(&path);
    assert!(loaded.warning.is_none());
    assert_eq!(loaded.project.version, sceneuron_core::SCHEMA_VERSION);
    assert!(loaded.project.decks.is_empty());
    assert_eq!(loaded.project.elements, default_project().elements);
    assert_eq!(fs::read_to_string(dir.join("project.json.v2")).unwrap(), old);
}

#[test]
fn decks_round_trip() {
    let path = scratch("decks").join("project.json");
    let pack = pack_for_canvas("meeting", &Canvas::default()).unwrap();
    let project = default_project().add_pack(&pack).unwrap();
    save_project(&path, &project).unwrap();
    let loaded = load_project(&path).unwrap().unwrap();
    assert_eq!(loaded.decks, pack.decks);
    assert_eq!(loaded, project);
    assert!(!path.with_file_name("project.json.v4").exists());
}

#[test]
fn version_three_file_upgrades_with_voice_off_and_keeps_a_copy() {
    let dir = scratch("upgrade-v3");
    let path = dir.join("project.json");
    let pack = pack_for_canvas("meeting", &Canvas::default()).unwrap();
    let mut old = serde_json::to_value(default_project().add_pack(&pack).unwrap()).unwrap();
    old["version"] = serde_json::json!(3);
    for deck in old["decks"].as_array_mut().unwrap() {
        let deck = deck.as_object_mut().unwrap();
        deck.remove("voice");
        deck.remove("speak");
        for card in deck["cards"].as_array_mut().unwrap() {
            card.as_object_mut().unwrap().remove("phrases");
        }
    }
    let old = serde_json::to_string_pretty(&old).unwrap();
    fs::write(&path, &old).unwrap();
    let loaded = load_or_default(&path);
    assert!(loaded.warning.is_none());
    assert_eq!(loaded.project.version, 4);
    let deck = &loaded.project.decks[0];
    assert!(!deck.voice && !deck.speak);
    assert!(deck.cards.iter().all(|card| card.phrases.is_empty()));
    assert_eq!(fs::read_to_string(dir.join("project.json.v3")).unwrap(), old);
}

#[test]
fn repeated_unreadable_files_are_all_kept() {
    let dir = scratch("unreadable-twice");
    let path = dir.join("project.json");
    fs::write(&path, "first").unwrap();
    load_or_default(&path);
    fs::write(&path, "second").unwrap();
    let loaded = load_or_default(&path);
    assert!(loaded.warning.unwrap().contains("project.json.unreadable.2"));
    assert_eq!(
        fs::read_to_string(dir.join("project.json.unreadable")).unwrap(),
        "first"
    );
    assert_eq!(
        fs::read_to_string(dir.join("project.json.unreadable.2")).unwrap(),
        "second"
    );
}

#[test]
fn a_file_that_cannot_be_set_aside_is_never_saved_over() {
    let dir = scratch("stuck");
    let path = dir.join("project.json");
    fs::write(&path, "{ not json").unwrap();
    // With every backup name taken, the first one is reused, and a folder there makes the rename
    // fail for every user; a read-only folder would not stop root.
    fs::create_dir(dir.join("project.json.unreadable")).unwrap();
    for n in 2..1_000 {
        fs::write(dir.join(format!("project.json.unreadable.{n}")), "").unwrap();
    }
    let loaded = load_or_default(&path);
    assert!(!loaded.writable);
    assert!(loaded.warning.unwrap().contains("could not be set aside"));
    assert_eq!(fs::read_to_string(&path).unwrap(), "{ not json");
}

#[test]
fn readable_and_missing_files_are_writable() {
    let path = scratch("writable").join("project.json");
    assert!(load_or_default(&path).writable);
    save_project(&path, &default_project()).unwrap();
    assert!(load_or_default(&path).writable);
}
