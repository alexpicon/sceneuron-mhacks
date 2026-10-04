// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises optional keys from `.env` files: parsing, which names count, warnings, keys the
//! editor saved winning over them, where the editor is told a key came from, and that keys from
//! the environment are used but never saved.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use common::sink::RecordingSink;
use common::{TEST_KEY, temp_dir};
use sceneuron_ai::{
    AiSettings, AiSettingsUpdate, Assistant, Credentials, ELEVENLABS_KEY_VAR, GEMINI_KEY_VAR, JEV_KEY_VAR,
    ProviderKind, default_connector, parse_env,
};
use serde_json::{Value, json};

fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

#[test]
fn env_files_allow_comments_export_and_quotes() {
    let text = "# keys\n\nSCENEURON_GEMINI_API_KEY=gemini-1 # mine\nexport SCENEURON_JEV_API_KEY=\"jev key\"\n\
                SCENEURON_OPENAI_API_KEY='nim'\nnot a line\n=missing\nBAD-NAME=1\n";
    let (parsed, bad) = parse_env(text);
    assert_eq!(parsed.get(GEMINI_KEY_VAR).map(String::as_str), Some("gemini-1"));
    assert_eq!(parsed.get(JEV_KEY_VAR).map(String::as_str), Some("jev key"));
    assert_eq!(parsed.get("SCENEURON_OPENAI_API_KEY").map(String::as_str), Some("nim"));
    assert_eq!(bad, [6, 7, 8]);
}

#[test]
fn only_sceneuron_names_count() {
    let found = Credentials::from_values(&values(&[
        ("OPENAI_API_KEY", "other-tool"),
        (JEV_KEY_VAR, "jev-secret"),
    ]));
    assert_eq!(found.jev, "jev-secret");
    assert_eq!(found.for_provider(ProviderKind::OpenAiCompatible), "");
    assert!(!format!("{found:?}").contains("jev-secret"));
    let left_over = Credentials::from_values(&values(&[("SCENEURON_ANTHROPIC_API_KEY", "sk-ant-left-over")]));
    assert_eq!(
        left_over,
        Credentials::default(),
        "an Anthropic key left in a .env file is never sent to Google"
    );
}

#[test]
fn a_missing_file_is_fine_and_a_shared_one_is_flagged() {
    let dir = temp_dir("credentials");
    let (_, warnings) = Credentials::load(Some(&dir.join("absent.env")));
    assert!(warnings.is_empty());
    let path = dir.join(".env");
    fs::write(&path, format!("{JEV_KEY_VAR}=from-file\nwhat\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let (found, warnings) = Credentials::load(Some(&path));
    assert_eq!(found.jev, "from-file");
    assert!(warnings.iter().any(|w| w.contains("ignored lines 2")), "{warnings:?}");
    assert!(warnings.iter().any(|w| w.contains("chmod 600")), "{warnings:?}");
}

#[test]
fn keys_saved_in_the_editor_win_and_sources_are_reported() {
    let credentials = Credentials {
        openai: "env-openai-key-1234".to_owned(),
        jev: "env-jev-key-5678".to_owned(),
        elevenlabs: "env-elevenlabs-key-9012".to_owned(),
        ..Credentials::default()
    };
    let settings = AiSettings {
        provider: ProviderKind::OpenAiCompatible,
        ..AiSettings::default()
    };
    let effective = settings.with_credentials(&credentials);
    assert_eq!(
        (effective.api_key.as_str(), effective.jev_api_key.as_str()),
        ("env-openai-key-1234", "env-jev-key-5678")
    );
    assert_eq!(effective.elevenlabs_api_key, "env-elevenlabs-key-9012");
    let view = settings.view_with(&credentials);
    assert_eq!(
        (view.key_source.as_str(), view.key_hint.as_str()),
        ("environment", "...1234")
    );
    assert_eq!((view.jev_key_source.as_str(), view.has_jev_key), ("environment", true));
    assert_eq!(
        (
            view.elevenlabs_key_source.as_str(),
            view.elevenlabs_key_hint.as_str(),
            view.has_elevenlabs_key
        ),
        ("environment", "...9012", true)
    );
    let saved = AiSettings {
        api_key: TEST_KEY.to_owned(),
        ..settings.clone()
    };
    assert_eq!(saved.with_credentials(&credentials).api_key, TEST_KEY);
    assert_eq!(saved.view_with(&credentials).key_source, "editor");
    let none = AiSettings::default().view_with(&Credentials::default());
    assert_eq!((none.key_source.as_str(), none.jev_key_source.as_str()), ("", ""));
    let json = serde_json::to_string(&view).unwrap();
    assert!(!json.contains("env-openai") && !json.contains("env-jev") && !json.contains("env-eleven"));
    assert!(!format!("{credentials:?}").contains("env-eleven"));
}

#[test]
fn the_elevenlabs_key_comes_from_its_own_name() {
    let found = Credentials::from_values(&values(&[
        ("ELEVENLABS_API_KEY", "other-tool"),
        (ELEVENLABS_KEY_VAR, "el-secret"),
    ]));
    assert_eq!(ELEVENLABS_KEY_VAR, "SCENEURON_ELEVENLABS_API_KEY");
    assert_eq!(found.elevenlabs, "el-secret");
}

#[test]
fn the_gemini_key_comes_from_its_own_name() {
    let found = Credentials::from_values(&values(&[
        ("GEMINI_API_KEY", "other-tool"),
        ("GOOGLE_API_KEY", "other-tool"),
        (GEMINI_KEY_VAR, "gemini-secret"),
    ]));
    assert_eq!(GEMINI_KEY_VAR, "SCENEURON_GEMINI_API_KEY");
    assert_eq!(found.gemini, "gemini-secret");
    assert!(!format!("{found:?}").contains("gemini-secret"));
}

#[test]
fn each_provider_takes_its_own_key() {
    let credentials = Credentials {
        gemini: "env-gemini-key-3456".to_owned(),
        openai: "env-openai-key-7890".to_owned(),
        ..Credentials::default()
    };
    for (provider, key) in [
        (ProviderKind::Gemini, "env-gemini-key-3456"),
        (ProviderKind::OpenAiCompatible, "env-openai-key-7890"),
    ] {
        assert_eq!(credentials.for_provider(provider), key);
    }
    let gemini = AiSettings::default();
    assert_eq!(gemini.with_credentials(&credentials).api_key, "env-gemini-key-3456");
    let view = gemini.view_with(&credentials);
    assert_eq!(
        (view.key_source.as_str(), view.key_hint.as_str()),
        ("environment", "...3456")
    );
}

#[test]
fn keys_from_the_environment_are_never_saved() {
    let path = temp_dir("credentials-save").join("ai.json");
    let credentials = Credentials {
        gemini: "env-gemini-key-3456".to_owned(),
        elevenlabs: "env-elevenlabs-key-9012".to_owned(),
        ..Credentials::default()
    };
    let sink = Arc::new(RecordingSink::default());
    let (assistant, _) = Assistant::new(path.clone(), default_connector(), sink, credentials);
    assert_eq!(assistant.settings_view().key_source, "environment");
    let update: AiSettingsUpdate = serde_json::from_value(json!({"consent": true, "jev_model": "jev-test"})).unwrap();
    assistant.update_settings(update).unwrap();
    let saved: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(saved["api_key"], "");
    assert_eq!(saved["elevenlabs_api_key"], "");
    assert_eq!(saved["jev_model"], "jev-test");
    assistant.shutdown();
}
