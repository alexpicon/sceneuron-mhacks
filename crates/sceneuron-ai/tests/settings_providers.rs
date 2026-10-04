// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the providers in the AI settings: Gemini as the default with its models, each
//! provider's URL and models, saved files keeping their provider and getting its models for the
//! roles they leave out, and switching provider through an update, which also leaves the old
//! provider's key and price overrides behind.

mod common;

use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use common::{TEST_KEY, temp_dir};
use sceneuron_ai::{
    AiSettings, AiSettingsUpdate, Credentials, DEFAULT_MODEL, DEFAULT_TRANSLATION_MODEL, DefaultModels, ProviderKind,
    gemini_prices, load_settings, prices_for, save_settings,
};
use serde_json::{Value, json};

const GEMINI_URL: &str = "https://generativelanguage.googleapis.com";
const LOCAL_URL: &str = "http://127.0.0.1:11434/v1";

/// The vision, planner, and translation models.
fn models(settings: &AiSettings) -> (&str, &str, &str) {
    (
        settings.vision_model.as_str(),
        settings.planner_model.as_str(),
        settings.subtitles.translation_model.as_str(),
    )
}

/// The settings `json` loads as, which must load without a warning.
fn loaded(name: &str, json: &Value) -> AiSettings {
    let path: PathBuf = temp_dir(name).join("ai.json");
    fs::write(&path, json.to_string()).unwrap();
    let (settings, warning) = load_settings(&path);
    assert_eq!(warning, None, "{json}");
    settings
}

fn updated(settings: &AiSettings, json: Value) -> AiSettings {
    let update: AiSettingsUpdate = serde_json::from_value(json).unwrap();
    settings.updated(&update).unwrap()
}

/// An OpenAI-compatible server with a model of its own for every role.
fn local() -> AiSettings {
    updated(
        &AiSettings::default(),
        json!({"provider": "openai-compatible", "vision_model": "llama3.2-vision", "planner_model": "qwen3",
            "subtitles": {"translation_model": "aya-expanse"}}),
    )
}

#[test]
fn gemini_is_the_default_and_each_provider_has_its_url_and_models() {
    let settings = AiSettings::default();
    assert_eq!(
        (settings.provider, settings.base_url.as_str()),
        (ProviderKind::Gemini, GEMINI_URL)
    );
    assert_eq!(
        models(&settings),
        ("gemini-3.5-flash-lite", "gemini-3.8-flash", "gemini-3.5-flash-lite")
    );
    assert_eq!(
        (DEFAULT_MODEL, DEFAULT_TRANSLATION_MODEL),
        ("gemini-3.5-flash-lite", "gemini-3.5-flash-lite")
    );
    assert_eq!(ProviderKind::default(), ProviderKind::Gemini);
    assert_eq!(serde_json::to_value(ProviderKind::Gemini).unwrap(), "gemini");
    assert_eq!(
        serde_json::from_value::<ProviderKind>(json!("gemini")).unwrap(),
        ProviderKind::Gemini
    );
    assert_eq!(ProviderKind::Gemini.name(), "gemini");
    assert_eq!(
        ProviderKind::Gemini.default_models(),
        Some(DefaultModels {
            vision: "gemini-3.5-flash-lite",
            planner: "gemini-3.8-flash",
            translation: "gemini-3.5-flash-lite",
        })
    );
    assert_eq!(ProviderKind::OpenAiCompatible.default_base_url(), LOCAL_URL);
    assert_eq!(ProviderKind::OpenAiCompatible.default_models(), None);
    assert_eq!(settings.view().provider, ProviderKind::Gemini);
    settings.validate().unwrap();
}

#[test]
fn a_saved_openai_compatible_file_stays_on_its_server() {
    let path = temp_dir("providers-saved").join("ai.json");
    let settings = AiSettings {
        api_key: "nvapi-test-key".to_owned(),
        ..local()
    };
    save_settings(&path, &settings).unwrap();
    assert_eq!(load_settings(&path), (settings, None));
}

#[test]
fn a_file_that_names_only_gemini_gets_its_url_and_models() {
    let settings = loaded("providers-partial", &json!({"provider": "gemini"}));
    assert_eq!(settings, AiSettings::default());
    let named = loaded(
        "providers-named",
        &json!({"provider": "gemini", "base_url": "https://gateway.example.com",
            "planner_model": "gemini-3.1-pro-preview",
            "subtitles": {"targets": ["es"], "translation_model": "gemini-3.1-flash-lite"}}),
    );
    assert_eq!(named.base_url, "https://gateway.example.com");
    assert_eq!(
        models(&named),
        (
            "gemini-3.5-flash-lite",
            "gemini-3.1-pro-preview",
            "gemini-3.1-flash-lite"
        )
    );
    let subtitles_only = loaded(
        "providers-subtitles",
        &json!({"provider": "gemini", "subtitles": {"targets": ["es"]}}),
    );
    assert_eq!(subtitles_only.subtitles.translation_model, "gemini-3.5-flash-lite");
}

#[test]
fn a_file_without_a_provider_is_gemini_and_keeps_the_models_it_names() {
    let settings = loaded("providers-none", &json!({"enabled": false}));
    assert_eq!(settings, AiSettings::default());
    let named = loaded(
        "providers-none-named",
        &json!({"vision_model": "gemini-3.1-pro-preview"}),
    );
    assert_eq!(
        (named.provider, named.base_url.as_str()),
        (ProviderKind::Gemini, GEMINI_URL)
    );
    assert_eq!(
        models(&named),
        ("gemini-3.1-pro-preview", "gemini-3.8-flash", "gemini-3.5-flash-lite")
    );
}

#[test]
fn a_file_on_an_openai_compatible_server_gets_its_url_and_keeps_the_models() {
    let settings = loaded("providers-local", &json!({"provider": "openai-compatible"}));
    assert_eq!(settings.base_url, LOCAL_URL);
    assert_eq!(models(&settings), models(&AiSettings::default()));
}

#[test]
fn switching_provider_takes_its_models_for_every_role_the_update_leaves_out() {
    let local = local();
    assert_eq!(local.base_url, LOCAL_URL);
    assert_eq!(models(&local), ("llama3.2-vision", "qwen3", "aya-expanse"));
    let gemini = updated(
        &local,
        json!({"provider": "gemini", "planner_model": "gemini-3.1-pro-preview"}),
    );
    assert_eq!(gemini.base_url, GEMINI_URL);
    assert_eq!(
        models(&gemini),
        (
            "gemini-3.5-flash-lite",
            "gemini-3.1-pro-preview",
            "gemini-3.5-flash-lite"
        )
    );
    let translated = updated(
        &local,
        json!({"provider": "gemini", "subtitles": {"translation_model": "gemini-3.1-flash-lite", "targets": ["fr"]}}),
    );
    assert_eq!(
        models(&translated),
        ("gemini-3.5-flash-lite", "gemini-3.8-flash", "gemini-3.1-flash-lite")
    );
    assert_eq!(translated.subtitles.targets, ["fr"]);
}

#[test]
fn switching_to_an_openai_compatible_server_or_to_the_same_provider_keeps_the_models() {
    let custom = updated(
        &AiSettings::default(),
        json!({"vision_model": "gemini-3.1-pro-preview", "subtitles": {"translation_model": "gemini-3.1-flash-lite"}}),
    );
    let same = updated(&custom, json!({"provider": "gemini"}));
    assert_eq!(same, custom);
    let local = updated(&custom, json!({"provider": "openai-compatible"}));
    assert_eq!(local.base_url, LOCAL_URL);
    assert_eq!(models(&local), models(&custom));
    let back = updated(
        &local,
        json!({"provider": "gemini", "base_url": "https://proxy.example.com/"}),
    );
    assert_eq!(back.base_url, "https://proxy.example.com", "a URL in the update wins");
    assert_eq!(models(&back), models(&AiSettings::default()));
}

#[test]
fn switching_provider_removes_the_old_providers_key_so_the_new_ones_from_the_environment_applies() {
    let nim = AiSettings {
        api_key: "nvapi-saved-in-the-editor".to_owned(),
        ..local()
    };
    let gemini = updated(&nim, json!({"provider": "gemini"}));
    assert_eq!(gemini.api_key, "", "an NVIDIA NIM key must never be sent to Google");
    let credentials = Credentials {
        gemini: TEST_KEY.to_owned(),
        openai: "nvapi-from-the-environment".to_owned(),
        ..Credentials::default()
    };
    assert_eq!(gemini.with_credentials(&credentials).api_key, TEST_KEY);
    let view = gemini.view_with(&credentials);
    assert_eq!((view.has_key, view.key_source.as_str()), (true, "environment"));
    let typed = updated(&nim, json!({"provider": "gemini", "api_key": TEST_KEY}));
    assert_eq!(
        typed.api_key, TEST_KEY,
        "a key in the same update is the new provider's"
    );
    let same = updated(&nim, json!({"provider": "openai-compatible"}));
    assert_eq!(same.api_key, nim.api_key, "naming the same provider is no switch");
}

#[test]
fn switching_provider_drops_price_overrides_so_the_new_models_list_prices_apply() {
    let local = updated(
        &AiSettings::default(),
        json!({"provider": "openai-compatible", "input_price_per_mtok": 0.0, "output_price_per_mtok": 0.0}),
    );
    assert_eq!(
        prices_for(&local, &local.vision_model).map(|prices| (prices.input, prices.output)),
        Some((0.0, 0.0))
    );
    let gemini = updated(&local, json!({"provider": "gemini"}));
    assert_eq!(
        (gemini.input_price_per_mtok, gemini.output_price_per_mtok),
        (None, None)
    );
    assert_eq!(
        prices_for(&gemini, &gemini.vision_model),
        gemini_prices(&gemini.vision_model, SystemTime::now())
    );
    assert!(prices_for(&gemini, &gemini.vision_model).is_some_and(|prices| prices.input > 0.0));
    let named = updated(&local, json!({"provider": "gemini", "output_price_per_mtok": 30.0}));
    assert_eq!(
        (named.input_price_per_mtok, named.output_price_per_mtok),
        (None, Some(30.0)),
        "prices in the same update are the new provider's"
    );
}
