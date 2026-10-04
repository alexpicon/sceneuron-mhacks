// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the overlay theme: its ids in project JSON, projects saved before themes existed,
//! refusing an unknown theme in an edit, loading an unknown saved theme as none, saving every
//! theme, and the theme in snapshots after a save.

use std::fs;
use std::path::PathBuf;

use sceneuron_core::{Engine, Project, Theme, default_project, load_or_default, load_project, save_project};
use serde_json::json;

/// A fresh directory under the system temp dir, unique per test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sceneuron-core-theme-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn themed(theme: Theme) -> Project {
    Project {
        theme,
        ..default_project()
    }
}

#[test]
fn every_theme_is_named_by_its_id_in_json() {
    let ids: Vec<_> = Theme::ALL.iter().map(|theme| theme.id()).collect();
    assert_eq!(
        ids,
        [
            "none",
            "graphite",
            "hacker",
            "eighties",
            "windows-95",
            "classic-mac",
            "vaporwave"
        ]
    );
    for theme in Theme::ALL {
        assert_eq!(serde_json::to_value(theme).unwrap(), json!(theme.id()));
        assert_eq!(serde_json::from_value::<Theme>(json!(theme.id())).unwrap(), theme);
        assert_eq!(Theme::from_id(theme.id()), Some(theme));
    }
    assert_eq!(Theme::from_id("Hacker"), None);
    assert_eq!(Theme::from_id("windows95"), None);
}

#[test]
fn the_default_project_has_no_theme_and_writes_its_id() {
    let project = default_project();
    assert_eq!(project.theme, Theme::None);
    assert_eq!(serde_json::to_value(&project).unwrap()["theme"], "none");
    project.validate().unwrap();
}

#[test]
fn a_project_without_a_theme_loads_with_none() {
    let mut old = serde_json::to_value(themed(Theme::Hacker)).unwrap();
    old.as_object_mut().unwrap().remove("theme");
    let project: Project = serde_json::from_value(old).unwrap();
    assert_eq!(project, default_project());
}

#[test]
fn an_edit_with_a_theme_this_build_does_not_draw_is_refused() {
    for theme in [
        json!("y2k"),
        json!("Hacker"),
        json!(""),
        json!(null),
        json!(3),
        json!({"id": "hacker"}),
    ] {
        let mut value = serde_json::to_value(default_project()).unwrap();
        value["theme"] = theme.clone();
        assert!(serde_json::from_value::<Project>(value).is_err(), "accepted {theme}");
    }
    let mut value = serde_json::to_value(default_project()).unwrap();
    value["theme"] = json!("y2k");
    let error = serde_json::from_value::<Project>(value).unwrap_err().to_string();
    assert!(
        error.contains("unknown variant `y2k`") && error.contains("`classic-mac`"),
        "{error}"
    );
}

#[test]
fn every_theme_survives_saving_and_loading() {
    let path = scratch("round-trip").join("project.json");
    for theme in Theme::ALL {
        save_project(&path, &themed(theme)).unwrap();
        assert_eq!(load_project(&path).unwrap(), Some(themed(theme)), "{}", theme.id());
    }
}

#[test]
fn a_saved_theme_this_build_does_not_draw_loads_as_none_and_keeps_the_rest() {
    let dir = scratch("unknown");
    let path = dir.join("project.json");
    let mut saved = serde_json::to_value(themed(Theme::Hacker)).unwrap();
    saved["theme"] = json!("y2k");
    saved["elements"][1]["props"]["text"] = json!("Kept");
    fs::write(&path, saved.to_string()).unwrap();

    let loaded = load_or_default(&path);
    assert_eq!(loaded.project.theme, Theme::None);
    assert_eq!(loaded.project.elements[1].props.text, "Kept");
    let warning = loaded.warning.unwrap();
    assert!(
        warning.contains(r#"the overlay theme "y2k" is not one this build draws"#),
        "{warning}"
    );
    assert!(loaded.writable);
    assert!(!dir.join("project.json.unreadable").exists());
    assert_eq!(load_project(&path).unwrap().unwrap().theme, Theme::None);
}

#[test]
fn a_version_three_file_loads_with_none_and_keeps_a_copy() {
    let dir = scratch("v3");
    let path = dir.join("project.json");
    let mut old = serde_json::to_value(default_project()).unwrap();
    old["version"] = json!(3);
    old.as_object_mut().unwrap().remove("theme");
    let old = serde_json::to_string_pretty(&old).unwrap();
    fs::write(&path, &old).unwrap();

    let loaded = load_or_default(&path);
    assert!(loaded.warning.is_none());
    assert_eq!(loaded.project, default_project());
    assert_eq!(fs::read_to_string(dir.join("project.json.v3")).unwrap(), old);
}

#[test]
fn snapshots_carry_the_saved_theme_and_leave_the_elements_alone() {
    let mut engine = Engine::new(default_project(), 0).unwrap();
    let before = engine.snapshot(0);
    assert_eq!(before.theme, Theme::None);
    assert_eq!(serde_json::to_value(&before).unwrap()["theme"], "none");

    let outcome = engine.replace_project(themed(Theme::Windows95), 1).unwrap();
    assert!(outcome.changed && outcome.project_dirty);
    let after = engine.snapshot(1);
    assert_eq!(after.theme, Theme::Windows95);
    assert_eq!(serde_json::to_value(&after).unwrap()["theme"], "windows-95");
    assert!(after.revision > before.revision);
    assert_eq!(after.elements, before.elements);
}
