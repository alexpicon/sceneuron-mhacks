// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the AI settings: defaults, the key-free editor view, and partial updates with
//! their validation.

mod common;

use common::{TEST_KEY, enabled_settings};
use sceneuron_ai::{AiError, AiSettings, AiSettingsUpdate, ProviderKind};

fn update(json: &str) -> AiSettingsUpdate {
    serde_json::from_str(json).unwrap()
}

fn config_error(result: Result<AiSettings, AiError>) -> String {
    match result {
        Err(AiError::Config(message)) => message,
        other => panic!("expected a configuration error, got {other:?}"),
    }
}

#[test]
fn defaults_are_off_without_consent_and_use_the_default_model() {
    let settings = AiSettings::default();
    assert!(!settings.enabled && !settings.consent);
    assert_eq!(settings.provider, ProviderKind::Gemini);
    assert_eq!(settings.base_url, "https://generativelanguage.googleapis.com");
    assert_eq!(settings.vision_model, "gemini-3.5-flash-lite");
    assert_eq!(settings.planner_model, "gemini-3.8-flash");
    assert_eq!(settings.subtitles.translation_model, "gemini-3.5-flash-lite");
    assert_eq!(
        (settings.vision_effort.as_str(), settings.planner_effort.as_str()),
        ("low", "medium")
    );
    assert!(settings.api_key.is_empty());
    assert_eq!(settings.image_width, 768);
    assert_eq!(
        (settings.input_price_per_mtok, settings.output_price_per_mtok),
        (None, None)
    );
    settings.validate().unwrap();
}

#[test]
fn missing_fields_in_json_take_their_defaults() {
    let settings: AiSettings = serde_json::from_str(r#"{"provider": "openai-compatible"}"#).unwrap();
    assert_eq!(settings.provider, ProviderKind::OpenAiCompatible);
    assert_eq!(settings.image_width, 768);
    let json = serde_json::to_value(ProviderKind::OpenAiCompatible).unwrap();
    assert_eq!(json, "openai-compatible");
}

#[test]
fn the_view_hides_the_key_and_shows_a_hint() {
    let settings = enabled_settings();
    let view = settings.view();
    assert!(view.available && view.has_key);
    assert_eq!(view.key_hint, "...abcd");
    let json = serde_json::to_string(&view).unwrap();
    assert!(!json.contains(TEST_KEY) && !json.contains("api_key"), "{json}");
    assert!(!format!("{settings:?}").contains(TEST_KEY));
    let short = AiSettings {
        api_key: "abc".to_owned(),
        ..AiSettings::default()
    };
    assert_eq!(short.view().key_hint, "...");
    assert_eq!(AiSettings::default().view().key_hint, "");
    assert!(!AiSettings::default().view().has_key);
}

#[test]
fn the_view_reports_effective_prices_and_whether_they_are_known() {
    assert!(AiSettings::default().view().price_known);
    let pro = AiSettings {
        vision_model: "gemini-3.1-pro-preview".to_owned(),
        ..AiSettings::default()
    };
    let view = pro.view();
    assert!(view.price_known);
    assert_eq!(
        (
            view.effective_input_price_per_mtok,
            view.effective_output_price_per_mtok
        ),
        (2.0, 12.0)
    );
    let local = AiSettings {
        provider: ProviderKind::OpenAiCompatible,
        ..AiSettings::default()
    };
    let view = local.view();
    assert!(!view.price_known);
    assert_eq!(view.effective_input_price_per_mtok, 0.0);
    let priced = AiSettings {
        input_price_per_mtok: Some(0.5),
        output_price_per_mtok: Some(1.5),
        ..local
    };
    let view = priced.view();
    assert!(view.price_known);
    assert_eq!(
        (view.input_price_per_mtok, view.effective_output_price_per_mtok),
        (Some(0.5), 1.5)
    );
}

#[test]
fn updates_change_only_the_fields_they_set() {
    let next = AiSettings::default()
        .updated(&update(r#"{"vision_effort": "high", "image_width": 1024}"#))
        .unwrap();
    assert_eq!((next.vision_effort.as_str(), next.image_width), ("high", 1024));
    assert_eq!(next.planner_effort, "medium");
    assert_eq!(next.vision_model, "gemini-3.5-flash-lite");
}

#[test]
fn updates_refuse_unknown_fields() {
    assert!(serde_json::from_str::<AiSettingsUpdate>(r#"{"api_kee": "x"}"#).is_err());
    assert!(serde_json::from_str::<AiSettingsUpdate>(r#"{"enabled": true, "extra": 1}"#).is_err());
}

#[test]
fn invalid_values_are_refused_with_a_reason() {
    let base = AiSettings::default();
    let cases = [
        (r#"{"vision_effort": "max"}"#, "vision_effort"),
        (r#"{"planner_effort": "LOW"}"#, "planner_effort"),
        (r#"{"image_width": 255}"#, "image_width"),
        (r#"{"image_width": 1569}"#, "image_width"),
        (r#"{"base_url": "ftp://example.com"}"#, "base_url"),
        (r#"{"base_url": "https://"}"#, "base_url"),
        (r#"{"base_url": "https://user@example.com"}"#, "base_url"),
        (r#"{"vision_model": ""}"#, "vision_model"),
        (r#"{"planner_model": "two words"}"#, "planner_model"),
        (r#"{"input_price_per_mtok": -1}"#, "input_price_per_mtok"),
        (r#"{"output_price_per_mtok": 1000.5}"#, "output_price_per_mtok"),
        (r#"{"api_key": "has inner space"}"#, "API key"),
    ];
    for (json, field) in cases {
        let message = config_error(base.updated(&update(json)));
        assert!(message.contains(field), "{json}: {message}");
    }
    let not_finite = AiSettings {
        input_price_per_mtok: Some(f64::NAN),
        ..AiSettings::default()
    };
    assert!(not_finite.validate().is_err());
}

#[test]
fn enabling_requires_consent() {
    let message = config_error(AiSettings::default().updated(&update(r#"{"enabled": true}"#)));
    assert_eq!(message, "confirm that camera images may be sent to the provider first");
    let next = AiSettings::default()
        .updated(&update(r#"{"enabled": true, "consent": true}"#))
        .unwrap();
    assert!(next.enabled && next.consent);
}

#[test]
fn withdrawing_consent_turns_scene_understanding_off() {
    let next = enabled_settings().updated(&update(r#"{"consent": false}"#)).unwrap();
    assert!(!next.enabled && !next.consent);
    let refused = enabled_settings().updated(&update(r#"{"consent": false, "enabled": true}"#));
    assert!(matches!(refused, Err(AiError::Config(_))));
}

#[test]
fn switching_provider_switches_to_its_default_url_unless_one_is_given() {
    let local = AiSettings::default()
        .updated(&update(r#"{"provider": "openai-compatible"}"#))
        .unwrap();
    assert_eq!(local.base_url, "http://127.0.0.1:11434/v1");
    let nim = AiSettings::default()
        .updated(&update(
            r#"{"provider": "openai-compatible", "base_url": "https://integrate.api.nvidia.com/v1/"}"#,
        ))
        .unwrap();
    assert_eq!(nim.base_url, "https://integrate.api.nvidia.com/v1");
    let same = local.updated(&update(r#"{"provider": "openai-compatible"}"#)).unwrap();
    assert_eq!(same.base_url, local.base_url);
}

#[test]
fn an_empty_key_clears_it_and_keys_are_trimmed() {
    let with_key = AiSettings::default()
        .updated(&update(&format!(r#"{{"api_key": "  {TEST_KEY}\n"}}"#)))
        .unwrap();
    assert_eq!(with_key.api_key, TEST_KEY);
    let cleared = with_key.updated(&update(r#"{"api_key": ""}"#)).unwrap();
    assert!(cleared.api_key.is_empty());
    let untouched = with_key.updated(&update(r#"{"vision_effort": "low"}"#)).unwrap();
    assert_eq!(untouched.api_key, TEST_KEY);
    let debug = format!("{:?}", update(&format!(r#"{{"api_key": "{TEST_KEY}"}}"#)));
    assert!(!debug.contains(TEST_KEY), "{debug}");
}

#[test]
fn a_null_price_removes_the_override() {
    let priced = AiSettings::default()
        .updated(&update(r#"{"input_price_per_mtok": 3.5, "output_price_per_mtok": 7}"#))
        .unwrap();
    assert_eq!(
        (priced.input_price_per_mtok, priced.output_price_per_mtok),
        (Some(3.5), Some(7.0))
    );
    let cleared = priced.updated(&update(r#"{"input_price_per_mtok": null}"#)).unwrap();
    assert_eq!(
        (cleared.input_price_per_mtok, cleared.output_price_per_mtok),
        (None, Some(7.0))
    );
}
