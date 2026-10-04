// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises how the session policy handles failures with made-up times: retries that back off
//! and reset after a stable session, fatal errors that stop until a press or new settings,
//! renewals, normal closes, and server errors and warnings. What refusals show is in
//! `refusals.rs`.

mod common;

use common::{config, count, pcm, sends, statuses};
use sceneuron_voice::session::{Action, FIRST_RETRY_MS, Failure, IDLE_CLOSE_MS, MAX_RETRY_MS, STABLE_MS, Session};
use sceneuron_voice::{ErrorKind, Mode, ServerMessage, VoiceStatus};

fn retrying(message: &str, retry_in_ms: u64) -> VoiceStatus {
    VoiceStatus::Retrying {
        message: message.to_owned(),
        retry_in_ms,
    }
}

fn always_on() -> Session {
    let mut session = Session::new(config(Mode::AlwaysOn), 0);
    session.take_actions();
    session
}

#[test]
fn retries_wait_longer_after_each_failure_up_to_a_minute() {
    let mut session = always_on();
    let mut now = 0;
    let mut delays = Vec::new();
    for _ in 0..9 {
        session.failed(Failure::Retry("cannot reach the server".to_owned()), now);
        let VoiceStatus::Retrying { retry_in_ms, .. } = session.status().clone() else {
            panic!("not retrying: {:?}", session.status());
        };
        delays.push(retry_in_ms);
        assert_eq!(session.next_wake_ms(), Some(now + retry_in_ms));
        session.tick(now + retry_in_ms - 1);
        assert_eq!(count(&session.take_actions(), &Action::Connect), 0, "too early");
        now += retry_in_ms;
        session.tick(now);
        assert_eq!(count(&session.take_actions(), &Action::Connect), 1);
    }
    assert_eq!(
        delays,
        vec![1_000, 2_000, 4_000, 8_000, 16_000, 32_000, 60_000, 60_000, 60_000]
    );
    assert_eq!((FIRST_RETRY_MS, MAX_RETRY_MS), (1_000, 60_000));
}

#[test]
fn a_session_that_stays_up_resets_the_wait() {
    let mut session = always_on();
    session.failed(Failure::Retry("down".to_owned()), 0);
    session.tick(1_000);
    session.failed(Failure::Retry("down".to_owned()), 1_100);
    assert_eq!(session.status(), &retrying("down", 2_000));
    session.tick(3_100);
    session.connected(3_200);
    session.tick(3_200 + STABLE_MS);
    session.failed(Failure::Closed, 3_200 + STABLE_MS + 1);
    assert_eq!(session.status(), &retrying("the server ended the session", 1_000));
}

#[test]
fn a_renewal_reconnects_at_once_and_counts_no_failure() {
    let mut session = always_on();
    session.connected(100);
    session.take_actions();
    session.message(
        ServerMessage::Error {
            kind: ErrorKind::Renew,
            code: "session_time_limit_exceeded".to_owned(),
            message: String::new(),
        },
        200,
    );
    let actions = session.take_actions();
    assert_eq!(count(&actions, &Action::Connect), 1);
    assert_eq!(statuses(&actions), vec![VoiceStatus::Connecting]);
}

#[test]
fn a_refused_key_stops_until_a_press_or_new_settings() {
    let mut session = always_on();
    session.connected(100);
    session.take_actions();
    let refusal = ServerMessage::Error {
        kind: ErrorKind::Fatal,
        code: "auth_error".to_owned(),
        message: "Invalid API key".to_owned(),
    };
    session.message(refusal, 200);
    let stopped = VoiceStatus::Stopped {
        message: "ElevenLabs refused the session (auth_error: Invalid API key)".to_owned(),
    };
    let actions = session.take_actions();
    assert_eq!(statuses(&actions), vec![stopped]);
    assert_eq!(
        count(&actions, &Action::Disconnect),
        1,
        "the worker lets the session go"
    );
    assert!(!session.wants_audio());
    session.audio(&pcm(500), 300);
    session.tick(3_600_000);
    let actions = session.take_actions();
    assert!(sends(&actions).is_empty() && count(&actions, &Action::Connect) == 0);
    assert_eq!(session.next_wake_ms(), None);
    session.talk(true, 3_600_100);
    assert_eq!(
        count(&session.take_actions(), &Action::Connect),
        1,
        "a press tries again"
    );
    session.failed(Failure::Fatal("refused".to_owned()), 3_600_200);
    session.talk(false, 3_600_300);
    let mut fixed = config(Mode::AlwaysOn);
    fixed.api_key = sceneuron_voice::ApiKey::new("sk_new_key");
    session.configure(fixed, 3_600_400);
    assert_eq!(
        count(&session.take_actions(), &Action::Connect),
        1,
        "new settings try again"
    );
}

#[test]
fn an_idle_push_to_talk_session_that_closes_is_not_a_failure() {
    let mut session = Session::new(config(Mode::PushToTalk), 0);
    session.talk(true, 0);
    session.connected(100);
    session.talk(false, 200);
    session.take_actions();
    session.failed(Failure::Closed, 5_000);
    let actions = session.take_actions();
    assert_eq!(count(&actions, &Action::Connect), 0);
    assert_eq!(session.status(), &VoiceStatus::Ready);
    session.failed(Failure::Retry("reset".to_owned()), 6_000);
    assert_eq!(
        session.status(),
        &VoiceStatus::Ready,
        "nobody is waiting for that session"
    );
}

#[test]
fn a_held_turn_retries_and_keeps_its_audio() {
    let mut session = Session::new(config(Mode::PushToTalk), 0);
    session.talk(true, 0);
    session.audio(&pcm(200), 50);
    session.failed(Failure::Retry("cannot connect".to_owned()), 100);
    assert_eq!(session.status(), &retrying("cannot connect", 1_000));
    session.talk(false, 500);
    session.tick(1_100);
    assert_eq!(count(&session.take_actions(), &Action::Connect), 2);
    session.connected(1_200);
    assert_eq!(sends(&session.take_actions()), vec![(3_200, false), (3_200, true)]);
}

#[test]
fn a_press_before_a_waiting_commit_goes_out_joins_that_turn() {
    let mut session = Session::new(config(Mode::PushToTalk), 0);
    session.talk(true, 0);
    session.audio(&pcm(150), 10);
    session.talk(false, 20);
    session.talk(true, 30);
    session.audio(&pcm(50), 40);
    session.talk(false, 50);
    session.connected(60);
    assert_eq!(sends(&session.take_actions()), vec![(3_200, false), (3_200, true)]);
}

#[test]
fn a_sent_commit_lost_with_its_session_ends_the_turn() {
    let mut session = Session::new(config(Mode::PushToTalk), 0);
    session.talk(true, 0);
    session.connected(100);
    session.audio(&pcm(100), 150);
    session.talk(false, 200);
    session.failed(Failure::Retry("reset".to_owned()), 300);
    assert_eq!(session.status(), &VoiceStatus::Ready);
    assert_eq!(count(&session.take_actions(), &Action::Connect), 1);
}

#[test]
fn a_refused_commit_ends_the_turn_but_keeps_the_session() {
    let mut session = Session::new(config(Mode::PushToTalk), 0);
    session.talk(true, 0);
    session.connected(100);
    session.audio(&pcm(100), 150);
    session.talk(false, 200);
    session.take_actions();
    let warning = ServerMessage::Error {
        kind: ErrorKind::Warning,
        code: "commit_throttled".to_owned(),
        message: String::new(),
    };
    session.message(warning, 300);
    assert_eq!(count(&session.take_actions(), &Action::Disconnect), 0);
    assert_eq!(
        session.next_wake_ms(),
        Some(200 + IDLE_CLOSE_MS),
        "only the idle close is left"
    );
}

#[test]
fn server_errors_that_may_pass_retry() {
    let mut session = always_on();
    session.connected(100);
    let error = ServerMessage::Error {
        kind: ErrorKind::Retry,
        code: "rate_limited".to_owned(),
        message: "too many sessions".to_owned(),
    };
    session.message(error, 200);
    assert_eq!(
        session.status(),
        &retrying("ElevenLabs reported an error (rate_limited: too many sessions)", 1_000)
    );
}
