// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the voice spend estimates: the list price of every current text to speech and
//! realtime speech to text model, the keyterm surcharge, the highest known rate for models
//! without a list price, and the view that tells the editor which applies.

use sceneuron_ai::{
    AiSettings, VoiceSettings, VoiceUsage, keyterm_rate_per_hour, stt_price_per_hour, stt_rate_per_hour,
    tts_price_per_1k_chars, tts_rate_per_1k_chars, voice_cost_usd,
};

#[test]
fn every_current_model_has_its_list_price() {
    for model in [
        "eleven_flash_v2_5",
        "eleven_flash_v2",
        "eleven_turbo_v2_5",
        "eleven_turbo_v2",
        "eleven_v4_turbo",
        "eleven_v3_conversational",
    ] {
        assert_eq!(tts_price_per_1k_chars(model), Some(0.04), "{model}");
    }
    for model in ["eleven_multilingual_v2", "eleven_v3", "eleven_v4"] {
        assert_eq!(tts_price_per_1k_chars(model), Some(0.08), "{model}");
    }
    assert_eq!(stt_price_per_hour("scribe_v2_realtime"), Some(0.39));
    assert_eq!(keyterm_rate_per_hour(), 0.05);
}

#[test]
fn a_model_without_a_list_price_counts_at_the_highest_known_rate() {
    assert_eq!(tts_price_per_1k_chars("eleven_v9_ultra"), None);
    assert_eq!(tts_rate_per_1k_chars("eleven_v9_ultra"), 0.08);
    assert_eq!(stt_rate_per_hour("scribe_v9_realtime"), 0.39);
    let usage = VoiceUsage {
        characters: 1_000,
        audio_seconds: 3_600.0,
        keyterm_seconds: 0.0,
    };
    let cost = voice_cost_usd(&usage, "eleven_v9_ultra", "scribe_v9_realtime");
    assert!((cost - (0.08 + 0.39)).abs() < 1e-12, "never counted as free: {cost}");
    let mut settings = VoiceSettings {
        tts_model: "eleven_v9_ultra".to_owned(),
        ..VoiceSettings::default()
    };
    let view = settings.view();
    assert_eq!(
        (
            view.tts_price_per_1k_chars,
            view.tts_price_listed,
            view.stt_price_listed
        ),
        (0.08, false, true)
    );
    settings.tts_model = "eleven_flash_v2_5".to_owned();
    assert!(settings.view().tts_price_listed);
}

#[test]
fn spend_adds_characters_seconds_and_the_keyterm_surcharge() {
    let usage = VoiceUsage {
        characters: 500,
        audio_seconds: 1_800.0,
        keyterm_seconds: 720.0,
    };
    let cost = voice_cost_usd(&usage, "eleven_flash_v2_5", "scribe_v2_realtime");
    assert!((cost - (0.02 + 0.195 + 0.01)).abs() < 1e-12, "{cost}");
    let odd = VoiceUsage {
        characters: 0,
        audio_seconds: f64::NAN,
        keyterm_seconds: f64::INFINITY,
    };
    assert_eq!(voice_cost_usd(&odd, "eleven_flash_v2_5", "scribe_v2_realtime"), 0.0);
    let negative = VoiceUsage {
        audio_seconds: -10.0,
        ..VoiceUsage::default()
    };
    assert_eq!(
        voice_cost_usd(&negative, "eleven_flash_v2_5", "scribe_v2_realtime"),
        0.0
    );
}

#[test]
fn the_ai_settings_view_carries_the_counted_rates() {
    let json = serde_json::to_value(AiSettings::default().view()).unwrap();
    let voice = &json["voice"];
    assert_eq!(
        (
            &voice["tts_price_per_1k_chars"],
            &voice["stt_price_per_hour"],
            &voice["keyterm_price_per_hour"]
        ),
        (&0.04.into(), &0.39.into(), &0.05.into())
    );
    assert_eq!(
        (&voice["tts_price_listed"], &voice["stt_price_listed"]),
        (&true.into(), &true.into())
    );
}
