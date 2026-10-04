// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the subtitle settings for how the overlay shows subtitles: the language labels, the
//! targets kept in Latin letters rather than a script of their own, and the Latin line under a
//! script; their defaults in older files, partial updates, a removed target taking its choice with
//! it, and the status reported to the engine.

mod common;

use common::temp_dir;
use sceneuron_ai::{AiSettings, AiSettingsUpdate, SubtitleSettings, load_settings};
use sceneuron_voice::VoiceStatus as InputStatus;
use serde_json::json;

fn updated(settings: &AiSettings, value: serde_json::Value) -> AiSettings {
    let update: AiSettingsUpdate = serde_json::from_value(json!({ "subtitles": value })).unwrap();
    settings.updated(&update).unwrap()
}

#[test]
fn labels_and_scripts_are_on_and_latin_lines_off_until_changed() {
    let settings = SubtitleSettings::default();
    assert!(settings.labels);
    assert!(settings.latin_targets.is_empty() && !settings.latin_lines);
    let path = temp_dir("subtitle-overlay").join("ai.json");
    std::fs::write(&path, r#"{"subtitles": {"targets": ["qya"], "lines": 3}}"#).unwrap();
    let (loaded, warning) = load_settings(&path);
    assert_eq!(warning, None);
    assert_eq!(loaded.subtitles.targets, ["qya"]);
    assert!(loaded.subtitles.labels, "an older file shows labels");
    assert!(
        loaded.subtitles.latin_targets.is_empty(),
        "and writes Quenya in Tengwar"
    );
}

#[test]
fn updates_change_each_setting_and_keep_target_codes_in_their_usual_case() {
    let base = updated(&AiSettings::default(), json!({"targets": ["qya", "tlh", "ja"]}));
    let next = updated(
        &base,
        json!({"labels": false, "latin_targets": [" TLH "], "latin_lines": true}),
    );
    assert!(!next.subtitles.labels && next.subtitles.latin_lines);
    assert_eq!(next.subtitles.latin_targets, ["tlh"]);
    let unchanged = updated(&next, json!({"lines": 1}));
    assert_eq!(
        (
            unchanged.subtitles.labels,
            unchanged.subtitles.latin_targets.clone(),
            unchanged.subtitles.latin_lines
        ),
        (false, vec!["tlh".to_owned()], true),
        "settings an update leaves out keep their values"
    );
    let back = updated(&unchanged, json!({"labels": true, "latin_targets": []}));
    assert!(back.subtitles.labels && back.subtitles.latin_targets.is_empty());
}

#[test]
fn a_removed_target_takes_its_latin_letters_with_it() {
    let base = updated(
        &AiSettings::default(),
        json!({"targets": ["qya", "tok"], "latin_targets": ["tok", "qya"]}),
    );
    assert_eq!(base.subtitles.latin_targets, ["tok", "qya"]);
    let next = updated(&base, json!({"targets": ["qya"]}));
    assert_eq!(next.subtitles.latin_targets, ["qya"]);
    let readded = updated(&next, json!({"targets": ["qya", "tok"]}));
    assert_eq!(
        readded.subtitles.latin_targets,
        ["qya"],
        "tok comes back in sitelen pona"
    );
    let unknown = updated(&base, json!({"latin_targets": ["sjn"]}));
    assert!(
        unknown.subtitles.latin_targets.is_empty(),
        "only a target can be kept in Latin letters"
    );
}

#[test]
fn the_status_carries_how_the_overlay_shows_subtitles() {
    let settings = SubtitleSettings {
        enabled: true,
        consent: true,
        targets: vec!["qya".to_owned(), "tlh".to_owned()],
        labels: false,
        latin_targets: vec!["tlh".to_owned()],
        latin_lines: true,
        ..SubtitleSettings::default()
    };
    let status = settings.status(true, &InputStatus::Listening, false, "");
    assert!(!status.labels && status.latin_lines);
    assert_eq!(status.latin_targets, ["tlh"]);
    assert_eq!(status.detected, "", "the language detected is the voice input's to add");
    let shown = SubtitleSettings::default().status(true, &InputStatus::Listening, false, "en");
    assert!(shown.labels && !shown.latin_lines && shown.latin_targets.is_empty());
}
