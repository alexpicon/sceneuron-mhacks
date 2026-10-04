// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises how AI settings saved for Anthropic are saved again for Gemini: at the first load, so
//! the warning shows once and the Anthropic key leaves the file, which stays readable only by its
//! owner; the Anthropic key is never used; and a move that cannot be saved says so at every start.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use common::TEST_KEY;
use common::retired::{ANTHROPIC_KEY, saved_on_anthropic, warning, written};
use common::sink::RecordingSink;
use sceneuron_ai::{AiSettings, AiSettingsUpdate, Assistant, Credentials, MockProvider, ProviderKind, load_settings};
use serde_json::json;

/// Nothing listens here, so ElevenLabs is never reached.
const CLOSED: &str = "http://127.0.0.1:9";

#[test]
fn the_anthropic_key_is_never_used_and_the_move_is_saved_once() {
    let watching = AiSettings {
        enabled: true,
        consent: true,
        ..AiSettings::default()
    };
    let path = written("retired-assistant", &saved_on_anthropic(&watching));
    let credentials = Credentials {
        gemini: TEST_KEY.to_owned(),
        ..Credentials::default()
    };
    let sink = Arc::new(RecordingSink::default());
    let mock = Arc::new(MockProvider::new(|_| Ok(json!({}))));
    let (assistant, shown) = Assistant::with_elevenlabs_url(path.clone(), mock.connector(), sink, credentials, CLOSED);
    assert_eq!(shown, Some(warning(&path, Some("camera images"))));
    let view = assistant.settings_view();
    assert_eq!(
        (view.provider, view.key_source.as_str(), view.key_hint.as_str()),
        (ProviderKind::Gemini, "environment", "...abcd")
    );
    let text = fs::read_to_string(&path).unwrap();
    for gone in [ANTHROPIC_KEY, "anthropic", "claude"] {
        assert!(!text.contains(gone), "{gone} in {text}");
    }
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    let (saved, shown) = load_settings(&path);
    assert_eq!(
        (saved.provider, saved.consent, saved.enabled, shown),
        (ProviderKind::Gemini, false, false, None)
    );
    let consent: AiSettingsUpdate = serde_json::from_value(json!({"consent": true})).unwrap();
    assistant.update_settings(consent).unwrap();
    assistant.shutdown();
    let (settings, shown) = load_settings(&path);
    assert_eq!(
        (settings.provider, settings.consent, settings.enabled, shown),
        (ProviderKind::Gemini, true, false, None)
    );
    assert_eq!(mock.calls(), 0);
}

#[test]
fn a_move_that_cannot_be_saved_says_so_at_every_start() {
    let path = written(
        "retired-unsaved",
        &json!({"provider": "anthropic", "api_key": ANTHROPIC_KEY}),
    );
    // A folder where the save writes its temporary file makes the save fail.
    fs::create_dir(path.with_file_name("ai.json.tmp")).unwrap();
    let unsaved = format!(
        "{}; saving the change failed, so this shows again at the next start: ",
        warning(&path, None)
    );
    for _ in 0..2 {
        let (settings, shown) = load_settings(&path);
        assert_eq!(settings, AiSettings::default());
        let shown = shown.unwrap();
        assert!(shown.starts_with(&unsaved), "{shown}");
        assert!(!shown.contains(ANTHROPIC_KEY));
    }
    assert!(fs::read_to_string(&path).unwrap().contains(ANTHROPIC_KEY));
}
