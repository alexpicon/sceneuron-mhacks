// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises voice control's mode while subtitles keep the stream open: in push-to-talk mode only
//! speech said while push to talk is held is a command, so words that start with the wake word
//! but were not held stay in the subtitles, raise nothing, and never reach the event log; in
//! always-on mode the wake word still addresses Sceneuron.

mod common;

use common::subtitles::{held, lines, partial, said, status};
use common::voice::meeting_engine;
use sceneuron_core::{Engine, VoiceMode};

const PROBE: &str = "Sceneuron is honestly my favorite tool";

/// The meeting and poker packs with voice on in `mode`, and subtitles on in English.
fn subtitled(mode: VoiceMode) -> Engine {
    let mut engine = meeting_engine(mode);
    engine.set_subtitle_status(status("en", &[]), 0);
    engine
}

fn details(engine: &Engine) -> Vec<String> {
    let events = engine.snapshot(0).events.into_iter();
    events.map(|event| event.detail).collect()
}

#[test]
fn in_push_to_talk_mode_the_wake_word_alone_is_not_a_command() {
    let mut engine = subtitled(VoiceMode::PushToTalk);
    let before = details(&engine);
    engine.hear(&partial("Sceneuron is"), 100);
    assert_eq!(engine.subtitles_view().live, "Sceneuron is", "live words stay");
    engine.hear(&said(PROBE), 200);
    engine.hear(&said("Sceneuron, wrap it up"), 300);
    assert_eq!(lines(&engine), [PROBE, "Sceneuron, wrap it up"]);
    assert_eq!(engine.raised_card("meeting"), None, "nothing is raised");
    assert_eq!(engine.voice_view().heard, None, "nothing is shown as heard");
    assert_eq!(details(&engine), before, "nothing reaches the event log");
}

#[test]
fn in_push_to_talk_mode_held_speech_is_a_command_and_stays_out_of_the_subtitles() {
    let mut engine = subtitled(VoiceMode::PushToTalk);
    engine.hear(&said("Good morning."), 100);
    engine.hear(&held("Sceneuron, wrap it up"), 200);
    assert_eq!(engine.raised_card("meeting"), Some("wrap-up"));
    assert_eq!(lines(&engine), ["Good morning."]);
    assert!(
        details(&engine).contains(&"heard: \"Sceneuron, wrap it up\"".to_owned()),
        "a command is logged as before"
    );
}

#[test]
fn in_always_on_mode_the_wake_word_addresses_sceneuron() {
    let mut engine = subtitled(VoiceMode::AlwaysOn);
    engine.hear(&said("Good morning."), 100);
    engine.hear(&said("Sceneuron, wrap it up"), 200);
    assert_eq!(engine.raised_card("meeting"), Some("wrap-up"));
    assert_eq!(lines(&engine), ["Good morning."]);
}
