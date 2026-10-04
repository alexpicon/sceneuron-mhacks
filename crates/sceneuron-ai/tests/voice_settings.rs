// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the voice settings: their defaults and JSON shape in `ai.json`, every validation
//! rule, partial updates with consent, the editor's view with prices and without the key, the
//! status reported to the engine, from the settings and what the voice input is doing.

mod common;

use common::temp_dir;
use sceneuron_ai::{
    AiSettings, AiSettingsUpdate, DEFAULT_STT_MODEL, DEFAULT_TTS_MODEL, DEFAULT_VOICE_ID, VoiceSettings, load_settings,
    save_settings,
};
use sceneuron_core::VoiceMode;
use sceneuron_voice::VoiceStatus as InputStatus;
use serde_json::json;

fn update(value: serde_json::Value) -> AiSettingsUpdate {
    serde_json::from_value(value).unwrap()
}

fn refusal(edit: impl FnOnce(&mut VoiceSettings)) -> String {
    let mut settings = AiSettings::default();
    edit(&mut settings.voice);
    settings.validate().unwrap_err().to_string()
}

#[test]
fn voice_starts_off_with_documented_defaults() {
    let voice = VoiceSettings::default();
    assert!(!voice.enabled && !voice.consent && !voice.speak);
    assert_eq!(voice.mode, VoiceMode::PushToTalk);
    assert_eq!((voice.wake_word.as_str(), voice.language.as_str()), ("Sceneuron", ""));
    assert_eq!(
        (
            voice.voice_id.as_str(),
            voice.tts_model.as_str(),
            voice.stt_model.as_str()
        ),
        ("JBFqnCBsd6RMkjVDRZzb", "eleven_flash_v2_5", "scribe_v2_realtime")
    );
    assert_eq!(
        (DEFAULT_VOICE_ID, DEFAULT_TTS_MODEL, DEFAULT_STT_MODEL),
        (
            voice.voice_id.as_str(),
            voice.tts_model.as_str(),
            voice.stt_model.as_str()
        )
    );
    let json = serde_json::to_value(&voice).unwrap();
    assert_eq!(
        json,
        json!({"enabled": false, "consent": false, "mode": "push_to_talk", "wake_word": "Sceneuron",
            "language": "", "speak": false, "voice_id": "JBFqnCBsd6RMkjVDRZzb",
            "tts_model": "eleven_flash_v2_5", "stt_model": "scribe_v2_realtime"})
    );
    AiSettings::default().validate().unwrap();
}

#[test]
fn older_settings_files_load_with_voice_off_and_new_ones_round_trip() {
    let dir = temp_dir("voice-settings");
    let path = dir.join("ai.json");
    std::fs::write(&path, r#"{"enabled": false, "vision_model": "gemini-3.5-flash"}"#).unwrap();
    let (loaded, warning) = load_settings(&path);
    assert_eq!(
        (loaded.voice, loaded.elevenlabs_api_key.as_str(), warning),
        (VoiceSettings::default(), "", None)
    );
    let mut settings = AiSettings::default();
    settings.voice.speak = true;
    settings.voice.mode = VoiceMode::AlwaysOn;
    settings.elevenlabs_api_key = "el-key-0123456789".to_owned();
    save_settings(&path, &settings).unwrap();
    assert_eq!(load_settings(&path).0, settings);
}

#[test]
fn every_value_is_checked() {
    assert_eq!(
        refusal(|voice| voice.enabled = true),
        "confirm that microphone audio may be sent to ElevenLabs first"
    );
    for wake in ["", "x", "Sceneuron!", "a very long wake word indeed that goes on", "--"] {
        assert!(
            refusal(|voice| voice.wake_word = wake.to_owned()).contains("wake word"),
            "{wake:?}"
        );
    }
    for language in ["EN", "e", "engl", "en-US", "1a"] {
        assert!(
            refusal(|voice| voice.language = language.to_owned()).contains("language"),
            "{language}"
        );
    }
    for id in ["", "has space", "../x", "x".repeat(65).as_str()] {
        assert!(
            refusal(|voice| voice.voice_id = id.to_owned()).contains("voice id"),
            "{id:?}"
        );
    }
    assert!(refusal(|voice| voice.tts_model = "Eleven Flash".to_owned()).contains("tts_model"));
    assert!(refusal(|voice| voice.stt_model = String::new()).contains("stt_model"));
    let good = AiSettings {
        voice: VoiceSettings {
            enabled: true,
            consent: true,
            mode: VoiceMode::AlwaysOn,
            wake_word: "Hey Neuron".to_owned(),
            language: "spa".to_owned(),
            ..VoiceSettings::default()
        },
        ..AiSettings::default()
    };
    good.validate().unwrap();
}

#[test]
fn updates_change_only_what_they_name_and_withdrawing_consent_stops_listening() {
    let settings = AiSettings::default();
    let refused = settings
        .updated(&update(json!({"voice": {"enabled": true}})))
        .unwrap_err();
    assert!(refused.to_string().contains("microphone"), "{refused}");
    let on = settings
        .updated(&update(
            json!({"voice": {"consent": true, "enabled": true, "wake_word": "  Neuron "}}),
        ))
        .unwrap();
    assert!(on.voice.enabled && on.voice.consent);
    assert_eq!(on.voice.wake_word, "Neuron");
    assert_eq!(on.voice.voice_id, DEFAULT_VOICE_ID);
    let off = on.updated(&update(json!({"voice": {"consent": false}}))).unwrap();
    assert!(!off.voice.enabled && !off.voice.consent);
    let keyed = settings
        .updated(&update(json!({"elevenlabs_api_key": " el-key-0123456789 "})))
        .unwrap();
    assert_eq!(keyed.elevenlabs_api_key, "el-key-0123456789");
    let cleared = keyed.updated(&update(json!({"elevenlabs_api_key": ""}))).unwrap();
    assert_eq!(cleared.elevenlabs_api_key, "");
    assert!(serde_json::from_value::<AiSettingsUpdate>(json!({"voice": {"volume": 3}})).is_err());
    let shown = format!("{:?}", update(json!({"elevenlabs_api_key": "el-secret-key"})));
    assert!(!shown.contains("el-secret-key"), "{shown}");
}

#[test]
fn the_view_has_prices_and_a_key_hint_but_never_the_key() {
    let settings = AiSettings {
        elevenlabs_api_key: "el-secret-key-9876".to_owned(),
        ..AiSettings::default()
    };
    let view = settings.view();
    assert!(view.has_elevenlabs_key);
    assert_eq!(
        (view.elevenlabs_key_hint.as_str(), view.elevenlabs_key_source.as_str()),
        ("...9876", "editor")
    );
    assert_eq!(
        (view.voice.tts_price_per_1k_chars, view.voice.stt_price_per_hour),
        (0.04, 0.39)
    );
    assert!(view.voice.tts_price_listed && view.voice.stt_price_listed);
    assert_eq!(view.voice.keyterm_price_per_hour, 0.05);
    let json = serde_json::to_value(&view).unwrap();
    assert_eq!(json["voice"]["voice_id"], DEFAULT_VOICE_ID);
    assert_eq!(json["voice"]["tts_price_per_1k_chars"], 0.04);
    assert!(!json.to_string().contains("el-secret-key"));
    assert!(!format!("{settings:?}").contains("el-secret-key"));
}

#[test]
fn the_status_says_what_voice_control_needs_or_what_the_input_is_doing() {
    let mut voice = VoiceSettings::default();
    let off = voice.status(true, &InputStatus::Off);
    assert_eq!((off.enabled, off.message.as_str()), (false, "voice control is off"));
    voice.enabled = true;
    voice.consent = true;
    voice.mode = VoiceMode::AlwaysOn;
    voice.wake_word = "Neuron".to_owned();
    let keyless = voice.status(false, &InputStatus::NeedsKey);
    assert_eq!(keyless.message, "add an ElevenLabs key to use voice control");
    let ready = voice.status(true, &InputStatus::Connecting);
    assert_eq!(
        (ready.enabled, ready.listening, ready.mode),
        (true, false, VoiceMode::AlwaysOn)
    );
    assert_eq!(ready.message, "connecting to ElevenLabs speech to text");
    assert_eq!(ready.wake_word, "Neuron");
    let listening = voice.status(true, &InputStatus::Listening);
    assert!(listening.listening);
    assert_eq!(listening.message, "listening");
    assert!(
        !voice.status(false, &InputStatus::Listening).listening,
        "nothing listens without a key"
    );
    let stopped = InputStatus::Stopped {
        message: "ElevenLabs refused the API key (HTTP 401)".to_owned(),
    };
    assert_eq!(
        voice.status(true, &stopped).message,
        "stopped: ElevenLabs refused the API key (HTTP 401)"
    );
}
