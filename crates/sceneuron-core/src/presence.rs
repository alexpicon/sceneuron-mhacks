// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Turns noisy per-frame face scores into stable presence changes.

use serde::Serialize;

use crate::settings::DetectionSettings;

/// Whether someone is in front of the analyzed camera.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Presence {
    /// No recent analyzed frames, or not enough evidence yet.
    Unknown,
    Present,
    Away,
}

/// A run of consecutive observations that agree on whether a face is in view.
#[derive(Clone, Copy, Debug)]
struct Run {
    face_in_view: bool,
    started_ms: u64,
}

/// Debounced presence state machine.
///
/// A change to `present` needs faces for `present_after_ms`, a change to `away`
/// needs no faces for `away_after_ms`, and the stale time without any
/// observation resets the state to `unknown` so a stopped camera never reads as
/// someone leaving. A gap that long also ends the current run, so evidence from
/// before the gap never counts toward a change after it.
#[derive(Clone, Debug)]
pub struct PresenceTracker {
    state: Presence,
    since_ms: u64,
    run: Option<Run>,
    last_seen_ms: Option<u64>,
    last_score: Option<f64>,
}

impl PresenceTracker {
    pub fn new(now_ms: u64) -> Self {
        Self {
            state: Presence::Unknown,
            since_ms: now_ms,
            run: None,
            last_seen_ms: None,
            last_score: None,
        }
    }

    pub fn state(&self) -> Presence {
        self.state
    }

    /// Time of the last state change.
    pub fn since_ms(&self) -> u64 {
        self.since_ms
    }

    /// Time of the last observation, if any.
    pub fn last_seen_ms(&self) -> Option<u64> {
        self.last_seen_ms
    }

    /// Best face score of the last observation, if any.
    pub fn last_score(&self) -> Option<f64> {
        self.last_score
    }

    /// Feeds one observation and returns the new state if it changed.
    ///
    /// Observations older than the previous one are ignored.
    pub fn observe(&mut self, score: f64, at_ms: u64, settings: &DetectionSettings) -> Option<Presence> {
        if self.last_seen_ms.is_some_and(|last| at_ms < last) {
            return None;
        }
        if self
            .last_seen_ms
            .is_some_and(|last| at_ms - last > settings.effective_stale_ms())
        {
            self.run = None;
        }
        self.last_seen_ms = Some(at_ms);
        self.last_score = Some(score);
        let face_in_view = score >= settings.min_confidence;
        let run = match self.run {
            Some(run) if run.face_in_view == face_in_view => run,
            _ => Run {
                face_in_view,
                started_ms: at_ms,
            },
        };
        self.run = Some(run);
        let (target, needed_ms) = if face_in_view {
            (Presence::Present, settings.present_after_ms)
        } else {
            (Presence::Away, settings.away_after_ms)
        };
        if self.state != target && at_ms - run.started_ms >= needed_ms {
            return self.change(target, at_ms);
        }
        None
    }

    /// Resets to `unknown` when no observation arrived for the stale time.
    pub fn tick(&mut self, now_ms: u64, settings: &DetectionSettings) -> Option<Presence> {
        let last_seen_ms = self.last_seen_ms?;
        let stale = now_ms.saturating_sub(last_seen_ms) > settings.effective_stale_ms();
        if stale && self.state != Presence::Unknown {
            self.run = None;
            return self.change(Presence::Unknown, now_ms);
        }
        None
    }

    /// Sets the state directly, for previews and tests of rules.
    pub fn force(&mut self, state: Presence, at_ms: u64) -> Option<Presence> {
        self.run = None;
        if self.state == state {
            None
        } else {
            self.change(state, at_ms)
        }
    }

    fn change(&mut self, state: Presence, at_ms: u64) -> Option<Presence> {
        self.state = state;
        self.since_ms = at_ms;
        Some(state)
    }
}
