// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the starter AI settings: a fresh install gets them as a private settings file that
//! loads like a saved one, with the starter's voice and subtitle choices but no key, no token, no
//! consent, and nothing that sends data switched on, and an existing settings file is never
//! changed.

mod common;

use std::fs;

use common::temp_dir;
use sceneuron_ai::{
    AiSettings, DEFAULT_MODEL, DEFAULT_PLANNER_MODEL, DEFAULT_STT_MODEL, DEFAULT_TRANSLATION_MODEL, DEFAULT_TTS_MODEL,
    DEFAULT_VOICE_ID, ProviderKind, load_settings, place_starter_settings,
};
use sceneuron_core::VoiceMode;
use serde_json::Value;

/// The starter settings put in place in a fresh directory, with the file's text.
fn placed(name: &str) -> (AiSettings, Value) {
    let path = temp_dir(name).join("config").join("ai.json");
    assert!(place_starter_settings(&path).unwrap());
    let (settings, warning) = load_settings(&path);
    assert_eq!(warning, None, "the starter must load like saved settings");
    (
        settings,
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap(),
    )
}

/// Every field of `value`, at any depth, as its path and value.
fn fields<'a>(value: &'a Value, path: &str, out: &mut Vec<(String, &'a Value)>) {
    if let Value::Object(map) = value {
        for (name, item) in map {
            let here = format!("{path}.{name}");
            fields(item, &here, out);
            out.push((here, item));
        }
    }
}

#[test]
fn a_fresh_install_gets_gemini_always_on_voice_and_japanese_subtitles() {
    let (settings, _) = placed("starter-choices");
    assert_eq!(settings.provider, ProviderKind::Gemini);
    assert_eq!(settings.base_url, ProviderKind::Gemini.default_base_url());
    assert_eq!(
        (settings.vision_model.as_str(), settings.planner_model.as_str()),
        (DEFAULT_MODEL, DEFAULT_PLANNER_MODEL)
    );
    assert_eq!(settings.subtitles.translation_model, DEFAULT_TRANSLATION_MODEL);
    assert_eq!(
        (settings.vision_effort.as_str(), settings.planner_effort.as_str()),
        ("low", "medium")
    );
    assert_eq!(settings.image_width, 768);
    assert_eq!(
        (settings.input_price_per_mtok, settings.output_price_per_mtok),
        (None, None)
    );
    let voice = &settings.voice;
    assert_eq!(
        (voice.mode, voice.wake_word.as_str(), voice.language.as_str()),
        (VoiceMode::AlwaysOn, "Sceneuron", "")
    );
    assert_eq!(voice.voice_id, DEFAULT_VOICE_ID);
    assert_eq!(
        (voice.tts_model.as_str(), voice.stt_model.as_str()),
        (DEFAULT_TTS_MODEL, DEFAULT_STT_MODEL)
    );
    let subtitles = &settings.subtitles;
    assert_eq!(
        (subtitles.targets.as_slice(), subtitles.lines, subtitles.clear_after_ms),
        (&["ja".to_owned()][..], 2, 4000)
    );
    assert!(subtitles.hide_commands && subtitles.stream_captions);
    assert_eq!(
        (subtitles.language.as_str(), subtitles.stream_language.as_str()),
        ("", "")
    );
}

#[test]
fn the_starter_holds_no_key_or_token_no_consent_and_nothing_that_sends_data() {
    let (settings, file) = placed("starter-secrets");
    let mut all = Vec::new();
    fields(&file, "$", &mut all);
    for (path, value) in &all {
        let name = path.rsplit('.').next().unwrap().to_lowercase();
        if name.contains("key") || name.contains("token") {
            assert_eq!(*value, &Value::from(""), "{path} must be empty");
        }
        if name == "consent" || name == "enabled" {
            assert_eq!(*value, &Value::from(false), "{path} must be false");
        }
    }
    assert!(
        all.iter().any(|(path, _)| path == "$.elevenlabs_api_key"),
        "the keys are named, and empty"
    );
    assert!(settings.api_key.is_empty() && settings.jev_api_key.is_empty() && settings.elevenlabs_api_key.is_empty());
    assert!(!(settings.consent || settings.voice.consent || settings.subtitles.consent));
    assert!(!(settings.enabled || settings.voice.enabled || settings.subtitles.enabled));
    // Spoken cards need only a key, so they too wait for the user to switch them on.
    assert!(!settings.voice.speak);
}

#[test]
fn every_value_the_starter_sets_survives_loading() {
    let (settings, file) = placed("starter-kept");
    let loaded = serde_json::to_value(&settings).unwrap();
    let mut all = Vec::new();
    fields(&file, "$", &mut all);
    for (path, value) in all.into_iter().filter(|(_, value)| !value.is_object()) {
        let pointer = path.trim_start_matches('$').replace('.', "/");
        assert_eq!(loaded.pointer(&pointer), Some(value), "{path}");
    }
}

#[cfg(unix)]
#[test]
fn the_starter_file_is_readable_only_by_its_owner() {
    use std::os::unix::fs::PermissionsExt;

    let path = temp_dir("starter-mode").join("ai.json");
    assert!(place_starter_settings(&path).unwrap());
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
}

#[test]
fn an_existing_settings_file_is_never_changed() {
    let dir = temp_dir("starter-existing");
    for (index, text) in [r#"{"provider": "openai-compatible"}"#, "{ not json", ""]
        .into_iter()
        .enumerate()
    {
        let path = dir.join(format!("ai-{index}.json"));
        fs::write(&path, text).unwrap();
        assert!(!place_starter_settings(&path).unwrap(), "{text:?}");
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
    }
    let path = dir.join("ai.json");
    assert!(place_starter_settings(&path).unwrap());
    let first = fs::read(&path).unwrap();
    assert!(!place_starter_settings(&path).unwrap(), "a second start finds the file");
    assert_eq!(fs::read(&path).unwrap(), first);
}
