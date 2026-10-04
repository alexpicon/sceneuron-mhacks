// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the languages of the subtitle settings: constructed and fictional targets accepted
//! by their ISO 639 code or a private-use tag and saved in their usual case, malformed tags
//! refused, stream captions in a fictional language, a constructed spoken language refused,
//! since speech to text does not hear it, and a file saved with languages an older Sceneuron
//! accepted loading without just those.

mod common;

use common::TEST_KEY;
use common::retired::written;
use sceneuron_ai::{AiSettings, AiSettingsUpdate, ProviderKind, SubtitleSettings, load_settings};
use serde_json::json;

fn updated(value: serde_json::Value) -> Result<SubtitleSettings, String> {
    let update: AiSettingsUpdate = serde_json::from_value(json!({ "subtitles": value })).unwrap();
    AiSettings::default()
        .updated(&update)
        .map(|settings| settings.subtitles)
        .map_err(|error| error.to_string())
}

#[test]
fn constructed_and_fictional_targets_are_accepted_in_their_usual_case() {
    for targets in [
        ["qya", "sjn", "tlh"],
        ["tok", "jbo", "eo"],
        ["ia", "vo", "ldn"],
        ["io", "ie", "nov"],
        ["art-x-dothraki", "art-x-valyrian", "art-x-navi"],
    ] {
        let settings = updated(json!({ "targets": targets })).unwrap();
        assert_eq!(settings.targets, targets);
        settings.validate().unwrap();
    }
    let settings = updated(json!({"targets": ["QYA", "Art-X-Dothraki"], "stream_language": "ART-x-dothraki"})).unwrap();
    assert_eq!(settings.targets, ["qya", "art-x-dothraki"]);
    assert_eq!(
        settings.stream_language, "art-x-dothraki",
        "captions in a fictional language"
    );
}

#[test]
fn malformed_tags_are_refused_with_examples_of_good_ones() {
    for bad in [
        "x-dothraki",
        "i-klingon",
        "art-x",
        "art-x-highvalyrian",
        "art-x-na'vi",
        "en-a-x-b",
        "en-US-GB",
    ] {
        let refused = updated(json!({ "targets": [bad] })).unwrap_err();
        let expected = format!("{bad:?} is not a language code such as es, pt-BR, zh-Hans, or art-x-dothraki");
        assert!(refused.ends_with(&expected), "{refused}");
    }
}

#[test]
fn a_constructed_spoken_language_is_refused_since_speech_to_text_does_not_hear_it() {
    let refused = updated(json!({"language": "qya"})).unwrap_err();
    assert!(
        refused.ends_with(
            "speech to text does not hear Quenya (Elvish, qya); choose it as a language to translate into instead"
        ),
        "{refused}"
    );
    for constructed in ["tlh", "eo", "art"] {
        assert!(updated(json!({ "language": constructed })).is_err(), "{constructed}");
    }
    let natural = updated(json!({"language": "es", "targets": ["qya"]})).unwrap();
    assert_eq!(
        (natural.language.as_str(), natural.targets.as_slice()),
        ("es", ["qya".to_owned()].as_slice())
    );
}

#[test]
fn languages_an_older_sceneuron_saved_are_dropped_on_loading_and_everything_else_is_kept() {
    let saved = json!({
        "provider": "openai-compatible",
        "base_url": "http://127.0.0.1:8000/v1",
        "vision_model": "local-vision",
        "planner_model": "local-planner",
        "api_key": TEST_KEY,
        "elevenlabs_api_key": TEST_KEY,
        "subtitles": {
            "enabled": true,
            "consent": true,
            "language": "eo",
            "targets": ["es", "en-x"],
            "stream_language": "en-x",
            "translation_model": "local-translator",
        },
    });
    let path = written("subtitles-older-languages", &saved);
    let (settings, warning) = load_settings(&path);
    let kept = (
        settings.provider,
        settings.base_url.as_str(),
        settings.vision_model.as_str(),
        settings.planner_model.as_str(),
        settings.api_key.as_str(),
        settings.elevenlabs_api_key.as_str(),
    );
    let expected = (
        ProviderKind::OpenAiCompatible,
        "http://127.0.0.1:8000/v1",
        "local-vision",
        "local-planner",
        TEST_KEY,
        TEST_KEY,
    );
    assert_eq!(kept, expected, "everything but the subtitle languages is kept");
    let subtitles = &settings.subtitles;
    assert_eq!(subtitles.targets, ["es"]);
    assert_eq!(
        (subtitles.language.as_str(), subtitles.stream_language.as_str()),
        ("", "")
    );
    assert!(subtitles.enabled && subtitles.consent);
    assert_eq!(subtitles.translation_model, "local-translator");
    let expected = format!(
        "cannot use all the subtitle settings in {}: \"en-x\" is not a language code, so subtitles no longer \
         translate into it; captions are in the spoken language instead; speech to text does not hear Esperanto \
         (eo), so subtitles detect the spoken language",
        path.display()
    );
    assert_eq!(warning.as_deref(), Some(expected.as_str()));
    assert!(!expected.contains(TEST_KEY));
}

#[test]
fn targets_kept_on_loading_take_their_usual_case_and_keep_their_captions() {
    let subtitles = json!({"targets": ["art-x-Navi", "de-1", "en-GB-oed"], "stream_language": "art-x-Navi"});
    let path = written(
        "subtitles-older-case",
        &json!({ "api_key": TEST_KEY, "subtitles": subtitles }),
    );
    let (settings, warning) = load_settings(&path);
    assert_eq!(settings.api_key, TEST_KEY);
    assert_eq!(settings.subtitles.targets, ["art-x-navi"]);
    assert_eq!(settings.subtitles.stream_language, "art-x-navi");
    let warning = warning.unwrap();
    for dropped in [
        "\"de-1\" is not a language code",
        "\"en-GB-oed\" is not a language code",
    ] {
        assert!(warning.contains(dropped), "{warning}");
    }
    assert!(!warning.contains("captions"), "{warning}");
}
