// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the AI settings file: atomic saves, owner-only permissions, and defaults with a
//! warning when the file is missing, unreadable, or invalid.

mod common;

use std::fs;

use common::{enabled_settings, temp_dir};
use sceneuron_ai::{AiSettings, ProviderKind, load_settings, save_settings};

#[test]
fn saved_settings_load_back_unchanged() {
    let path = temp_dir("settings-roundtrip").join("nested").join("ai.json");
    let settings = AiSettings {
        provider: ProviderKind::OpenAiCompatible,
        base_url: "http://127.0.0.1:8000/v1".to_owned(),
        input_price_per_mtok: Some(0.25),
        ..enabled_settings()
    };
    save_settings(&path, &settings).unwrap();
    assert_eq!(load_settings(&path), (settings, None));
    let leftovers: Vec<_> = fs::read_dir(path.parent().unwrap()).unwrap().collect();
    assert_eq!(leftovers.len(), 1, "the temporary file must be renamed away");
}

#[test]
fn the_settings_file_is_readable_only_by_its_owner() {
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir("settings-mode");
    let path = dir.join("ai.json");
    // A temporary file left behind by a crash, readable by everyone.
    let stale = dir.join("ai.json.tmp");
    fs::write(&stale, "{}").unwrap();
    fs::set_permissions(&stale, fs::Permissions::from_mode(0o644)).unwrap();
    save_settings(&path, &enabled_settings()).unwrap();
    let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    assert!(!stale.exists());
}

#[test]
fn a_missing_file_gives_the_defaults_without_a_warning() {
    let path = temp_dir("settings-missing").join("ai.json");
    assert_eq!(load_settings(&path), (AiSettings::default(), None));
}

#[test]
fn an_unreadable_or_invalid_file_gives_the_defaults_with_a_warning() {
    let dir = temp_dir("settings-bad");
    let garbage = dir.join("garbage.json");
    fs::write(&garbage, "{not json").unwrap();
    let (settings, warning) = load_settings(&garbage);
    assert_eq!(settings, AiSettings::default());
    let warning = warning.unwrap();
    assert!(
        warning.contains("garbage.json") && warning.contains("using the defaults"),
        "{warning}"
    );
    let invalid = dir.join("invalid.json");
    fs::write(&invalid, r#"{"image_width": 5, "api_key": "secret-value-123"}"#).unwrap();
    let (settings, warning) = load_settings(&invalid);
    assert_eq!(settings, AiSettings::default());
    let warning = warning.unwrap();
    assert!(
        warning.contains("image_width") && !warning.contains("secret-value-123"),
        "{warning}"
    );
    let directory = dir.join("a-directory");
    fs::create_dir(&directory).unwrap();
    let (settings, warning) = load_settings(&directory);
    assert_eq!(settings, AiSettings::default());
    assert!(warning.is_some());
}
