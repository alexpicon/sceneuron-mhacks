// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the subtitle settings: their defaults and JSON shape in `ai.json`, every validation
//! rule, partial updates with codes in their usual case and consent, the editor's view with the
//! translation model's prices, and the status reported to the engine.

mod common;

use common::temp_dir;
use sceneuron_ai::{
    AiSettings, AiSettingsUpdate, DEFAULT_TRANSLATION_MODEL, ProviderKind, SubtitleSettings, load_settings,
    save_settings,
};
use sceneuron_voice::VoiceStatus as InputStatus;
use serde_json::json;

fn update(value: serde_json::Value) -> AiSettingsUpdate {
    serde_json::from_value(value).unwrap()
}

fn refusal(edit: impl FnOnce(&mut SubtitleSettings)) -> String {
    let mut settings = AiSettings::default();
    edit(&mut settings.subtitles);
    settings.validate().unwrap_err().to_string()
}

fn subtitles(value: serde_json::Value) -> SubtitleSettings {
    AiSettings::default()
        .updated(&update(json!({ "subtitles": value })))
        .unwrap()
        .subtitles
}

#[test]
fn subtitles_start_off_with_documented_defaults() {
    let json = serde_json::to_value(SubtitleSettings::default()).unwrap();
    assert_eq!(
        json,
        json!({"enabled": false, "consent": false, "language": "", "targets": [],
            "translation_model": "gemini-3.5-flash-lite", "lines": 2, "clear_after_ms": 4000,
            "hide_commands": true, "stream_captions": false, "stream_language": "", "labels": true,
            "latin_targets": [], "latin_lines": false})
    );
    assert_eq!(DEFAULT_TRANSLATION_MODEL, "gemini-3.5-flash-lite");
    AiSettings::default().validate().unwrap();
}

#[test]
fn older_settings_files_load_with_subtitles_off_and_new_ones_round_trip() {
    let path = temp_dir("subtitle-settings").join("ai.json");
    std::fs::write(&path, r#"{"voice": {"enabled": false}}"#).unwrap();
    let (loaded, warning) = load_settings(&path);
    assert_eq!((loaded.subtitles, warning), (SubtitleSettings::default(), None));
    let settings = AiSettings {
        subtitles: subtitles(json!({"consent": true, "enabled": true, "targets": ["es", "pt-BR"]})),
        ..AiSettings::default()
    };
    save_settings(&path, &settings).unwrap();
    assert_eq!(load_settings(&path).0, settings);
}

/// A change to the settings and the start of the refusal it gets.
type Case = (fn(&mut SubtitleSettings), &'static str);

#[test]
fn every_value_is_checked() {
    let cases: [Case; 10] = [
        (
            |s| s.enabled = true,
            "confirm that microphone audio may be sent to ElevenLabs for subtitles first",
        ),
        (
            |s| s.language = "English".to_owned(),
            "the spoken language must be empty or an ISO 639-1",
        ),
        (
            |s| s.language = "EN".to_owned(),
            "the spoken language must be empty or an ISO 639-1",
        ),
        (
            |s| s.targets = vec!["es".into(), "fr".into(), "de".into(), "ja".into()],
            "subtitles can be translated into at most 3",
        ),
        (|s| s.targets = vec!["es_MX".into()], "\"es_MX\" is not a language code"),
        (|s| s.targets = vec!["es".into(), "ES".into()], "ES is listed twice"),
        (
            |s| s.translation_model = "flash lite".to_owned(),
            "translation_model must be a model name",
        ),
        (|s| s.lines = 4, "subtitles show 1 to 3 lines"),
        (|s| s.clear_after_ms = 999, "clear_after_ms must be 1000 to 60000"),
        (
            |s| s.stream_language = "es".to_owned(),
            "stream captions must be in the spoken language or one of",
        ),
    ];
    for (edit, expected) in cases {
        let refused = refusal(edit);
        assert!(refused.starts_with(expected), "{refused}");
    }
    assert!(refusal(|s| s.lines = 0).contains("1 to 3 lines"));
    assert!(refusal(|s| s.clear_after_ms = 60_001).contains("1000 to 60000"));
    let mut fine = SubtitleSettings {
        language: "spa".to_owned(),
        targets: vec!["en".into(), "pt-BR".into(), "zh-Hans".into()],
        stream_language: "pt-BR".to_owned(),
        lines: 3,
        clear_after_ms: 60_000,
        ..SubtitleSettings::default()
    };
    fine.validate().unwrap();
    fine.lines = 1;
    fine.clear_after_ms = 1_000;
    fine.validate().unwrap();
}

#[test]
fn updates_take_codes_in_their_usual_case_and_keep_captions_on_a_target_they_have() {
    let settings = subtitles(json!({"targets": [" es ", "pt-br", "", "ZH-hant"], "stream_language": "PT-BR"}));
    assert_eq!(settings.targets, ["es", "pt-BR", "zh-Hant"]);
    assert_eq!(settings.stream_language, "pt-BR");
    let ai = AiSettings {
        subtitles: settings,
        ..AiSettings::default()
    };
    let next = ai.updated(&update(json!({"subtitles": {"targets": ["es"]}}))).unwrap();
    assert_eq!(
        next.subtitles.stream_language, "",
        "captions go back to the spoken language"
    );
    let refused = ai
        .updated(&update(json!({"subtitles": {"stream_language": "fr"}})))
        .unwrap_err();
    assert!(refused.to_string().starts_with("stream captions must be"), "{refused}");
    assert!(serde_json::from_value::<AiSettingsUpdate>(json!({"subtitles": {"color": "red"}})).is_err());
}

#[test]
fn consent_comes_first_and_withdrawing_it_turns_subtitles_off() {
    let refused = AiSettings::default()
        .updated(&update(json!({"subtitles": {"enabled": true}})))
        .unwrap_err();
    assert!(refused.to_string().contains("for subtitles first"), "{refused}");
    let ai = AiSettings {
        subtitles: subtitles(json!({"consent": true, "enabled": true})),
        ..AiSettings::default()
    };
    let off = ai.updated(&update(json!({"subtitles": {"consent": false}}))).unwrap();
    assert!(!off.subtitles.enabled && !off.subtitles.consent);
}

#[test]
fn the_view_prices_the_translation_model() {
    let prices = |settings: &AiSettings| {
        let view = settings.view();
        assert!(view.subtitles.translation_price_known);
        (
            view.subtitles.translation_input_price_per_mtok,
            view.subtitles.translation_output_price_per_mtok,
        )
    };
    assert_eq!(
        prices(&AiSettings::default()),
        (0.30, 2.50),
        "Gemini 3.5 Flash-Lite's list price"
    );
    let mut older_lite = AiSettings::default();
    "gemini-3.1-flash-lite".clone_into(&mut older_lite.subtitles.translation_model);
    assert_eq!(prices(&older_lite), (0.25, 1.50), "Gemini 3.1 Flash-Lite's list price");
    let view = AiSettings::default().view();
    assert_eq!(
        serde_json::to_value(&view).unwrap()["subtitles"]["lines"],
        2,
        "the settings are flattened in"
    );
    let local = AiSettings {
        provider: ProviderKind::OpenAiCompatible,
        ..AiSettings::default()
    };
    assert!(
        !local.view().subtitles.translation_price_known,
        "a local model has no list price"
    );
}

#[test]
fn the_status_follows_the_settings_the_key_the_input_and_the_budget() {
    let on = SubtitleSettings {
        enabled: true,
        consent: true,
        targets: vec!["es".to_owned()],
        ..SubtitleSettings::default()
    };
    let state = |settings: &SubtitleSettings, key: bool, input: &InputStatus, spent: bool| {
        let status = settings.status(key, input, spent, "en");
        (status.state, status.enabled)
    };
    let listening = InputStatus::Listening;
    let off = SubtitleSettings::default();
    assert_eq!(state(&off, true, &listening, false), ("off".to_owned(), false));
    assert_eq!(state(&on, false, &listening, false), ("needs_key".to_owned(), false));
    assert_eq!(state(&on, true, &listening, true), ("paused".to_owned(), false));
    assert_eq!(state(&on, true, &listening, false), ("listening".to_owned(), true));
    assert_eq!(
        state(&on, true, &InputStatus::Ready, false),
        ("connecting".to_owned(), true)
    );
    let retrying = InputStatus::Retrying {
        message: "the server ended the session".to_owned(),
        retry_in_ms: 2_000,
    };
    assert_eq!(state(&on, true, &retrying, false), ("retrying".to_owned(), true));
    let status = on.status(true, &InputStatus::Listening, true, "en");
    assert_eq!(
        status.message,
        "the hourly budget is spent; subtitles resume when the budget allows"
    );
    assert_eq!(
        (status.language.as_str(), status.targets.as_slice()),
        ("en", ["es".to_owned()].as_slice())
    );
}
