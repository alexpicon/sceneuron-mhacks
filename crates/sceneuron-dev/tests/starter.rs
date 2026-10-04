// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the starter setup in the development server: a new project file starts from the
//! starter project, and with `--ai`, new AI settings start from the starter settings, with no key
//! and no consent; a later start keeps the files as saved, and an existing project, even an
//! unreadable one, gets no starter, then or at a later start.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::voice::NO_ENV_FILE;
use common::{Running, project_path};
use sceneuron_core::{default_project, save_project};
use serde_json::{Value, json};

/// Starts the server on `project`, with the real AI and its settings in `settings`, and no key
/// from any file.
fn start(project: &Path, settings: &Path) -> Running {
    let args = [
        "--port",
        "0",
        "--project",
        project.to_str().unwrap(),
        "--ai",
        "--ai-settings",
        settings.to_str().unwrap(),
        "--env-file",
        "missing.env",
    ];
    Running::start_with_env(&args, &[NO_ENV_FILE])
}

fn paths(name: &str) -> (PathBuf, PathBuf) {
    let project = project_path(name);
    let settings = project.with_file_name("ai.json");
    (project, settings)
}

fn deck_ids(running: &Running) -> Value {
    let (status, project) = running.request("GET", "/api/project", "");
    assert_eq!(status, 200);
    project["decks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|deck| deck["id"].clone())
        .collect()
}

#[test]
fn a_new_project_starts_from_the_starter_setup_with_starter_ai_settings() {
    let (project, settings) = paths("starter-fresh");
    let mut running = start(&project, &settings);
    let line = running.line_starting("Project:");
    assert!(line.ends_with("(new: the starter setup)"), "{line}");
    assert_eq!(deck_ids(&running), json!(["poker", "meeting", "classroom", "status"]));
    let (status, ai) = running.request("GET", "/api/ai", "");
    assert_eq!(status, 200);
    assert_eq!(
        (&ai["provider"], &ai["consent"], &ai["enabled"]),
        (&json!("gemini"), &json!(false), &json!(false))
    );
    assert_eq!(
        (&ai["voice"]["mode"], &ai["voice"]["consent"]),
        (&json!("always_on"), &json!(false))
    );
    assert_eq!(
        (&ai["subtitles"]["targets"], &ai["subtitles"]["consent"]),
        (&json!(["ja"]), &json!(false))
    );
    assert!(settings.exists());
    assert!(running.quit_with_q().success());
}

#[test]
fn a_later_start_loads_the_saved_files_as_they_are() {
    let (project, settings) = paths("starter-later");
    let mut first = start(&project, &settings);
    let (status, _) = first.request("POST", "/api/automation", r#"{"enabled":false}"#);
    assert_eq!(status, 200);
    let (status, _) = first.request("PUT", "/api/ai", r#"{"image_width": 512}"#);
    assert_eq!(status, 200);
    assert!(first.quit_with_q().success());
    let saved = fs::read(&settings).unwrap();

    let mut again = start(&project, &settings);
    let line = again.line_starting("Project:");
    assert!(line.ends_with("project.json"), "{line}");
    let (_, state) = again.request("GET", "/api/state", "");
    assert_eq!(state["automation_enabled"], false);
    let (_, ai) = again.request("GET", "/api/ai", "");
    assert_eq!(ai["image_width"], 512);
    assert!(again.quit_with_q().success());
    assert_eq!(fs::read(&settings).unwrap(), saved);
}

#[test]
fn an_existing_project_gets_no_starter_ai_settings() {
    let (project, settings) = paths("starter-existing");
    save_project(&project, &default_project()).unwrap();
    let saved = fs::read(&project).unwrap();
    let mut running = start(&project, &settings);
    assert_eq!(deck_ids(&running), json!([]));
    let (_, ai) = running.request("GET", "/api/ai", "");
    assert_eq!(ai["voice"]["mode"], "push_to_talk");
    assert!(!settings.exists());
    assert!(running.quit_with_q().success());
    assert_eq!(fs::read(&project).unwrap(), saved);
}

#[test]
fn an_unreadable_project_gets_no_starter_then_or_at_a_later_start() {
    let (project, settings) = paths("starter-unreadable");
    fs::write(&project, "{ not json").unwrap();
    for run in 1..3 {
        let mut running = start(&project, &settings);
        let line = running.line_starting("Project:");
        assert!(line.ends_with("project.json"), "start {run}: {line}");
        assert_eq!(deck_ids(&running), json!([]), "start {run}");
        assert!(running.quit_with_q().success());
        assert!(!settings.exists(), "start {run}");
    }
}
