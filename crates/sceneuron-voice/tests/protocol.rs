// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the Scribe realtime messages: the session URL and its query parameters, the audio
//! message, and parsing what the server sends, including how each error type is treated.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sceneuron_voice::session::Kind;
use sceneuron_voice::{
    ApiKey, DEFAULT_ENDPOINT, ErrorKind, Mode, ServerMessage, VoiceConfig, audio_message, parse_server_message,
    session_url,
};
use serde_json::{Value, json};

fn config(mode: Mode) -> VoiceConfig {
    VoiceConfig {
        enabled: true,
        consent: true,
        mode,
        keyterms: vec![
            "Sceneuron".to_owned(),
            "Let's take this".to_owned(),
            "Wrap it up".to_owned(),
        ],
        api_key: ApiKey::new("sk_secret_key_value"),
        ..VoiceConfig::default()
    }
}

#[test]
fn turns_commit_by_hand_and_carry_every_keyterm() {
    let url = session_url(&config(Mode::PushToTalk), Kind::Turn);
    assert_eq!(
        url,
        format!(
            "{DEFAULT_ENDPOINT}?model_id=scribe_v2_realtime&audio_format=pcm_16000&commit_strategy=manual\
             &include_language_detection=true&keyterms=Sceneuron&keyterms=Let%27s%20take%20this\
             &keyterms=Wrap%20it%20up"
        )
    );
    assert!(url.starts_with("wss://api.elevenlabs.io/v1/speech-to-text/realtime?"));
    let always_on_turn = session_url(&config(Mode::AlwaysOn), Kind::Turn);
    assert_eq!(always_on_turn, url, "held speech commits by hand in either mode");
}

#[test]
fn streams_use_voice_activity_and_a_set_language() {
    let mut config = config(Mode::AlwaysOn);
    config.language = " es ".to_owned();
    config.keyterms.clear();
    config.endpoint = "wss://api.eu.residency.elevenlabs.io/v1/speech-to-text/realtime?x=1".to_owned();
    let url = session_url(&config, Kind::Stream);
    assert!(
        url.ends_with("?x=1&model_id=scribe_v2_realtime&audio_format=pcm_16000&commit_strategy=vad&language_code=es")
    );
    config.language = "  ".to_owned();
    let detecting = session_url(&config, Kind::Stream);
    assert!(
        detecting.ends_with("&commit_strategy=vad&include_language_detection=true"),
        "a language left to detection is asked for: {detecting}"
    );
}

#[test]
fn the_key_never_appears_in_the_url_or_in_debug_output() {
    let config = config(Mode::PushToTalk);
    assert!(!session_url(&config, Kind::Turn).contains("sk_secret_key_value"));
    let shown = format!("{config:?}");
    assert!(
        !shown.contains("sk_secret_key_value") && shown.contains("ApiKey(<set>)"),
        "{shown}"
    );
    assert_eq!(format!("{:?}", ApiKey::new("  ")), "ApiKey(none)");
    assert_eq!(config.api_key.redact("bad key sk_secret_key_value!"), "bad key [key]!");
}

#[test]
fn audio_messages_carry_base64_pcm_and_the_sample_rate() {
    let message: Value = serde_json::from_str(&audio_message(&[1, 2, 3, 250], true)).unwrap();
    assert_eq!(message["message_type"], "input_audio_chunk");
    assert_eq!(
        STANDARD.decode(message["audio_base_64"].as_str().unwrap()).unwrap(),
        vec![1, 2, 3, 250]
    );
    assert_eq!(
        (message["commit"].clone(), message["sample_rate"].clone()),
        (json!(true), json!(16000))
    );
    let empty: Value = serde_json::from_str(&audio_message(&[], false)).unwrap();
    assert_eq!(
        (empty["audio_base_64"].clone(), empty["commit"].clone()),
        (json!(""), json!(false))
    );
}

#[test]
fn transcripts_and_session_start_are_parsed() {
    let parse = |value: Value| parse_server_message(&value.to_string());
    assert_eq!(
        parse(json!({"message_type": "session_started", "session_id": "s1", "config": {}})),
        Some(ServerMessage::SessionStarted)
    );
    assert_eq!(
        parse(json!({"message_type": "partial_transcript", "text": "wrap"})),
        Some(ServerMessage::Partial("wrap".to_owned()))
    );
    assert_eq!(
        parse(json!({"message_type": "committed_transcript", "text": "wrap it up"})),
        Some(ServerMessage::Committed("wrap it up".to_owned()))
    );
    assert_eq!(
        parse(json!({"message_type": "committed_transcript_with_timestamps", "text": "x", "words": []})),
        Some(ServerMessage::Other("committed_transcript_with_timestamps".to_owned()))
    );
    let detected = json!({"message_type": "committed_transcript_with_timestamps", "text": "x",
        "language_code": " eng ", "words": []});
    assert_eq!(parse(detected), Some(ServerMessage::Language("eng".to_owned())));
    let blank = json!({"message_type": "committed_transcript_with_timestamps", "text": "x", "language_code": ""});
    assert_eq!(
        parse(blank),
        Some(ServerMessage::Other("committed_transcript_with_timestamps".to_owned()))
    );
    assert_eq!(parse_server_message("not json"), None);
    assert_eq!(parse(json!({"text": "no type"})), None);
}

#[test]
fn every_documented_error_type_has_a_meaning() {
    let kinds = [
        ("auth_error", ErrorKind::Fatal),
        ("unaccepted_terms", ErrorKind::Fatal),
        ("quota_exceeded", ErrorKind::Fatal),
        ("session_time_limit_exceeded", ErrorKind::Renew),
        ("commit_throttled", ErrorKind::Warning),
        ("insufficient_audio_activity", ErrorKind::Warning),
        ("error", ErrorKind::Retry),
        ("rate_limited", ErrorKind::Retry),
        ("queue_overflow", ErrorKind::Retry),
        ("resource_exhausted", ErrorKind::Retry),
        ("transcriber_error", ErrorKind::Retry),
        ("input_error", ErrorKind::Retry),
        ("invalid_request", ErrorKind::Retry),
        ("chunk_size_exceeded", ErrorKind::Retry),
    ];
    for (code, kind) in kinds {
        let parsed = parse_server_message(&json!({"message_type": code, "error": "details"}).to_string());
        let expected = ServerMessage::Error {
            kind,
            code: code.to_owned(),
            message: "details".to_owned(),
        };
        assert_eq!(parsed, Some(expected), "{code}");
    }
}

#[test]
fn warnings_are_errors_that_do_not_end_the_session_and_long_text_is_kept_for_redaction() {
    let warning = parse_server_message(&json!({"message_type": "warning", "warning": "slow"}).to_string());
    assert_eq!(
        warning,
        Some(ServerMessage::Error {
            kind: ErrorKind::Warning,
            code: "warning".to_owned(),
            message: "slow".to_owned(),
        })
    );
    let long = "é".repeat(500);
    let parsed = parse_server_message(&json!({"message_type": "error", "error": long}).to_string());
    let Some(ServerMessage::Error { message, .. }) = parsed else {
        panic!("not an error: {parsed:?}");
    };
    assert_eq!(message, long, "cut only once the key is out of it");
}
