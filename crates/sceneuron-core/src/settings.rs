// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Project settings for the local face detector and for the vision model's cadence and budget.

use serde::{Deserialize, Serialize};

/// Thresholds that turn per-frame face scores into stable presence changes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DetectionSettings {
    /// Minimum face score that counts as "someone is in view".
    pub min_confidence: f64,
    /// Time a face must stay in view before presence becomes `present`.
    pub present_after_ms: u64,
    /// Time without a face before presence becomes `away`.
    pub away_after_ms: u64,
    /// Time without any analyzed frame before presence becomes `unknown`.
    pub stale_after_ms: u64,
    /// Frames per second sent to the detector.
    pub sample_fps: f64,
}

impl Default for DetectionSettings {
    fn default() -> Self {
        Self {
            min_confidence: 0.7,
            present_after_ms: 500,
            away_after_ms: 3_000,
            stale_after_ms: 2_000,
            sample_fps: 5.0,
        }
    }
}

impl DetectionSettings {
    /// The stale timeout actually used: never shorter than three sample intervals, so a slow
    /// sample rate cannot make presence flap to `unknown` between frames.
    pub fn effective_stale_ms(&self) -> u64 {
        let interval_ms = (1_000.0 / self.sample_fps.max(0.01)).ceil() as u64;
        self.stale_after_ms.max(interval_ms.saturating_mul(3))
    }
}

/// When and how often the vision model is asked about the camera.
///
/// The model is only consulted when something may have changed: the camera image moved enough,
/// presence changed, or the refresh interval passed. The call rate and the hourly spend are capped.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AiPolicy {
    /// Whether scene understanding runs at all.
    pub enabled: bool,
    /// Minimum time between two vision calls.
    pub min_interval_ms: u64,
    /// Ask again after this long even when the image looks unchanged.
    pub refresh_ms: u64,
    /// Fraction of the image (0 to 1) that must change to justify an early call.
    pub change_threshold: f64,
    /// Upper bound on vision calls per minute.
    pub max_calls_per_minute: u32,
    /// Spending cap in US dollars per hour; scene understanding pauses when it is reached.
    pub max_cost_per_hour: f64,
}

impl Default for AiPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            min_interval_ms: 2_500,
            refresh_ms: 20_000,
            change_threshold: 0.06,
            max_calls_per_minute: 12,
            max_cost_per_hour: 2.0,
        }
    }
}

impl AiPolicy {
    /// Time without an analyzed image after which conditions turn off: three refresh intervals,
    /// and at least one minute.
    pub fn stale_after_ms(&self) -> u64 {
        self.refresh_ms.saturating_mul(3).max(60_000)
    }
}
