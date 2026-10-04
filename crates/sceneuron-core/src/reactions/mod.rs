// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Fun reactions: overlay-wide emoji floods, each a [`Preset`] with its emojis and an optional
//! banner line, that the streamer plays by hand, rules play, and the camera plays by itself.
//!
//! The camera plays a preset in one of two ways (see [`Camera`]). The vision model judges a
//! built-in condition only while on-device analysis reports its [`Cue`], and a built-in rule plays
//! the preset when the condition turns on: sticking out a tongue, once the mouth is open, floods
//! the overlay with tongue-out faces. Or on-device analysis alone sees it: a facepalm held for a
//! second plays "This meeting could have been an email". The camera plays presets while the
//! project's switch is on and automation runs, except those whose camera the streamer switched
//! off. In always-on voice mode, a preset's trigger phrases play it as well, unless the streamer
//! switched its voice off. Rules, the camera, and trigger phrases play floods at least the cooldown
//! apart; a flood the streamer asks for, by hand or with words addressed to Sceneuron, plays at
//! once.

mod facepalm;
mod looks;
mod presets;

use serde::{Deserialize, Serialize};

pub use facepalm::FACEPALM_HOLD_MS;
pub(crate) use facepalm::FacepalmWatch;
pub use looks::{Cue, Reaction, TONGUE_OUT, reactions};
pub(crate) use looks::{builtin, watched_condition, watching};
pub use presets::{Camera, Flood, Preset, presets};

use crate::engine::Engine;
use crate::model::ValidationError;
use crate::types::Outcome;

/// The least time between two floods that rules and the camera play.
pub const MIN_FLOOD_COOLDOWN_MS: u64 = 1_000;
/// The time between two floods a new project starts with.
pub const DEFAULT_FLOOD_COOLDOWN_MS: u64 = 3_000;
/// The longest cooldown the settings accept: an hour.
pub const MAX_FLOOD_COOLDOWN_MS: u64 = 3_600_000;

/// The project's fun reactions switch, the presets whose camera or voice is off, and how far apart
/// rules, the camera, and trigger phrases may play floods.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Reactions {
    /// Whether the camera and trigger phrases play presets. Looks also need a vision model the
    /// streamer consented to, and every camera reaction and trigger phrase needs automation to run.
    pub enabled: bool,
    /// The least time between two floods that rules, the camera, and trigger phrases play, from
    /// [`MIN_FLOOD_COOLDOWN_MS`] to [`MAX_FLOOD_COOLDOWN_MS`].
    pub cooldown_ms: u64,
    /// Presets whose camera the streamer switched off; each has a [`Camera`], and none repeats.
    pub camera_off: Vec<Flood>,
    /// Presets whose trigger phrases the streamer switched off; each has phrases, and none
    /// repeats.
    pub voice_off: Vec<Flood>,
}

impl Default for Reactions {
    fn default() -> Self {
        Self {
            enabled: true,
            cooldown_ms: DEFAULT_FLOOD_COOLDOWN_MS,
            camera_off: Vec::new(),
            voice_off: Vec::new(),
        }
    }
}

impl Reactions {
    /// Whether the camera may play `flood`: the switch is on, the preset has a camera, and its
    /// camera is not switched off. Automation must run as well.
    pub fn watches(&self, flood: Flood) -> bool {
        self.enabled && flood.preset().camera.is_some() && !self.camera_off.contains(&flood)
    }

    /// Whether the trigger phrases of `flood` play it: the switch is on, the preset has phrases,
    /// and its voice is not switched off. Automation must run and voice listen always as well.
    pub fn hears(&self, flood: Flood) -> bool {
        self.enabled && !flood.preset().phrases.is_empty() && !self.voice_off.contains(&flood)
    }

    pub(crate) fn validate(&self) -> Result<(), ValidationError> {
        if !(MIN_FLOOD_COOLDOWN_MS..=MAX_FLOOD_COOLDOWN_MS).contains(&self.cooldown_ms) {
            let message = format!("reactions.cooldown_ms must be {MIN_FLOOD_COOLDOWN_MS} to {MAX_FLOOD_COOLDOWN_MS}");
            return Err(ValidationError(message));
        }
        let camera = |flood: Flood| flood.preset().camera.is_some();
        check_off("camera_off", &self.camera_off, camera, "the camera never plays")?;
        let voice = |flood: Flood| !flood.preset().phrases.is_empty();
        check_off("voice_off", &self.voice_off, voice, "has no trigger phrase")
    }
}

/// Checks a list of presets switched off: each must have what `has` asks for, and none repeats.
fn check_off(field: &str, off: &[Flood], has: impl Fn(Flood) -> bool, without: &str) -> Result<(), ValidationError> {
    for (index, flood) in off.iter().enumerate() {
        let id = serde_json::to_string(flood).unwrap_or_default();
        if !has(*flood) {
            return Err(ValidationError(format!(
                "reactions.{field} names {id}, which {without}"
            )));
        }
        if off[..index].contains(flood) {
            return Err(ValidationError(format!("reactions.{field} names {id} twice")));
        }
    }
    Ok(())
}

/// The latest flood: its preset, a nonce that changes with every flood, and when it started on
/// the engine's clock, so a page that connects after a flood ended does not play it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct FloodState {
    pub flood: Flood,
    pub nonce: u64,
    pub at_ms: u64,
}

impl FloodState {
    /// How long the flood still plays at `now_ms`: zero once it has ended, and never more than
    /// its preset's length.
    pub fn left_ms(&self, now_ms: u64) -> u64 {
        let duration_ms = self.flood.preset().duration_ms;
        let ends_ms = self.at_ms.saturating_add(duration_ms);
        ends_ms.saturating_sub(now_ms).min(duration_ms)
    }
}

impl Engine {
    /// The reactions the vision model sees that watch the camera now; hosts give their conditions
    /// and cues to the vision model along with the project's conditions.
    pub fn active_reactions(&self) -> Vec<&'static Reaction> {
        watching(&self.project)
    }

    /// The latest flood, if any.
    pub fn flood(&self) -> Option<FloodState> {
        self.flood
    }

    /// Plays `flood` at the streamer's request without waiting for the cooldown; floods from
    /// rules and the camera then wait for the cooldown from now.
    pub fn trigger_flood(&mut self, flood: Flood, now_ms: u64) -> Outcome {
        self.start_flood(flood, now_ms);
        self.record(
            now_ms,
            "manual",
            &format!("flooded the overlay with {}", flood.describe()),
        );
        self.bump(Outcome::CHANGED)
    }

    /// Plays `flood` for a rule, the camera, or a trigger phrase unless the last flood started
    /// less than the cooldown ago, in which case it returns how long ago that was.
    pub(crate) fn rule_flood(&mut self, flood: Flood, now_ms: u64) -> Result<(), u64> {
        let cooldown_ms = self.project.reactions.cooldown_ms.max(MIN_FLOOD_COOLDOWN_MS);
        if let Some(last) = self.flood {
            let ago_ms = now_ms.saturating_sub(last.at_ms);
            if ago_ms < cooldown_ms {
                return Err(ago_ms);
            }
        }
        self.start_flood(flood, now_ms);
        Ok(())
    }

    pub(crate) fn start_flood(&mut self, flood: Flood, now_ms: u64) {
        let nonce = self.take_nonce();
        self.flood = Some(FloodState {
            flood,
            nonce,
            at_ms: now_ms,
        });
    }
}
