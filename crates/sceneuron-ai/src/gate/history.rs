// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Sliding windows over past calls and spend, for the per-minute cap and the hourly budget.

use std::collections::VecDeque;

pub(super) const MINUTE_MS: u64 = 60_000;
pub(super) const HOUR_MS: u64 = 3_600_000;
/// Sums of float costs drift by tiny amounts; less than this is no spend at all.
const NO_SPEND: f64 = 1e-9;

/// Start times of calls in the last minute.
#[derive(Clone, Debug, Default)]
pub(super) struct Calls {
    starts: VecDeque<u64>,
}

impl Calls {
    pub fn record(&mut self, at_ms: u64) {
        self.starts.push_back(at_ms);
        while self
            .starts
            .front()
            .is_some_and(|start| start.saturating_add(MINUTE_MS) <= at_ms)
        {
            self.starts.pop_front();
        }
    }

    /// When `cap` calls already started in the minute before `now_ms`, the time the oldest of
    /// them leaves the window and allows another.
    pub fn limited_until(&self, cap: u32, now_ms: u64) -> Option<u64> {
        if cap == 0 {
            return Some(now_ms.saturating_add(MINUTE_MS));
        }
        let recent: Vec<u64> = self
            .starts
            .iter()
            .copied()
            .filter(|start| now_ms < start.saturating_add(MINUTE_MS))
            .collect();
        let cap = cap as usize;
        (recent.len() >= cap).then(|| recent[recent.len() - cap].saturating_add(MINUTE_MS))
    }
}

/// Costs of calls in the last hour.
#[derive(Clone, Debug, Default)]
pub(super) struct Spend {
    entries: VecDeque<(u64, f64)>,
}

impl Spend {
    pub fn record(&mut self, at_ms: u64, cost_usd: f64) {
        if cost_usd.is_finite() && cost_usd > 0.0 {
            self.entries.push_back((at_ms, cost_usd));
        }
        while self
            .entries
            .front()
            .is_some_and(|(at, _)| at.saturating_add(HOUR_MS) <= at_ms)
        {
            self.entries.pop_front();
        }
    }

    /// Spend in the hour before `now_ms`.
    pub fn total(&self, now_ms: u64) -> f64 {
        self.recent(now_ms).map(|(_, cost)| cost).sum()
    }

    /// When the spend has reached `cap`, the time at which enough of it leaves the window to
    /// fall back under the cap. A zero cap pauses after the first paid call.
    pub fn over_budget_until(&self, cap: f64, now_ms: u64) -> Option<u64> {
        let over_budget = |spend: f64| spend > NO_SPEND && spend >= cap;
        let mut remaining = self.total(now_ms);
        if !over_budget(remaining) {
            return None;
        }
        let mut until_ms = now_ms;
        for (at, cost) in self.recent(now_ms) {
            remaining -= cost;
            until_ms = at.saturating_add(HOUR_MS);
            if !over_budget(remaining) {
                break;
            }
        }
        Some(until_ms)
    }

    fn recent(&self, now_ms: u64) -> impl Iterator<Item = (u64, f64)> + '_ {
        self.entries
            .iter()
            .copied()
            .filter(move |(at, _)| now_ms < at.saturating_add(HOUR_MS))
    }
}
