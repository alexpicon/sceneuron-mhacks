// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the ElevenLabs text to speech client against a scripted server: the request it
//! sends, MP3 answers, refusals and retries, a voice the account lacks, the account's voice
//! list, and errors that never show the key, even in part.

mod common;

use common::server::{MockServer, Reply};
use common::speech::{EL_KEY, audio_reply, mp3, voices_reply};
use sceneuron_ai::{AiError, TextToSpeech, missing_voice};
use serde_json::json;

#[test]
fn text_to_speech_sends_the_text_and_model_with_the_key_only_in_a_header() {
    let server = MockServer::start(vec![audio_reply()]);
    let client = TextToSpeech::new(&server.url(), EL_KEY);
    assert_eq!(
        client.speak("Wrap it up", "voice123", "eleven_flash_v2_5").unwrap(),
        mp3()
    );
    let sent = &server.requests()[0];
    assert_eq!(sent.method, "POST");
    assert_eq!(sent.path, "/v1/text-to-speech/voice123?output_format=mp3_44100_128");
    assert_eq!(sent.header("xi-api-key"), Some(EL_KEY));
    assert_eq!(sent.header("accept"), Some("audio/mpeg"));
    assert_eq!(
        sent.json(),
        json!({"text": "Wrap it up", "model_id": "eleven_flash_v2_5"})
    );
}

#[test]
fn refusals_say_what_went_wrong_without_the_key() {
    let echo = json!({"detail": {"type": "authentication_error", "code": "invalid_api_key",
        "message": format!("Invalid API key {EL_KEY}")}});
    let server = MockServer::start(vec![
        Reply::json(401, echo),
        Reply::json(200, json!({"not": "audio"})),
        Reply::json(429, json!({"detail": {"message": "too many requests"}})),
        audio_reply(),
    ]);
    let client = TextToSpeech::new(&server.url(), EL_KEY);
    let error = client.speak("Hi", "voice123", "eleven_flash_v2_5").unwrap_err();
    assert!(matches!(error, AiError::Auth(_)), "{error:?}");
    let message = error.to_string();
    assert!(
        message.contains("Invalid API key [redacted]") && !message.contains(EL_KEY),
        "{message}"
    );
    let error = client.speak("Hi", "voice123", "eleven_flash_v2_5").unwrap_err();
    assert_eq!(
        error,
        AiError::Invalid("ElevenLabs answered without MP3 audio".to_owned())
    );
    assert_eq!(
        client.speak("Hi", "voice123", "eleven_flash_v2_5").unwrap(),
        mp3(),
        "a rate limit is retried"
    );
    assert_eq!(server.requests().len(), 4);
    let sent = server.requests().len();
    assert!(matches!(client.speak("Hi", "../voices", "m"), Err(AiError::Config(_))));
    let keyless = TextToSpeech::new(&server.url(), "");
    assert!(matches!(keyless.speak("Hi", "voice123", "m"), Err(AiError::Config(_))));
    assert_eq!(server.requests().len(), sent, "refused before any request");
}

#[test]
fn a_voice_the_account_lacks_is_named_as_such() {
    let missing = json!({"detail": {"type": "not_found", "code": "voice_not_found", "status": "voice_not_found",
        "message": "A voice with the voice_id voice123 was not found."}});
    let legacy = json!({"detail": {"status": "voice_not_found", "message": "not found"}});
    let other = json!({"detail": {"code": "model_not_found", "message": "no such model"}});
    let server = MockServer::start(vec![
        Reply::json(404, missing),
        Reply::json(404, legacy),
        Reply::json(404, other),
    ]);
    let client = TextToSpeech::new(&server.url(), EL_KEY);
    for _ in 0..2 {
        let error = client.speak("Hi", "voice123", "eleven_flash_v2_5").unwrap_err();
        assert_eq!(error, missing_voice("voice123"));
        assert!(error.to_string().contains("not in your ElevenLabs account"), "{error}");
    }
    let error = client.speak("Hi", "voice123", "eleven_flash_v2_5").unwrap_err();
    assert!(matches!(error, AiError::BadRequest(_)), "{error:?}");
    assert_eq!(server.requests().len(), 3, "a missing voice is not retried");
}

#[test]
fn the_voice_list_is_read_with_the_key_in_a_header() {
    let server = MockServer::start(vec![
        voices_reply(&[("abc", "Rachel", "premade"), ("def", "Mine", "cloned")], true),
        Reply::json(401, json!({"detail": {"message": "bad key"}})),
    ]);
    let client = TextToSpeech::new(&server.url(), EL_KEY);
    let list = client.voices().unwrap();
    assert!(list.contains("abc") && list.contains("def") && !list.contains("xyz"));
    assert!(list.has_more);
    assert_eq!(list.replacement().map(|voice| voice.name.as_str()), Some("Rachel"));
    let sent = &server.requests()[0];
    assert_eq!(
        (sent.method.as_str(), sent.path.as_str()),
        ("GET", "/v2/voices?page_size=100")
    );
    assert_eq!(sent.header("xi-api-key"), Some(EL_KEY));
    assert!(matches!(client.voices(), Err(AiError::Auth(_))));
    let keyless = TextToSpeech::new(&server.url(), "");
    assert!(matches!(keyless.voices(), Err(AiError::Config(_))));
    assert_eq!(server.requests().len(), 2);
}

#[test]
fn a_key_echoed_across_the_cut_of_a_long_message_is_still_hidden() {
    for at in [280, 290, 295, 299, 300] {
        let message = format!("{}{EL_KEY} is not valid", "x".repeat(at));
        let server = MockServer::start(vec![Reply::json(401, json!({"detail": {"message": message}}))]);
        let error = TextToSpeech::new(&server.url(), EL_KEY)
            .speak("Hi", "voice123", "eleven_flash_v2_5")
            .unwrap_err()
            .to_string();
        assert!(!error.contains(&EL_KEY[..6]), "a part of the key at {at}: {error}");
    }
}
