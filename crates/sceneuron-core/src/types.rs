// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Values the engine reports: outcomes, vision status, effects, recent events, and errors.

use serde::Serialize;

use crate::model::{Effect, ValidationError};

/// What a state change requires from the host.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// The effective overlay state changed; publish a new snapshot.
    pub changed: bool,
    /// The saved project changed; persist it.
    pub project_dirty: bool,
    /// Only live telemetry (scores, timings) changed.
    pub telemetry: bool,
}

impl Outcome {
    pub(crate) const CHANGED: Outcome = Outcome {
        changed: true,
        project_dirty: false,
        telemetry: false,
    };
    pub(crate) const SAVED: Outcome = Outcome {
        changed: true,
        project_dirty: true,
        telemetry: false,
    };
    pub(crate) const NONE: Outcome = Outcome {
        changed: false,
        project_dirty: false,
        telemetry: false,
    };
}

/// Health of the camera-analysis pipeline, reported by the host.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum VisionStatus {
    /// No camera filter is sending frames.
    #[default]
    Idle,
    Starting,
    Running {
        latency_ms: f64,
        /// Whether analysis measures how far apart the lips are, which the tongue-out reaction
        /// waits for.
        mouths: bool,
        /// Whether analysis locates hands and their joints, which the facepalm needs.
        hands: bool,
    },
    Failed {
        message: String,
    },
}

/// The last one-shot effect requested for an element; the nonce changes per request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct EffectState {
    pub effect: Effect,
    pub nonce: u64,
}

/// A recent change, shown in the editor so users can see why the overlay changed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EventRecord {
    pub at_ms: u64,
    pub source: String,
    pub detail: String,
}

/// A request that names a missing element, deck, or card, or carries invalid values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineError {
    UnknownElement(String),
    EmptyPatch,
    Invalid(ValidationError),
    UnknownDeck(String),
    UnknownCard {
        deck: String,
        card: String,
    },
    /// Raising or toggling needs a card.
    MissingCard,
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::UnknownElement(id) => write!(f, "unknown element {id:?}"),
            EngineError::EmptyPatch => f.write_str("the change sets no property"),
            EngineError::Invalid(error) => error.fmt(f),
            EngineError::UnknownDeck(id) => write!(f, "unknown deck {id:?}"),
            EngineError::UnknownCard { deck, card } => write!(f, "deck {deck:?} has no card {card:?}"),
            EngineError::MissingCard => f.write_str("name the card to raise or toggle"),
        }
    }
}

impl std::error::Error for EngineError {}

/// What the vision model is doing, reported by the host for the editor.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AiStatus {
    /// One of `off`, `needs_setup`, `watching`, `thinking`, `paused`, or `failed`.
    pub state: String,
    /// A human-readable note: why it is paused or failed, or what needs setting up.
    pub message: String,
    pub provider: String,
    pub model: String,
    pub calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// Estimated spend so far in US dollars.
    pub cost_usd: f64,
    /// Estimated spend in the last hour, which the policy's hourly cap limits.
    pub hour_cost_usd: f64,
    pub last_latency_ms: Option<f64>,
    /// Quick judgments from on-device facts: `off`, `watching`, or `failed`.
    pub quick_state: String,
    /// Why quick judgments are off or failed, or what they last did.
    pub quick_message: String,
    pub quick_calls: u64,
    pub quick_latency_ms: Option<f64>,
    /// Characters sent to text to speech for spoken cards.
    pub voice_characters: u64,
    /// Seconds of microphone audio sent for transcription.
    pub voice_seconds: f64,
    /// Estimated voice spend in US dollars, also counted in `cost_usd` and `hour_cost_usd`.
    pub voice_cost_usd: f64,
    /// Subtitle lines sent for translation.
    pub translation_calls: u64,
    /// Tokens subtitle translations read, cached or not.
    pub translation_input_tokens: u64,
    pub translation_output_tokens: u64,
    /// Estimated translation spend in US dollars, also counted in `cost_usd` and `hour_cost_usd`.
    pub translation_cost_usd: f64,
}

impl Default for AiStatus {
    fn default() -> Self {
        Self {
            state: "off".to_owned(),
            message: String::new(),
            provider: String::new(),
            model: String::new(),
            calls: 0,
            input_tokens: 0,
            output_tokens: 0,
            cost_usd: 0.0,
            hour_cost_usd: 0.0,
            last_latency_ms: None,
            quick_state: "off".to_owned(),
            quick_message: String::new(),
            quick_calls: 0,
            quick_latency_ms: None,
            voice_characters: 0,
            voice_seconds: 0.0,
            voice_cost_usd: 0.0,
            translation_calls: 0,
            translation_input_tokens: 0,
            translation_output_tokens: 0,
            translation_cost_usd: 0.0,
        }
    }
}
