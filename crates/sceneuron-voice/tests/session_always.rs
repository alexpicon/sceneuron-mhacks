// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the session policy in always-on mode with made-up times: the stream opens at once
//! and carries everything, leaves the wake word to the engine, caps audio waiting for a
//! reconnect, and reports usage as it adds up. Held speech is in `session_turns.rs`.

mod common;

use common::{config, count, final_text, heard, pcm, sends, statuses, usage};
use sceneuron_voice::session::{Action, CHUNK_BYTES, MAX_STREAM_BYTES, Session};
use sceneuron_voice::{Heard, Mode, ServerMessage, VoiceConfig, VoiceStatus};

fn listening() -> Session {
    let mut session = Session::new(config(Mode::AlwaysOn), 0);
    let actions = session.take_actions();
    assert_eq!(count(&actions, &Action::Connect), 1);
    assert_eq!(statuses(&actions), vec![VoiceStatus::Connecting]);
    session.connected(100);
    assert_eq!(statuses(&session.take_actions()), vec![VoiceStatus::Listening]);
    session
}

#[test]
fn audio_streams_without_commits() {
    let mut session = listening();
    assert!(session.wants_audio());
    session.audio(&pcm(350), 200);
    session.audio(&pcm(50), 250);
    assert_eq!(sends(&session.take_actions()), vec![(CHUNK_BYTES, false); 4]);
}

#[test]
fn a_stream_leaves_the_wake_word_to_the_engine() {
    let mut session = listening();
    session.message(ServerMessage::Partial("Sceneuron".to_owned()), 200);
    session.message(ServerMessage::Committed("Sceneuron, wrap it up".to_owned()), 300);
    session.message(ServerMessage::Committed("let's take this offline".to_owned()), 400);
    let partial = Heard {
        text: "Sceneuron".to_owned(),
        is_final: false,
        addressed: false,
    };
    assert_eq!(
        heard(&session.take_actions()),
        vec![
            partial,
            final_text("Sceneuron, wrap it up", false),
            final_text("let's take this offline", false)
        ]
    );
}

#[test]
fn a_stream_passes_on_the_language_detected_only_while_it_is_open() {
    let mut session = listening();
    session.message(ServerMessage::Committed("Good morning".to_owned()), 200);
    session.message(ServerMessage::Language("eng".to_owned()), 250);
    let languages = |actions: &[Action]| -> Vec<String> {
        let events = actions.iter().filter_map(|action| match action {
            Action::Emit(sceneuron_voice::VoiceEvent::Language(code)) => Some(code.clone()),
            _ => None,
        });
        events.collect()
    };
    assert_eq!(languages(&session.take_actions()), ["eng"]);
    session.failed(sceneuron_voice::session::Failure::Renew, 300);
    session.take_actions();
    session.message(ServerMessage::Language("spa".to_owned()), 350);
    assert!(
        languages(&session.take_actions()).is_empty(),
        "a connection let go says nothing"
    );
}

#[test]
fn audio_waiting_for_a_reconnect_keeps_only_the_latest_seconds() {
    let mut session = listening();
    session.failed(sceneuron_voice::session::Failure::Renew, 200);
    assert_eq!(
        count(&session.take_actions(), &Action::Connect),
        1,
        "a renewed session starts at once"
    );
    session.audio(&pcm(8_000), 300);
    session.connected(400);
    let sent: usize = sends(&session.take_actions()).iter().map(|(bytes, _)| bytes).sum();
    assert_eq!(sent, MAX_STREAM_BYTES);
}

#[test]
fn usage_is_reported_every_ten_seconds_of_audio() {
    let mut session = listening();
    for second in 0..25 {
        session.audio(&pcm(1_000), 200 + second * 1_000);
    }
    let actions = session.take_actions();
    let reports: Vec<f64> = actions
        .iter()
        .filter_map(|action| match action {
            Action::Emit(sceneuron_voice::VoiceEvent::Usage { audio_seconds, .. }) => Some(*audio_seconds),
            _ => None,
        })
        .collect();
    assert_eq!(reports.len(), 2);
    assert!((usage(&actions) - 20.0).abs() < 1e-9);
    session.shutdown();
    let actions = session.take_actions();
    assert_eq!(count(&actions, &Action::Disconnect), 1);
    assert!((usage(&actions) - 5.0).abs() < 1e-9, "shutting down reports the rest");
}

#[test]
fn switching_to_push_to_talk_closes_the_stream() {
    let mut session = listening();
    session.configure(
        VoiceConfig {
            mode: Mode::PushToTalk,
            ..config(Mode::AlwaysOn)
        },
        200,
    );
    let actions = session.take_actions();
    assert_eq!(
        (count(&actions, &Action::Disconnect), count(&actions, &Action::Connect)),
        (1, 0)
    );
    assert_eq!(statuses(&actions), vec![VoiceStatus::Ready]);
    assert!(!session.wants_audio());
}

#[test]
fn new_keyterms_reopen_the_stream_at_once() {
    let mut session = listening();
    let mut changed = config(Mode::AlwaysOn);
    changed.keyterms = vec!["Sceneuron".to_owned()];
    session.configure(changed, 200);
    let actions = session.take_actions();
    assert_eq!(
        actions
            .iter()
            .filter(|a| matches!(a, Action::Disconnect | Action::Connect))
            .count(),
        2
    );
    assert_eq!(actions.iter().position(|a| *a == Action::Disconnect), Some(0));
}

#[test]
fn settings_changed_while_a_session_opens_restart_the_attempt() {
    let mut session = Session::new(config(Mode::AlwaysOn), 0);
    session.take_actions();
    let mut changed = config(Mode::AlwaysOn);
    changed.language = "en".to_owned();
    session.configure(changed, 50);
    let actions = session.take_actions();
    let order: Vec<&Action> = actions
        .iter()
        .filter(|a| matches!(a, Action::Disconnect | Action::Connect))
        .collect();
    assert_eq!(order, vec![&Action::Disconnect, &Action::Connect]);
    session.connected(100);
    assert_eq!(statuses(&session.take_actions()), vec![VoiceStatus::Listening]);
}
