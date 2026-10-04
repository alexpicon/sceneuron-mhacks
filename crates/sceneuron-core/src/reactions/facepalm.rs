// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The facepalm: on-device analysis reports, frame by frame, whether a hand covers the upper half
//! of the face, and a facepalm held for [`FACEPALM_HOLD_MS`] plays the presets whose camera is
//! [`Camera::Facepalm`], with no AI call. Holding the hand there longer plays nothing more; the
//! hand has to leave and come back.

use super::presets::{Camera, Flood, presets};
use crate::engine::Engine;
use crate::types::Outcome;

/// How long a hand must stay over the face before the camera plays its preset: longer than
/// rubbing an eye or pushing up glasses, short enough to feel like a reaction.
pub const FACEPALM_HOLD_MS: u64 = 1_000;
/// Frames in a row without the hand that end the facepalm, since analysis misses a frame now and
/// then while the hand stays. Counting frames rather than time keeps a facepalm at any sample rate
/// and through a frame that a slow analysis delays.
const FACEPALM_MISSES: u32 = 2;
/// The least time without the hand that ends the facepalm, so a fast camera forgives as long a
/// miss as five frames a second do.
const FACEPALM_GONE_MS: u64 = 400;

/// What the camera has shown of a facepalm; kept in memory only.
#[derive(Clone, Debug, Default)]
pub(crate) struct FacepalmWatch {
    /// When the newest frame was taken; older frames are ignored.
    last_ms: Option<u64>,
    /// When the current facepalm began and when the hand was last seen over the face.
    run: Option<(u64, u64)>,
    /// Frames in a row without the hand over the face.
    missed: u32,
    /// The current facepalm reached the hold time, so it plays nothing more.
    acted: bool,
}

impl FacepalmWatch {
    /// Advances with a frame taken at `at_ms`; returns how long the facepalm had been held when
    /// it reaches the hold time with this frame. While `watching` is false, it is forgotten, and
    /// so it is when no frame came for `stale_ms`, since the hand may have left in between.
    fn step(&mut self, seen: bool, watching: bool, at_ms: u64, stale_ms: u64) -> Option<u64> {
        if self.last_ms.is_some_and(|last| at_ms < last) {
            return None;
        }
        let stalled = self.last_ms.is_some_and(|last| at_ms - last > stale_ms);
        self.last_ms = Some(at_ms);
        self.missed = if seen { 0 } else { self.missed.saturating_add(1) };
        let gone = self
            .run
            .is_some_and(|(_, last)| self.missed >= FACEPALM_MISSES && at_ms - last >= FACEPALM_GONE_MS);
        if !watching || stalled || gone {
            self.run = None;
            self.acted = false;
        }
        if !(seen && watching) {
            return None;
        }
        let since = self.run.map_or(at_ms, |(since, _)| since);
        self.run = Some((since, at_ms));
        let held_ms = at_ms - since;
        if self.acted || held_ms < FACEPALM_HOLD_MS {
            return None;
        }
        self.acted = true;
        Some(held_ms)
    }
}

impl Engine {
    /// Feeds whether a frame taken at `at_ms` shows a hand over the upper half of the face. A
    /// facepalm held for [`FACEPALM_HOLD_MS`] plays its preset unless the last flood that rules or
    /// the camera played began less than the fun reactions' cooldown ago; the event log says which.
    /// The camera watches for it only while fun reactions are on, the preset's camera is not
    /// switched off, and automation runs; otherwise the facepalm is forgotten. Two frames in a row
    /// without the hand end the facepalm once it has been gone for 400 ms, and so does a gap
    /// between frames longer than the detection stale time. Frames older than the newest are
    /// ignored.
    pub fn observe_facepalm(&mut self, seen: bool, at_ms: u64) -> Outcome {
        let floods: Vec<Flood> = presets()
            .iter()
            .filter(|preset| preset.camera == Some(Camera::Facepalm))
            .map(|preset| preset.flood)
            .filter(|flood| self.project.automation_enabled && self.project.reactions.watches(*flood))
            .collect();
        let stale_ms = self.project.detection.effective_stale_ms();
        let Some(held_ms) = self.facepalm.step(seen, !floods.is_empty(), at_ms, stale_ms) else {
            return Outcome::NONE;
        };
        let held_s = held_ms as f64 / 1_000.0;
        for flood in floods {
            let name = flood.preset().name;
            let detail = match self.rule_flood(flood, at_ms) {
                Ok(()) => format!("played {name:?} because a hand covered the face for {held_s:.1} s"),
                Err(ago_ms) => format!(
                    "saw a hand over the face for {held_s:.1} s, without {name:?}: the last flood began {} s ago",
                    ago_ms / 1_000
                ),
            };
            self.record(at_ms, "camera", &detail);
        }
        self.bump(Outcome::CHANGED)
    }
}
