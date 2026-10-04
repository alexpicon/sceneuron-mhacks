// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the scheduling limits over time windows with explicit times: the per-minute cap
//! in any sixty-second window, and the hourly budget with its pause, expiry, and zero cap.

mod common;

use common::gate::{analyzed_at, call, policy, wait};
use sceneuron_ai::{Decision, Gate, Reason, Thumbnail};
use sceneuron_core::AiPolicy;

#[test]
fn the_per_minute_cap_holds_in_any_sixty_second_window() {
    let quick = AiPolicy {
        min_interval_ms: 1_000,
        max_calls_per_minute: 3,
        ..policy()
    };
    let mut gate = Gate::new();
    for at_ms in [0, 1_000, 2_000] {
        gate.note(Reason::Asked);
        assert_eq!(
            gate.decide(&quick, at_ms, &Thumbnail::uniform(0)),
            call("the streamer asked")
        );
        gate.started(at_ms, Thumbnail::uniform(0));
        gate.succeeded();
    }
    gate.note(Reason::Asked);
    assert_eq!(gate.decide(&quick, 3_000, &Thumbnail::uniform(0)), wait(60_000));
    assert_eq!(gate.decide(&quick, 59_999, &Thumbnail::uniform(0)), wait(60_000));
    assert_eq!(
        gate.decide(&quick, 60_000, &Thumbnail::uniform(0)),
        call("the streamer asked")
    );

    let two = AiPolicy {
        min_interval_ms: 1_000,
        max_calls_per_minute: 2,
        ..policy()
    };
    let mut gate = Gate::new();
    gate.started(0, Thumbnail::uniform(0));
    gate.started(10_000, Thumbnail::uniform(0));
    gate.note(Reason::Asked);
    assert_eq!(gate.decide(&two, 20_000, &Thumbnail::uniform(0)), wait(60_000));
    assert_eq!(
        gate.decide(&two, 60_000, &Thumbnail::uniform(0)),
        call("the streamer asked")
    );
    gate.started(60_000, Thumbnail::uniform(0));
    gate.note(Reason::Asked);
    // The window now holds the calls at 10 s and 60 s; the one at 10 s leaves it at 70 s.
    assert_eq!(gate.decide(&two, 65_000, &Thumbnail::uniform(0)), wait(70_000));
}

#[test]
fn the_hourly_budget_pauses_until_enough_spend_expires() {
    let mut gate = analyzed_at(0);
    gate.spent(0, 1.5);
    gate.spent(600_000, 0.6);
    assert!((gate.hour_cost(1_200_000) - 2.1).abs() < 1e-9);
    let paused = gate.decide(&policy(), 1_200_000, &Thumbnail::uniform(255));
    let message = "hourly budget of $2.00 reached; resumes in 40 min".to_owned();
    assert_eq!(
        paused,
        Decision::Paused {
            message,
            until_ms: 3_600_000
        }
    );
    assert_eq!(
        gate.decide(&policy(), 3_600_000, &Thumbnail::uniform(255)),
        call("the picture changed")
    );
    assert!((gate.hour_cost(3_600_000) - 0.6).abs() < 1e-9);
}

#[test]
fn spend_under_the_budget_never_pauses() {
    let mut gate = analyzed_at(0);
    gate.spent(1_000, 1.99);
    assert_eq!(
        gate.decide(&policy(), 2_500, &Thumbnail::uniform(255)),
        call("the picture changed")
    );
    gate.spent(2_000, 0.0);
    gate.spent(2_000, f64::NAN);
    assert!((gate.hour_cost(2_500) - 1.99).abs() < 1e-9);
    assert_eq!(gate.pause(&policy(), 2_500), None);
    gate.spent(3_000, 0.01);
    let Some(Decision::Paused { until_ms, .. }) = gate.pause(&policy(), 3_000) else {
        panic!("the budget is spent, with or without a frame");
    };
    assert_eq!(until_ms, 3_601_000);
}

#[test]
fn a_zero_budget_pauses_after_the_first_paid_call_only() {
    let broke = AiPolicy {
        max_cost_per_hour: 0.0,
        ..policy()
    };
    let mut gate = Gate::new();
    assert_eq!(gate.decide(&broke, 0, &Thumbnail::uniform(0)), call("first look"));
    gate.started(0, Thumbnail::uniform(0));
    gate.succeeded();
    gate.spent(500, 0.01);
    let Decision::Paused { message, until_ms } = gate.decide(&broke, 30_000, &Thumbnail::uniform(0)) else {
        panic!("a zero budget must pause after spending");
    };
    assert_eq!(until_ms, 3_600_500);
    assert!(message.starts_with("hourly budget of $0.00 reached"), "{message}");
}

#[test]
fn spend_leaves_the_window_after_an_hour() {
    let mut gate = Gate::new();
    gate.spent(1_000, 0.5);
    assert_eq!(gate.hour_cost(3_600_999), 0.5);
    assert_eq!(gate.hour_cost(3_601_000), 0.0);
}
