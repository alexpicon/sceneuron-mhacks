// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the plugin's first start: a fresh install loads the starter project and gets the
//! starter AI settings, a later start that still has no project keeps the settings it got, and an
//! install with a project file, even an unreadable one set aside at an earlier start, or with AI
//! settings of its own, keeps them.

use std::fs;
use std::path::PathBuf;

use sceneuron::config::Paths;
use sceneuron::starter::load_project;
use sceneuron_ai::{AiSettings, load_settings};
use sceneuron_core::{default_project, save_project, starter_project};

fn scratch(name: &str) -> Paths {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("obs-starter-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    Paths::in_dir(&dir)
}

#[test]
fn a_fresh_install_starts_from_the_starter_project_and_ai_settings() {
    let paths = scratch("fresh");
    let loaded = load_project(&paths);
    assert!(loaded.fresh);
    assert!(loaded.warning.is_none());
    assert_eq!(loaded.project, starter_project().unwrap());
    let (settings, warning) = load_settings(&paths.ai);
    assert_eq!(warning, None);
    assert!(settings.subtitles.targets == ["ja"] && !settings.voice.consent);
    assert!(!paths.project.exists(), "the project is saved only once it changes");
}

#[test]
fn a_second_start_without_a_saved_project_keeps_the_ai_settings_it_has() {
    let paths = scratch("again");
    load_project(&paths);
    let changed = r#"{"provider": "gemini", "image_width": 512}"#;
    fs::write(&paths.ai, changed).unwrap();
    assert!(load_project(&paths).fresh);
    assert_eq!(fs::read_to_string(&paths.ai).unwrap(), changed);
}

#[test]
fn an_install_with_a_project_keeps_its_ai_defaults_and_gets_no_starter() {
    let paths = scratch("existing");
    save_project(&paths.project, &default_project()).unwrap();
    let loaded = load_project(&paths);
    assert!(!loaded.fresh);
    assert_eq!(loaded.project, default_project());
    assert!(!paths.ai.exists());
    assert_eq!(load_settings(&paths.ai), (AiSettings::default(), None));
}

#[test]
fn an_unreadable_project_is_not_a_fresh_install_then_or_at_a_later_start() {
    let paths = scratch("unreadable");
    fs::write(&paths.project, "{ not json").unwrap();
    for run in 1..4 {
        let loaded = load_project(&paths);
        assert!(!loaded.fresh, "start {run}");
        assert_eq!(loaded.project, default_project(), "start {run}");
        assert!(!paths.ai.exists(), "start {run}");
    }
}

#[test]
fn ai_settings_already_there_are_never_changed() {
    let paths = scratch("own-ai");
    let own = r#"{"provider": "openai-compatible", "base_url": "http://127.0.0.1:11434/v1"}"#;
    fs::write(&paths.ai, own).unwrap();
    let loaded = load_project(&paths);
    assert!(loaded.fresh && loaded.warning.is_none());
    assert_eq!(fs::read_to_string(&paths.ai).unwrap(), own);
}

#[test]
fn starter_ai_settings_that_cannot_be_written_are_a_warning() {
    let paths = scratch("blocked");
    // A file where the settings' folder would be makes writing them fail.
    let blocked = Paths {
        ai: paths.config.join("ai.json"),
        ..paths
    };
    fs::write(&blocked.config, "{}").unwrap();
    let loaded = load_project(&blocked);
    assert!(loaded.fresh);
    assert_eq!(loaded.project, starter_project().unwrap());
    assert!(
        loaded
            .warning
            .unwrap()
            .contains("the starter AI settings could not be written")
    );
}
