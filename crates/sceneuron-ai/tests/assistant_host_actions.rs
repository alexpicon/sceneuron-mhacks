// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the voice requests the assistant carries out itself, with no model call: a look needs
//! scene understanding, the project's policy, and a provider the settings can build; subtitles
//! need their consent, and a language goes first among the targets and turns agreed-to subtitles
//! on. assistant_commands.rs covers words for the AI.

mod common;

use std::sync::Arc;

use common::assistant::{answering, festive, open};
use common::sink::RecordingSink;
use common::temp_dir;
use sceneuron_ai::{AiError, AiSettings, AiSettingsUpdate, Assistant, Connector, Credentials};
use sceneuron_core::{AiPolicy, VoiceAction, VoiceStep, VoiceTask, default_project};
use serde_json::{Value, json};

fn act(action: VoiceAction) -> VoiceTask {
    VoiceTask::Act { action }
}

fn update(value: Value) -> AiSettingsUpdate {
    serde_json::from_value(value).unwrap()
}

/// What a host task did, in words.
fn done(assistant: &Assistant, task: VoiceTask) -> String {
    match assistant.carry_out(&default_project(), &task).unwrap().steps.as_slice() {
        [VoiceStep::Done(what)] => what.clone(),
        other => panic!("expected one step done here, got {other:?}"),
    }
}

#[test]
fn a_look_needs_scene_understanding() {
    let mock = answering(vec![festive()]);
    let (assistant, _) = open(&temp_dir("voice-look").join("ai.json"), &mock);
    let look = || done(&assistant, act(VoiceAction::Look));
    assert_eq!(
        look(),
        "the vision model cannot look: turn on scene understanding in the AI tab"
    );
    let on = json!({"api_key": common::TEST_KEY, "consent": true, "enabled": true});
    assistant.update_settings(update(on)).unwrap();
    assert_eq!(look(), "asked the vision model to look at the camera");
    let mut off = default_project();
    off.ai = AiPolicy {
        enabled: false,
        ..AiPolicy::default()
    };
    let reply = assistant.carry_out(&off, &act(VoiceAction::Look)).unwrap();
    let refused = "the vision model cannot look: scene understanding is off for this project";
    assert_eq!(reply.steps, [VoiceStep::Done(refused.to_owned())]);
    assert_eq!(mock.calls(), 0, "a look is the director's");
    assistant.shutdown();
}

#[test]
fn a_look_names_what_the_provider_still_needs() {
    let keyless: Connector =
        Arc::new(|_: &AiSettings| Err(AiError::Config("add a Gemini API key in the AI settings".to_owned())));
    let path = temp_dir("voice-look-keyless").join("ai.json");
    let sink = Arc::new(RecordingSink::default());
    let (assistant, _) = Assistant::new(path, keyless, sink, Credentials::default());
    assistant
        .update_settings(update(json!({"consent": true, "enabled": true})))
        .unwrap();
    assert_eq!(
        done(&assistant, act(VoiceAction::Look)),
        "the vision model cannot look: add a Gemini API key in the AI settings"
    );
    assistant.shutdown();
}

#[test]
fn subtitles_need_their_consent_and_go_into_a_language_first() {
    let mock = answering(vec![festive()]);
    let (assistant, _) = open(&temp_dir("voice-subtitles").join("ai.json"), &mock);
    let on = || done(&assistant, act(VoiceAction::Subtitles { enabled: true }));
    assert_eq!(
        on(),
        "subtitles stay off: confirm that microphone audio may be sent to ElevenLabs for subtitles first"
    );
    let into = |language: &str| {
        let language = language.to_owned();
        done(&assistant, act(VoiceAction::SubtitleLanguage { language }))
    };
    assert_eq!(
        into("es"),
        "subtitles will translate into Spanish (es) first; to show them, confirm in the AI tab that microphone \
         audio may be sent to ElevenLabs for subtitles"
    );
    let agreed = json!({"subtitles": {"consent": true, "targets": ["es", "fr", "ja"]}});
    assistant.update_settings(update(agreed)).unwrap();
    assert_eq!(
        into("ja"),
        "subtitles translate into Japanese (ja) first; they show once an ElevenLabs key is added in the AI tab"
    );
    let subtitles = assistant.settings_view().subtitles.settings;
    assert!(subtitles.enabled, "agreed to, so they turn on");
    assert_eq!(subtitles.targets, ["ja", "es", "fr"]);
    assert_eq!(
        into("tlh"),
        "subtitles translate into Klingon (tlh) first; they show once an ElevenLabs key is added in the AI tab"
    );
    let targets = assistant.settings_view().subtitles.settings.targets;
    assert_eq!(targets, ["tlh", "ja", "es"], "at most three");
    assert_eq!(
        into("no such language!"),
        "\"no such language!\" is not a language code"
    );
    assert_eq!(on(), "subtitles were already on");
    let off = done(&assistant, act(VoiceAction::Subtitles { enabled: false }));
    assert_eq!(off, "turned subtitles off");
    assistant.shutdown();
}
