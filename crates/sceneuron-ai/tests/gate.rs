// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the scheduling policy with explicit times: first looks, early reasons, picture
//! changes, refreshes, waiting for every limit, the failure backoff, and restarts.

mod common;

use common::gate::{analyzed_at, call, policy, wait};
use sceneuron_ai::{Decision, Gate, Reason, Thumbnail};
use sceneuron_core::AiPolicy;

/// A thumbnail whose first `changed` of 576 cells are white and the rest black.
fn partly_white(changed: usize) -> Thumbnail {
    let width = 32u32;
    let rgb: Vec<u8> = (0..18 * width as usize)
        .flat_map(|cell| if cell < changed { [255u8; 3] } else { [0u8; 3] })
        .collect();
    Thumbnail::of(&sceneuron_ai::Frame {
        rgb: rgb.into(),
        width,
        height: 18,
        captured_ms: 0,
    })
}

#[test]
fn a_new_gate_wants_a_first_look() {
    assert_eq!(
        Gate::new().decide(&policy(), 0, &Thumbnail::uniform(0)),
        call("first look")
    );
}

#[test]
fn an_unchanged_picture_waits_for_the_refresh_interval() {
    let gate = analyzed_at(1_000);
    let same = Thumbnail::uniform(0);
    assert_eq!(gate.decide(&policy(), 5_000, &same), wait(21_000));
    assert_eq!(gate.decide(&policy(), 20_999, &same), wait(21_000));
    assert_eq!(gate.decide(&policy(), 21_000, &same), call("refresh"));
}

#[test]
fn a_changed_picture_calls_early_once_the_minimum_interval_passed() {
    let gate = analyzed_at(1_000);
    let changed = Thumbnail::uniform(255);
    assert_eq!(gate.decide(&policy(), 2_000, &changed), wait(3_500));
    assert_eq!(gate.decide(&policy(), 3_500, &changed), call("the picture changed"));
}

#[test]
fn the_change_threshold_decides_what_counts_as_changed() {
    let gate = analyzed_at(0);
    // 0.06 of 576 cells is 34.56 cells.
    assert_eq!(partly_white(34).difference(&Thumbnail::uniform(0)), 34.0 / 576.0);
    assert_eq!(gate.decide(&policy(), 5_000, &partly_white(34)), wait(20_000));
    assert_eq!(
        gate.decide(&policy(), 5_000, &partly_white(35)),
        call("the picture changed")
    );
    let strict = AiPolicy {
        change_threshold: 0.5,
        ..policy()
    };
    assert_eq!(gate.decide(&strict, 5_000, &partly_white(287)), wait(20_000));
    assert_eq!(
        gate.decide(&strict, 5_000, &partly_white(288)),
        call("the picture changed")
    );
}

#[test]
fn each_early_reason_calls_and_the_streamer_comes_first() {
    let reasons = [
        (Reason::PresenceChanged, "presence changed"),
        (Reason::Novel, "a local detector noticed something new"),
        (Reason::ConditionsChanged, "conditions changed"),
        (Reason::Asked, "the streamer asked"),
        (Reason::FirstLook, "first look"),
    ];
    for (reason, text) in reasons {
        let mut gate = analyzed_at(0);
        gate.note(reason);
        assert_eq!(gate.decide(&policy(), 1_000, &Thumbnail::uniform(0)), wait(2_500));
        assert_eq!(gate.decide(&policy(), 2_500, &Thumbnail::uniform(0)), call(text));
    }
    let mut gate = analyzed_at(0);
    gate.note(Reason::Novel);
    gate.note(Reason::Asked);
    gate.note(Reason::PresenceChanged);
    assert_eq!(
        gate.decide(&policy(), 2_500, &Thumbnail::uniform(0)),
        call("the streamer asked")
    );
}

#[test]
fn a_started_call_consumes_the_pending_reasons() {
    let mut gate = analyzed_at(0);
    gate.note(Reason::Asked);
    gate.started(3_000, Thumbnail::uniform(0));
    gate.succeeded();
    assert_eq!(gate.decide(&policy(), 6_000, &Thumbnail::uniform(0)), wait(23_000));
}

#[test]
fn waits_last_until_every_limit_allows_a_call() {
    let mut gate = analyzed_at(0);
    gate.started(10_000, Thumbnail::uniform(0));
    // The backoff ends at 12 s but the minimum interval only at 12.5 s.
    assert_eq!(gate.failed(10_000), 2_000);
    assert_eq!(gate.decide(&policy(), 11_000, &Thumbnail::uniform(0)), wait(12_500));
}

#[test]
fn failures_back_off_exponentially_up_to_two_minutes() {
    let quick = AiPolicy {
        min_interval_ms: 1_000,
        ..policy()
    };
    let mut gate = Gate::new();
    let mut backoffs = Vec::new();
    let mut now_ms = 0;
    for _ in 0..9 {
        gate.started(now_ms, Thumbnail::uniform(0));
        let backoff = gate.failed(now_ms);
        backoffs.push(backoff);
        let before = gate.decide(&quick, now_ms + backoff - 1, &Thumbnail::uniform(0));
        assert_eq!(before, wait(now_ms + backoff));
        let after = gate.decide(&quick, now_ms + backoff, &Thumbnail::uniform(0));
        assert_eq!(after, call("first look"));
        now_ms += backoff;
    }
    assert_eq!(
        backoffs,
        [2_000, 4_000, 8_000, 16_000, 32_000, 64_000, 120_000, 120_000, 120_000]
    );
    assert_eq!(gate.failures(), 9);
}

#[test]
fn a_failed_call_keeps_its_reasons_and_success_resets_the_backoff() {
    let mut gate = analyzed_at(0);
    gate.note(Reason::Asked);
    gate.started(10_000, Thumbnail::uniform(255));
    assert_eq!(gate.failed(10_000), 2_000);
    assert_eq!(gate.decide(&policy(), 11_000, &Thumbnail::uniform(255)), wait(12_500));
    assert_eq!(
        gate.decide(&policy(), 12_500, &Thumbnail::uniform(255)),
        call("the streamer asked")
    );
    gate.started(12_500, Thumbnail::uniform(255));
    gate.succeeded();
    assert_eq!(gate.failures(), 0);
    gate.started(30_000, Thumbnail::uniform(255));
    assert_eq!(gate.failed(30_000), 2_000);
}

#[test]
fn a_failed_call_without_reasons_retries_after_the_backoff() {
    let mut gate = analyzed_at(0);
    gate.started(20_000, Thumbnail::uniform(0));
    gate.failed(20_000);
    assert_eq!(
        gate.decide(&policy(), 22_500, &Thumbnail::uniform(0)),
        call("retrying after a failure")
    );
}

#[test]
fn only_an_answered_picture_becomes_the_reference() {
    let mut gate = analyzed_at(0);
    gate.started(20_000, Thumbnail::uniform(255));
    gate.failed(20_000);
    gate.started(22_500, Thumbnail::uniform(255));
    gate.succeeded();
    assert_eq!(gate.decide(&policy(), 30_000, &Thumbnail::uniform(255)), wait(42_500));
    assert_eq!(
        gate.decide(&policy(), 30_000, &Thumbnail::uniform(0)),
        call("the picture changed")
    );
}

#[test]
fn restarting_forgets_the_picture_and_backoff_but_keeps_limits() {
    let mut gate = analyzed_at(0);
    gate.spent(0, 5.0);
    gate.started(1_000, Thumbnail::uniform(0));
    gate.failed(1_000);
    gate.restart();
    assert_eq!(gate.failures(), 0);
    let Decision::Paused { .. } = gate.decide(&policy(), 4_000, &Thumbnail::uniform(0)) else {
        panic!("the budget must survive a restart");
    };
    let mut fresh = analyzed_at(0);
    fresh.restart();
    assert_eq!(fresh.decide(&policy(), 2_000, &Thumbnail::uniform(0)), wait(2_500));
    assert_eq!(
        fresh.decide(&policy(), 2_500, &Thumbnail::uniform(0)),
        call("first look")
    );
}
