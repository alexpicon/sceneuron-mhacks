// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises AI settings saved for Anthropic, which Sceneuron no longer offers: a full file, a
//! partial one, and one without a provider that holds an Anthropic key or Claude model load as
//! Gemini with its URL and default models, without the saved key, price overrides, or consent that
//! named Anthropic, and with a warning that says so; and an update that names Anthropic, or any
//! unknown provider, is refused.

mod common;

use common::TEST_KEY;
use common::retired::{ANTHROPIC_KEY, saved_on_anthropic, warning, written};
use sceneuron_ai::{AiSettings, AiSettingsUpdate, ProviderKind, load_settings};
use sceneuron_core::VoiceMode;
use serde_json::json;

/// Everything a streamer may have set besides the provider, all of it valid on Gemini too.
fn customized() -> AiSettings {
    let mut settings = AiSettings {
        enabled: true,
        consent: true,
        vision_effort: "high".to_owned(),
        planner_effort: "low".to_owned(),
        image_width: 1024,
        jev_api_key: "jev-key-0123456789".to_owned(),
        jev_model: "jev-test".to_owned(),
        elevenlabs_api_key: "el-key-0123456789".to_owned(),
        ..AiSettings::default()
    };
    settings.voice.consent = true;
    settings.voice.enabled = true;
    settings.voice.mode = VoiceMode::AlwaysOn;
    settings.subtitles.consent = true;
    settings.subtitles.enabled = true;
    settings.subtitles.targets = vec!["es".to_owned()];
    settings
}

#[test]
fn a_full_anthropic_file_loads_as_gemini_without_its_key_prices_or_consent() {
    let path = written("retired-full", &saved_on_anthropic(&customized()));
    let (settings, shown) = load_settings(&path);
    let mut expected = AiSettings {
        enabled: false,
        consent: false,
        ..customized()
    };
    expected.subtitles.consent = false;
    expected.subtitles.enabled = false;
    assert_eq!(settings, expected);
    assert_eq!(
        (settings.provider, settings.base_url.as_str(), settings.api_key.as_str()),
        (ProviderKind::Gemini, "https://generativelanguage.googleapis.com", "")
    );
    assert_eq!(
        (
            settings.vision_model.as_str(),
            settings.subtitles.translation_model.as_str()
        ),
        ("gemini-3.5-flash-lite", "gemini-3.5-flash-lite")
    );
    assert_eq!(
        (settings.input_price_per_mtok, settings.output_price_per_mtok),
        (None, None)
    );
    assert_eq!(
        shown.as_deref(),
        Some(warning(&path, Some("camera images and subtitle lines")).as_str())
    );
    assert!(!shown.unwrap().contains(ANTHROPIC_KEY));
}

#[test]
fn a_partial_anthropic_file_loads_as_the_gemini_defaults() {
    let bare = written("retired-bare", &json!({"provider": "anthropic"}));
    assert_eq!(
        load_settings(&bare),
        (AiSettings::default(), Some(warning(&bare, None)))
    );
    let some = written(
        "retired-some",
        &json!({"provider": "anthropic", "api_key": ANTHROPIC_KEY, "planner_model": "claude-fable-5-1",
            "output_price_per_mtok": 50.0, "subtitles": {"targets": ["fr"], "translation_model": "claude-sonnet-5"}}),
    );
    let (settings, shown) = load_settings(&some);
    let mut expected = AiSettings::default();
    expected.subtitles.targets = vec!["fr".to_owned()];
    assert_eq!(settings, expected);
    assert_eq!(
        shown,
        Some(warning(&some, None)),
        "no consent was given, so none is withdrawn"
    );
    let consented = written(
        "retired-consented",
        &json!({"provider": "anthropic", "consent": true, "subtitles": {"consent": true, "enabled": true}}),
    );
    let (settings, shown) = load_settings(&consented);
    assert!(!settings.consent);
    assert!(
        settings.subtitles.consent && settings.subtitles.enabled,
        "subtitles without translations never reached Anthropic"
    );
    assert_eq!(shown, Some(warning(&consented, Some("camera images"))));
}

#[test]
fn a_file_without_a_provider_loses_an_anthropic_key_or_claude_model() {
    let keyed = written(
        "retired-unnamed-key",
        &json!({"api_key": ANTHROPIC_KEY, "vision_model": "claude-opus-5-5", "consent": true, "enabled": true}),
    );
    let (settings, shown) = load_settings(&keyed);
    assert_eq!(settings, AiSettings::default());
    assert_eq!(shown, Some(warning(&keyed, Some("camera images"))));
    let modeled = written(
        "retired-unnamed-model",
        &json!({"subtitles": {"translation_model": "claude-haiku-4-5"}, "jev_model": "jev-test"}),
    );
    let (settings, shown) = load_settings(&modeled);
    let expected = AiSettings {
        jev_model: "jev-test".to_owned(),
        ..AiSettings::default()
    };
    assert_eq!((settings, shown), (expected, Some(warning(&modeled, None))));
    let gemini = written(
        "retired-unnamed-gemini",
        &json!({"api_key": TEST_KEY, "vision_model": "gemini-3.1-pro-preview"}),
    );
    let (settings, shown) = load_settings(&gemini);
    assert_eq!(
        (
            settings.provider,
            settings.api_key.as_str(),
            settings.vision_model.as_str(),
            shown
        ),
        (ProviderKind::Gemini, TEST_KEY, "gemini-3.1-pro-preview", None)
    );
    let named = json!({"provider": "openai-compatible", "base_url": "http://127.0.0.1:4000/v1",
        "vision_model": "claude-opus-5-5", "planner_model": "claude-opus-5-5"});
    let (settings, shown) = load_settings(&written("retired-named-local", &named));
    assert_eq!(
        (settings.provider, settings.vision_model.as_str(), shown),
        (ProviderKind::OpenAiCompatible, "claude-opus-5-5", None),
        "a file that names a provider keeps the models it names"
    );
}

#[test]
fn an_update_that_names_anthropic_or_an_unknown_provider_is_refused() {
    let refused = serde_json::from_value::<AiSettingsUpdate>(json!({"provider": "anthropic"})).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "Anthropic is no longer offered; use gemini or openai-compatible"
    );
    let refused = serde_json::from_str::<AiSettingsUpdate>(r#"{"provider": "anthropic", "api_key": "x"}"#).unwrap_err();
    assert!(
        refused.to_string().starts_with("Anthropic is no longer offered"),
        "{refused}"
    );
    let unknown = serde_json::from_value::<ProviderKind>(json!("vertex")).unwrap_err();
    assert_eq!(
        unknown.to_string(),
        "unknown provider \"vertex\"; use gemini or openai-compatible"
    );
    assert_eq!(
        ProviderKind::try_from("openai-compatible".to_owned()),
        Ok(ProviderKind::OpenAiCompatible)
    );
}
