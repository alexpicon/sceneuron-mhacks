// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the session policy in push-to-talk mode with made-up times: nothing leaves without
//! consent, a press opens a session, audio waits for it and goes out in 100 ms pieces, a release
//! commits, transcripts are addressed, and an idle session closes.

mod common;

use common::{config, count, final_text, heard, pcm, sends, statuses, usage};
use sceneuron_voice::session::{Action, CHUNK_BYTES, COMMIT_WAIT_MS, IDLE_CLOSE_MS, MAX_HELD_BYTES, Session};
use sceneuron_voice::{ApiKey, Heard, Mode, ServerMessage, VoiceConfig, VoiceStatus};

fn session() -> Session {
    let mut session = Session::new(config(Mode::PushToTalk), 0);
    assert_eq!(statuses(&session.take_actions()), vec![VoiceStatus::Ready]);
    session
}

/// Presses, connects, says `ms` of audio, and releases; returns the actions after the press.
fn turn(session: &mut Session, at: u64, ms: usize) -> Vec<Action> {
    session.talk(true, at);
    session.connected(at + 100);
    session.audio(&pcm(ms), at + 200);
    session.talk(false, at + 300);
    session.take_actions()
}

#[test]
fn nothing_is_sent_without_being_on_consent_and_a_key() {
    let blocked = [
        (
            VoiceConfig {
                enabled: false,
                ..config(Mode::PushToTalk)
            },
            VoiceStatus::Off,
        ),
        (
            VoiceConfig {
                consent: false,
                ..config(Mode::PushToTalk)
            },
            VoiceStatus::NeedsConsent,
        ),
        (
            VoiceConfig {
                api_key: ApiKey::default(),
                ..config(Mode::PushToTalk)
            },
            VoiceStatus::NeedsKey,
        ),
    ];
    for (config, status) in blocked {
        let mut session = Session::new(config, 0);
        session.talk(true, 10);
        assert!(!session.wants_audio());
        session.audio(&pcm(500), 20);
        session.talk(false, 30);
        session.tick(60_000);
        let actions = session.take_actions();
        assert!(
            sends(&actions).is_empty() && count(&actions, &Action::Connect) == 0,
            "{status:?}"
        );
        assert_eq!(session.status(), &status);
    }
}

#[test]
fn a_press_opens_a_session_and_audio_waits_for_it() {
    let mut session = session();
    assert!(!session.wants_audio());
    session.talk(true, 1_000);
    assert!(session.wants_audio());
    let actions = session.take_actions();
    assert_eq!(count(&actions, &Action::Connect), 1);
    assert_eq!(statuses(&actions), vec![VoiceStatus::Connecting]);
    session.audio(&pcm(250), 1_050);
    assert!(
        sends(&session.take_actions()).is_empty(),
        "nothing goes out before the session starts"
    );
    session.connected(1_200);
    let actions = session.take_actions();
    assert_eq!(sends(&actions), vec![(CHUNK_BYTES, false), (CHUNK_BYTES, false)]);
    assert_eq!(statuses(&actions), vec![VoiceStatus::Listening]);
    session.audio(&pcm(60), 1_300);
    assert_eq!(sends(&session.take_actions()), vec![(CHUNK_BYTES, false)]);
}

#[test]
fn a_release_commits_the_rest_and_reports_the_audio_used() {
    let mut session = session();
    let actions = turn(&mut session, 0, 250);
    assert_eq!(
        sends(&actions),
        vec![(CHUNK_BYTES, false), (CHUNK_BYTES, false), (CHUNK_BYTES / 2, true)]
    );
    assert!((usage(&actions) - 0.25).abs() < 1e-9);
    assert_eq!(session.status(), &VoiceStatus::Ready);
    assert!(!session.wants_audio());
    session.audio(&pcm(100), 400);
    assert!(
        sends(&session.take_actions()).is_empty(),
        "audio after the release stays home"
    );
}

#[test]
fn a_release_before_the_session_starts_commits_once_it_does() {
    let mut session = session();
    session.talk(true, 0);
    session.audio(&pcm(150), 50);
    session.talk(false, 100);
    assert!(sends(&session.take_actions()).is_empty());
    session.connected(900);
    assert_eq!(
        sends(&session.take_actions()),
        vec![(CHUNK_BYTES, false), (CHUNK_BYTES / 2, true)]
    );
}

#[test]
fn a_tap_without_audio_sends_nothing() {
    let mut session = session();
    session.talk(true, 0);
    session.connected(100);
    session.talk(false, 150);
    let actions = session.take_actions();
    assert!(sends(&actions).is_empty());
    assert_eq!(session.next_wake_ms(), Some(150 + IDLE_CLOSE_MS));
}

#[test]
fn everything_heard_in_a_turn_is_addressed_and_the_final_text_ends_it() {
    let mut session = session();
    session.talk(true, 0);
    session.connected(100);
    session.audio(&pcm(500), 200);
    session.message(ServerMessage::Partial("wrap".to_owned()), 300);
    session.talk(false, 400);
    session.message(ServerMessage::Committed(" wrap it up ".to_owned()), 700);
    let partial = Heard {
        text: "wrap".to_owned(),
        is_final: false,
        addressed: true,
    };
    assert_eq!(
        heard(&session.take_actions()),
        vec![partial, final_text("wrap it up", true)]
    );
    assert_eq!(
        session.next_wake_ms(),
        Some(700 + IDLE_CLOSE_MS),
        "the turn ended with its final text"
    );
}

#[test]
fn empty_transcripts_end_the_turn_without_an_event() {
    let mut session = session();
    turn(&mut session, 0, 300);
    session.message(ServerMessage::Committed("   ".to_owned()), 500);
    assert!(heard(&session.take_actions()).is_empty());
    assert_eq!(session.next_wake_ms(), Some(500 + IDLE_CLOSE_MS));
}

#[test]
fn a_turn_whose_transcript_never_comes_ends_after_the_wait() {
    let mut session = session();
    turn(&mut session, 0, 300);
    assert_eq!(session.next_wake_ms(), Some(300 + COMMIT_WAIT_MS));
    session.tick(300 + COMMIT_WAIT_MS);
    assert_eq!(session.next_wake_ms(), Some(300 + IDLE_CLOSE_MS));
}

#[test]
fn an_idle_session_closes_and_the_next_press_opens_another() {
    let mut session = session();
    turn(&mut session, 0, 300);
    session.message(ServerMessage::Committed("lower".to_owned()), 500);
    session.take_actions();
    session.tick(500 + IDLE_CLOSE_MS - 1);
    assert_eq!(count(&session.take_actions(), &Action::Disconnect), 0);
    session.tick(500 + IDLE_CLOSE_MS);
    assert_eq!(count(&session.take_actions(), &Action::Disconnect), 1);
    session.talk(true, 40_000);
    assert_eq!(count(&session.take_actions(), &Action::Connect), 1);
}

#[test]
fn a_press_while_the_session_is_open_reuses_it() {
    let mut session = session();
    turn(&mut session, 0, 300);
    session.message(ServerMessage::Committed("pause".to_owned()), 500);
    session.take_actions();
    session.talk(true, 5_000);
    session.audio(&pcm(100), 5_100);
    let actions = session.take_actions();
    assert_eq!(count(&actions, &Action::Connect), 0);
    assert_eq!(sends(&actions), vec![(CHUNK_BYTES, false)]);
}

#[test]
fn audio_held_while_connecting_is_capped_to_the_latest() {
    let mut session = session();
    session.talk(true, 0);
    session.audio(&pcm(40_000), 10);
    session.connected(20);
    let sent: usize = sends(&session.take_actions()).iter().map(|(bytes, _)| bytes).sum();
    assert_eq!(sent, MAX_HELD_BYTES);
}

#[test]
fn withdrawing_consent_closes_the_session_and_drops_waiting_audio() {
    let mut session = session();
    session.talk(true, 0);
    session.connected(100);
    session.audio(&pcm(50), 200);
    session.take_actions();
    session.configure(
        VoiceConfig {
            consent: false,
            ..config(Mode::PushToTalk)
        },
        300,
    );
    let actions = session.take_actions();
    assert_eq!(count(&actions, &Action::Disconnect), 1);
    assert_eq!(statuses(&actions), vec![VoiceStatus::NeedsConsent]);
    assert!(!session.wants_audio());
    session.audio(&pcm(500), 400);
    session.talk(false, 500);
    assert!(sends(&session.take_actions()).is_empty());
}

#[test]
fn new_keyterms_replace_the_session_once_the_turn_is_over() {
    let mut session = session();
    session.talk(true, 0);
    session.connected(100);
    let mut changed = config(Mode::PushToTalk);
    changed.keyterms.push("Lower".to_owned());
    session.configure(changed.clone(), 200);
    assert_eq!(
        count(&session.take_actions(), &Action::Disconnect),
        0,
        "the held turn keeps its session"
    );
    session.talk(false, 300);
    assert_eq!(count(&session.take_actions(), &Action::Disconnect), 1);
    assert_eq!(session.config(), &changed);
    session.configure(changed, 400);
    assert!(session.take_actions().is_empty(), "the same config changes nothing");
}
