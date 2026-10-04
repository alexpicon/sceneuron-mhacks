// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the debounced presence state machine: confirmation delays,
//! flicker resistance, stale-camera handling, ordering, and forced states.

use sceneuron_core::{DetectionSettings, Presence, PresenceTracker};

fn settings() -> DetectionSettings {
    DetectionSettings {
        min_confidence: 0.7,
        present_after_ms: 500,
        away_after_ms: 3_000,
        stale_after_ms: 2_000,
        sample_fps: 5.0,
    }
}

/// Feeds `score` every 200 ms from `start` through `end` inclusive; returns every change.
fn feed(tracker: &mut PresenceTracker, score: f64, start: u64, end: u64) -> Vec<(u64, Presence)> {
    let s = settings();
    (start..=end)
        .step_by(200)
        .filter_map(|t| tracker.observe(score, t, &s).map(|p| (t, p)))
        .collect()
}

#[test]
fn starts_unknown() {
    let tracker = PresenceTracker::new(0);
    assert_eq!(tracker.state(), Presence::Unknown);
    assert_eq!(tracker.last_score(), None);
}

#[test]
fn face_must_persist_before_present() {
    let mut tracker = PresenceTracker::new(0);
    let changes = feed(&mut tracker, 0.95, 0, 1_000);
    assert_eq!(changes, vec![(600, Presence::Present)]);
    assert_eq!(tracker.since_ms(), 600);
}

#[test]
fn absence_must_persist_before_away() {
    let mut tracker = PresenceTracker::new(0);
    feed(&mut tracker, 0.95, 0, 1_000);
    let changes = feed(&mut tracker, 0.1, 1_200, 5_000);
    assert_eq!(changes, vec![(4_200, Presence::Away)]);
}

#[test]
fn short_dropouts_do_not_flip_state() {
    let mut tracker = PresenceTracker::new(0);
    feed(&mut tracker, 0.95, 0, 1_000);
    let s = settings();
    for (i, t) in (1_200..10_000).step_by(200).enumerate() {
        let score = if i % 5 == 0 { 0.2 } else { 0.9 };
        assert_eq!(tracker.observe(score, t, &s), None, "flipped at {t}");
    }
    assert_eq!(tracker.state(), Presence::Present);
}

#[test]
fn score_exactly_at_threshold_counts_as_face() {
    let mut tracker = PresenceTracker::new(0);
    let changes = feed(&mut tracker, 0.7, 0, 600);
    assert_eq!(changes, vec![(600, Presence::Present)]);
}

#[test]
fn stale_camera_becomes_unknown_not_away() {
    let mut tracker = PresenceTracker::new(0);
    feed(&mut tracker, 0.95, 0, 1_000);
    let s = settings();
    assert_eq!(tracker.tick(2_900, &s), None);
    assert_eq!(tracker.tick(3_100, &s), Some(Presence::Unknown));
    assert_eq!(tracker.tick(9_000, &s), None);
}

#[test]
fn tick_without_observations_stays_unknown() {
    let mut tracker = PresenceTracker::new(0);
    assert_eq!(tracker.tick(60_000, &settings()), None);
    assert_eq!(tracker.state(), Presence::Unknown);
}

#[test]
fn recovers_after_stale_with_fresh_confirmation() {
    let mut tracker = PresenceTracker::new(0);
    feed(&mut tracker, 0.95, 0, 1_000);
    tracker.tick(4_000, &settings());
    let changes = feed(&mut tracker, 0.95, 10_000, 10_600);
    assert_eq!(changes, vec![(10_600, Presence::Present)]);
}

#[test]
fn ignores_out_of_order_observations() {
    let mut tracker = PresenceTracker::new(0);
    let s = settings();
    tracker.observe(0.9, 1_000, &s);
    assert_eq!(tracker.observe(0.1, 900, &s), None);
    assert_eq!(tracker.last_score(), Some(0.9));
}

#[test]
fn forced_state_changes_once_and_real_evidence_wins_later() {
    let mut tracker = PresenceTracker::new(0);
    assert_eq!(tracker.force(Presence::Away, 100), Some(Presence::Away));
    assert_eq!(tracker.force(Presence::Away, 200), None);
    let changes = feed(&mut tracker, 0.95, 400, 1_000);
    assert_eq!(changes, vec![(1_000, Presence::Present)]);
}

#[test]
fn evidence_before_a_gap_does_not_count_after_it() {
    let mut tracker = PresenceTracker::new(0);
    let s = settings();
    tracker.observe(0.95, 0, &s);
    assert_eq!(tracker.observe(0.95, 400, &s), None);
    assert_eq!(
        tracker.observe(0.95, 5_000, &s),
        None,
        "a run across a stale gap must restart"
    );
    assert_eq!(tracker.observe(0.95, 5_600, &s), Some(Presence::Present));
}

#[test]
fn stale_time_is_never_shorter_than_three_samples() {
    let slow = DetectionSettings {
        sample_fps: 0.5,
        ..settings()
    };
    assert_eq!(slow.effective_stale_ms(), 6_000);
    let mut tracker = PresenceTracker::new(0);
    tracker.observe(0.95, 0, &slow);
    assert_eq!(tracker.tick(5_900, &slow), None);
}
