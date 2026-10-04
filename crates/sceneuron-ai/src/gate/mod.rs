// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! When to ask the vision model: the scheduling policy behind the director, free of threads and
//! clocks so every rule can be tested with explicit times.
//!
//! The model is asked early when something may have changed (the first look, the streamer
//! asked, the conditions changed, presence changed, a local detector noticed something new, or
//! the picture moved enough), otherwise once per refresh interval. Calls never come closer than
//! the minimum interval or more often than the per-minute cap, the hourly budget pauses them,
//! and failures back off exponentially.

mod history;
mod pending;

use sceneuron_core::AiPolicy;

use crate::image::Thumbnail;
use history::{Calls, MINUTE_MS, Spend};
use pending::Pending;

const FIRST_BACKOFF_MS: u64 = 2_000;
const MAX_BACKOFF_MS: u64 = 120_000;

/// Something that makes a new look worthwhile before the refresh interval passes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    FirstLook,
    PresenceChanged,
    /// A local detector reported something it had not seen before.
    Novel,
    /// The streamer asked for a look now.
    Asked,
    ConditionsChanged,
}

/// What the director should do now.
#[derive(Clone, Debug, PartialEq)]
pub enum Decision {
    Call {
        reason: &'static str,
    },
    /// Nothing is due before `until_ms`, unless a new reason or frame arrives.
    Wait {
        until_ms: u64,
    },
    /// The hourly budget is spent; calls resume at `until_ms`.
    Paused {
        message: String,
        until_ms: u64,
    },
}

/// Call history and pending reasons; [`Gate::decide`] turns them into a [`Decision`].
#[derive(Clone, Debug)]
pub struct Gate {
    pending: Pending,
    in_flight: Pending,
    last_call_ms: Option<u64>,
    calls: Calls,
    spend: Spend,
    failures: u32,
    retry_at_ms: u64,
    /// Thumbnail of the last frame the model answered about.
    analyzed: Option<Thumbnail>,
    /// Thumbnail of the frame in flight.
    candidate: Option<Thumbnail>,
}

impl Default for Gate {
    fn default() -> Self {
        Self::new()
    }
}

impl Gate {
    /// A gate that wants a first look.
    pub fn new() -> Gate {
        Gate {
            pending: Pending::FIRST_LOOK,
            in_flight: Pending::default(),
            last_call_ms: None,
            calls: Calls::default(),
            spend: Spend::default(),
            failures: 0,
            retry_at_ms: 0,
            analyzed: None,
            candidate: None,
        }
    }

    /// Records a reason for an early call.
    pub fn note(&mut self, reason: Reason) {
        let pending = &mut self.pending;
        match reason {
            Reason::FirstLook => pending.first_look = true,
            Reason::PresenceChanged => pending.presence = true,
            Reason::Novel => pending.novel = true,
            Reason::Asked => pending.asked = true,
            Reason::ConditionsChanged => pending.conditions = true,
        }
    }

    /// Starts over after scene understanding was inactive: forgets the last analyzed picture and
    /// any failure backoff, and wants a first look. Call and spend history are kept, so the
    /// rate and budget limits still hold.
    pub fn restart(&mut self) {
        self.pending = Pending::FIRST_LOOK;
        self.in_flight = Pending::default();
        self.analyzed = None;
        self.candidate = None;
        self.failures = 0;
        self.retry_at_ms = 0;
    }

    /// The pause the hourly budget imposes at `now_ms`, if it is spent; no frame is needed.
    pub fn pause(&self, policy: &AiPolicy, now_ms: u64) -> Option<Decision> {
        let until_ms = self.spend.over_budget_until(policy.max_cost_per_hour, now_ms)?;
        let minutes = until_ms.saturating_sub(now_ms).div_ceil(MINUTE_MS).max(1);
        let message = format!(
            "hourly budget of ${:.2} reached; resumes in {minutes} min",
            policy.max_cost_per_hour
        );
        Some(Decision::Paused { message, until_ms })
    }

    /// Decides what to do at `now_ms` about a new frame summarized by `thumbnail`.
    pub fn decide(&self, policy: &AiPolicy, now_ms: u64, thumbnail: &Thumbnail) -> Decision {
        if let Some(paused) = self.pause(policy, now_ms) {
            return paused;
        }
        // The failure backoff, the minimum interval, and the per-minute cap each forbid a call
        // until some time; waiting for the latest of them avoids waking up too early.
        let limits = [
            Some(self.retry_at_ms),
            self.last_call_ms
                .map(|last| last.saturating_add(policy.min_interval_ms)),
            self.calls.limited_until(policy.max_calls_per_minute, now_ms),
        ];
        if let Some(until_ms) = limits.into_iter().flatten().filter(|until| now_ms < *until).max() {
            return Decision::Wait { until_ms };
        }
        if let Some(reason) = self.pending.reason() {
            return Decision::Call { reason };
        }
        let Some(analyzed) = &self.analyzed else {
            return Decision::Call { reason: "first look" };
        };
        if thumbnail.difference(analyzed) >= policy.change_threshold {
            return Decision::Call {
                reason: "the picture changed",
            };
        }
        let due_ms = self
            .last_call_ms
            .map_or(now_ms, |last| last.saturating_add(policy.refresh_ms));
        if now_ms >= due_ms {
            Decision::Call { reason: "refresh" }
        } else {
            Decision::Wait { until_ms: due_ms }
        }
    }

    /// Records that a call started at `now_ms` for the frame summarized by `thumbnail`.
    pub fn started(&mut self, now_ms: u64, thumbnail: Thumbnail) {
        self.last_call_ms = Some(now_ms);
        self.calls.record(now_ms);
        self.in_flight = std::mem::take(&mut self.pending);
        self.candidate = Some(thumbnail);
    }

    /// Records that the call answered (a refusal counts): its picture becomes the reference for
    /// change detection and the failure backoff resets.
    pub fn succeeded(&mut self) {
        self.failures = 0;
        self.retry_at_ms = 0;
        self.in_flight = Pending::default();
        if let Some(thumbnail) = self.candidate.take() {
            self.analyzed = Some(thumbnail);
        }
    }

    /// Records a failed call at `now_ms` and returns the backoff before the next attempt:
    /// 2 s, 4 s, 8 s, and so on up to 120 s. The reasons the call was for stay pending.
    pub fn failed(&mut self, now_ms: u64) -> u64 {
        self.failures = self.failures.saturating_add(1);
        let doublings = self.failures.saturating_sub(1).min(16);
        let backoff_ms = (FIRST_BACKOFF_MS << doublings).min(MAX_BACKOFF_MS);
        self.retry_at_ms = now_ms.saturating_add(backoff_ms);
        self.pending = self.pending.union(self.in_flight).union(Pending::RETRY);
        self.in_flight = Pending::default();
        self.candidate = None;
        backoff_ms
    }

    /// Consecutive failures since the last answer.
    pub fn failures(&self) -> u32 {
        self.failures
    }

    /// Records spend at `at_ms`, from vision or planner calls alike.
    pub fn spent(&mut self, at_ms: u64, cost_usd: f64) {
        self.spend.record(at_ms, cost_usd);
    }

    /// Estimated spend in the hour before `now_ms`.
    pub fn hour_cost(&self, now_ms: u64) -> f64 {
        self.spend.total(now_ms)
    }
}
