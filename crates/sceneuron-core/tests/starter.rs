// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the starter project: it loads through the same reading and checks as a saved project
//! file, keeps every value it sets, holds the decks, switches, and rules a fresh install should
//! find, and is used only while no project file exists, never in place of a saved one or after one
//! was set aside as unreadable.

use std::fs;
use std::path::PathBuf;

use sceneuron_core::{
    Engine, SCHEMA_VERSION, Theme, default_project, load_or_starter, load_project, save_project, starter_project,
};
use serde_json::Value;

const STARTER: &str = include_str!("../starter/project.json");

/// A fresh directory under Cargo's temporary directory for integration tests, unique per test.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("core-starter-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Whether every value `part` sets is in `whole`, so loading dropped or changed none of them.
fn kept(part: &Value, whole: &Value, path: &str) -> Result<(), String> {
    match (part, whole) {
        (Value::Object(fields), Value::Object(all)) => fields.iter().try_for_each(|(name, value)| {
            let here = format!("{path}.{name}");
            all.get(name)
                .map_or(Err(format!("{here} was dropped")), |found| kept(value, found, &here))
        }),
        (Value::Array(items), Value::Array(all)) if items.len() == all.len() => items
            .iter()
            .zip(all)
            .enumerate()
            .try_for_each(|(index, (item, found))| kept(item, found, &format!("{path}[{index}]"))),
        _ if part == whole => Ok(()),
        _ => Err(format!("{path} is {whole}, not {part}")),
    }
}

#[test]
fn the_starter_loads_like_a_saved_project_file() {
    let project = starter_project().unwrap();
    let path = scratch("saved").join("project.json");
    fs::write(&path, STARTER).unwrap();
    assert_eq!(load_project(&path).unwrap(), Some(project.clone()));
    assert!(Engine::new(project, 0).is_ok());
}

#[test]
fn every_value_the_starter_sets_survives_loading() {
    let file: Value = serde_json::from_str(STARTER).unwrap();
    assert_eq!(file["version"], SCHEMA_VERSION, "the starter needs no upgrade");
    let loaded = serde_json::to_value(starter_project().unwrap()).unwrap();
    kept(&file, &loaded, "$").unwrap();
}

#[test]
fn the_starter_holds_the_decks_switches_and_rules_of_a_set_up_install() {
    let project = starter_project().unwrap();
    let switches: Vec<(&str, bool, bool, bool)> = project
        .decks
        .iter()
        .map(|deck| (deck.id.as_str(), deck.camera, deck.voice, deck.speak))
        .collect();
    assert_eq!(
        switches,
        [
            ("poker", true, true, true),
            ("meeting", true, true, true),
            ("classroom", false, false, false),
            ("status", false, false, false),
        ]
    );
    let elements: Vec<&str> = project.elements.iter().map(|element| element.id.as_str()).collect();
    assert_eq!(
        elements,
        [
            "brb",
            "name-label",
            "cam-frame",
            "live-description",
            "now-showing",
            "on-air",
            "on-air-time",
            "pet-alert",
            "poker-card",
            "meeting-card",
            "classroom-card",
            "status-card",
            "subtitles-spoken",
            "subtitles-translated",
        ]
    );
    let base = default_project();
    assert_eq!(project.rules, base.rules);
    assert_eq!(project.conditions, base.conditions);
    assert_eq!((project.theme, project.reactions.enabled), (Theme::Graphite, true));
    assert_eq!((&project.detection, &project.ai), (&base.detection, &base.ai));
    assert!(project.automation_enabled);
    let label = project.element("name-label").unwrap();
    assert_eq!(label.props.text, "Sceneuron", "the name label shows no person's name");
}

#[test]
fn a_fresh_install_starts_from_the_starter_and_writes_nothing() {
    let path = scratch("fresh").join("project.json");
    let loaded = load_or_starter(&path);
    assert_eq!(loaded.project, starter_project().unwrap());
    assert!(loaded.fresh && loaded.writable);
    assert!(loaded.warning.is_none());
    assert!(!path.exists());
}

#[test]
fn a_saved_project_loads_as_saved_and_its_file_never_changes() {
    let path = scratch("saved-default").join("project.json");
    let mut project = default_project();
    project.elements[1].props.text = "Saved name".into();
    save_project(&path, &project).unwrap();
    let before = fs::read(&path).unwrap();
    let loaded = load_or_starter(&path);
    assert_eq!(loaded.project, project);
    assert!(!loaded.fresh);
    assert!(loaded.warning.is_none());
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn an_unreadable_project_is_set_aside_for_the_default_project_not_the_starter() {
    let dir = scratch("unreadable");
    let path = dir.join("project.json");
    fs::write(&path, "{ not json").unwrap();
    let loaded = load_or_starter(&path);
    assert_eq!(loaded.project, default_project());
    assert!(!loaded.fresh);
    assert!(loaded.warning.unwrap().contains("unreadable"));
    assert_eq!(
        fs::read_to_string(dir.join("project.json.unreadable")).unwrap(),
        "{ not json"
    );
}

#[test]
fn after_an_unreadable_project_was_set_aside_later_starts_are_not_fresh_either() {
    let dir = scratch("unreadable-again");
    let path = dir.join("project.json");
    fs::write(&path, "{ not json").unwrap();
    load_or_starter(&path);
    for run in 2..4 {
        let loaded = load_or_starter(&path);
        assert_eq!(loaded.project, default_project(), "start {run}");
        assert!(!loaded.fresh && loaded.writable, "start {run}");
        assert!(loaded.warning.is_none(), "start {run}");
        assert!(!path.exists(), "start {run}");
    }
}
