// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises push-to-talk on the engine, which lets go by itself after a minute without news,
//! and the voice status the host reports, which reaches the view and the event log.

mod common;

use common::last_event;
use common::voice::{meeting_engine, on, pair};
use sceneuron_core::{Engine, VoiceMode, VoiceStatus, default_project};

#[test]
fn push_to_talk_is_held_until_let_go_or_for_a_minute_without_news() {
    let mut engine = meeting_engine(VoiceMode::PushToTalk);
    assert!(engine.set_talking(true, 1_000).changed);
    assert!(engine.talking() && engine.voice_view().talking);
    assert!(
        !engine.set_talking(true, 30_000).changed,
        "pressing again only extends the hold"
    );
    assert!(!engine.tick(80_000).changed);
    assert!(engine.tick(90_000).changed);
    assert!(!engine.talking());
    assert_eq!(
        last_event(&engine, 90_000),
        pair("voice", "push to talk let go after a minute without news")
    );
    engine.set_talking(true, 100_000);
    assert!(engine.set_talking(false, 101_000).changed);
    assert!(!engine.tick(200_000).changed);
}

#[test]
fn the_status_reaches_the_view_and_turning_on_is_logged() {
    let mut engine = Engine::new(default_project(), 0).unwrap();
    let view = engine.voice_view();
    assert!(!view.enabled && !view.listening && !view.talking && view.heard.is_none());
    let status = VoiceStatus {
        listening: true,
        message: "listening".to_owned(),
        ..on(VoiceMode::AlwaysOn)
    };
    assert!(engine.set_voice_status(status.clone(), 10).changed);
    assert!(!engine.set_voice_status(status, 20).changed);
    assert_eq!(last_event(&engine, 20), pair("voice", "voice control turned on"));
    let view = engine.snapshot(20).voice;
    assert_eq!(
        (view.enabled, view.mode, view.listening),
        (true, VoiceMode::AlwaysOn, true)
    );
    let json = serde_json::to_value(engine.snapshot(20).voice).unwrap();
    assert_eq!(json["mode"], "always_on");
}
