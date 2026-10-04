// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What the vision model reports about the camera, and the debounced state of each condition.

use serde::{Deserialize, Serialize};

use crate::model::Condition;

const MAX_DETAIL_CHARS: usize = 80;
const MAX_REASON_CHARS: usize = 200;
const MAX_CAPTION_CHARS: usize = 200;

/// The model's judgment of one condition in one camera image.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Verdict {
    pub condition: String,
    pub holds: bool,
    /// Model confidence from 0 to 1.
    pub confidence: f64,
    /// The requested detail, such as an object's name; empty when there is none.
    #[serde(default)]
    pub detail: String,
    /// A short explanation, shown to the streamer so decisions are never a black box.
    #[serde(default)]
    pub reason: String,
}

/// One analyzed camera image: a short description plus one verdict per condition.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SceneObservation {
    pub caption: String,
    pub verdicts: Vec<Verdict>,
}

impl SceneObservation {
    /// Bounds free text from the model before it reaches the overlay or the editor.
    pub fn sanitized(mut self) -> SceneObservation {
        self.caption = clean(&self.caption, MAX_CAPTION_CHARS);
        for verdict in &mut self.verdicts {
            verdict.detail = clean(&verdict.detail, MAX_DETAIL_CHARS);
            verdict.reason = clean(&verdict.reason, MAX_REASON_CHARS);
            verdict.confidence = if verdict.confidence.is_finite() {
                verdict.confidence.clamp(0.0, 1.0)
            } else {
                0.0
            };
        }
        self
    }
}

/// Keeps printable characters on one line and caps the length.
fn clean(text: &str, max_chars: usize) -> String {
    let flat: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    flat.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_chars)
        .collect()
}

/// Debounced state of one condition across successive judgments.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ConditionState {
    pub on: bool,
    /// Whether the condition has ever been judged since the project loaded.
    pub judged: bool,
    pub since_ms: u64,
    pub confidence: f64,
    pub detail: String,
    pub reason: String,
    #[serde(skip)]
    positives: u32,
    #[serde(skip)]
    negatives: u32,
    #[serde(skip)]
    flipped: bool,
    /// Time of the evidence behind the latest judgment.
    #[serde(skip)]
    evidence_ms: u64,
}

impl ConditionState {
    /// Applies one verdict and returns the new state when it flips.
    pub fn judge(&mut self, condition: &Condition, verdict: &Verdict, at_ms: u64) -> Option<bool> {
        self.judged = true;
        self.evidence_ms = at_ms;
        self.confidence = verdict.confidence;
        self.reason = verdict.reason.clone();
        let holds = verdict.holds && verdict.confidence >= condition.min_confidence;
        if holds {
            self.positives += 1;
            self.negatives = 0;
            if !verdict.detail.is_empty() {
                self.detail = verdict.detail.clone();
            }
        } else {
            self.negatives += 1;
            self.positives = 0;
        }
        let flip = if !self.on && self.positives >= condition.confirm.max(1) {
            Some(true)
        } else if self.on && self.negatives >= condition.release.max(1) {
            Some(false)
        } else {
            None
        };
        if let Some(on) = flip {
            self.on = on;
            self.since_ms = at_ms;
            self.flipped = true;
        }
        flip
    }

    /// Whether a judgment already rests on evidence newer than `at_ms`.
    pub(crate) fn has_newer_evidence(&self, at_ms: u64) -> bool {
        self.judged && self.evidence_ms > at_ms
    }

    /// Takes the name from an older verdict that saw the condition hold, while the condition is
    /// on and no name is known; returns whether the name changed.
    pub(crate) fn adopt_name(&mut self, verdict: &Verdict) -> bool {
        if self.on && verdict.holds && self.detail.is_empty() && !verdict.detail.is_empty() {
            self.detail.clone_from(&verdict.detail);
            return true;
        }
        false
    }

    /// Forgets every judgment, turning the condition off at `at_ms` if it was on.
    ///
    /// Returns whether it was on. The time of the last change is kept, so the automatic layer
    /// can still replay the condition's last rules.
    pub(crate) fn reset(&mut self, at_ms: u64) -> bool {
        let was_on = self.on;
        let since_ms = if was_on { at_ms } else { self.since_ms };
        let flipped = self.flipped || was_on;
        *self = ConditionState {
            since_ms,
            flipped,
            ..ConditionState::default()
        };
        was_on
    }

    /// Whether the condition has turned on or off at least once, so its rules have run.
    pub fn has_flipped(&self) -> bool {
        self.flipped
    }
}

/// The latest scene description, for the editor and for `{{scene}}` and `{{scene:age}}` in overlay
/// text.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct SceneView {
    pub caption: String,
    /// When the camera image the caption describes was taken.
    pub at_ms: u64,
    /// How long ago that was, in steps of ten seconds or more, such as "just now" or "20 s ago";
    /// empty while there is no caption.
    pub age: String,
}
