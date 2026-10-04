// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The live description in the engine: the shown line, the line waiting to replace it, and the
//! caption's age for `{{scene:age}}`.

use super::{MOUTH_LINGER_MS, SEEN_CHANGE_MS, SETTLE_FRAMES, Sighting};
use crate::engine::Engine;
use crate::template::{SCENE_AGE, SEEN, shows_placeholder};
use crate::types::Outcome;

/// What the live description shows and what it waits to show; kept in memory only.
#[derive(Clone, Debug, Default)]
pub(crate) struct LiveDescription {
    /// The line `{{seen}}` shows; empty while camera analysis reports nothing.
    pub(crate) seen: String,
    /// When the shown line last changed.
    changed_ms: Option<u64>,
    /// A line that differs from the shown one, and how many analyzed frames in a row showed it.
    candidate: Option<(String, u32)>,
    /// When the newest analyzed frame was captured.
    observed_ms: Option<u64>,
    /// When the mouth was last seen open.
    mouth_open_ms: Option<u64>,
    /// The latest time the engine was told, for the caption's age.
    clock_ms: u64,
}

impl LiveDescription {
    /// Shows the waiting line once enough frames showed it and the last change is old enough.
    fn promote(&mut self, now_ms: u64) -> bool {
        let settled = self
            .candidate
            .as_ref()
            .is_some_and(|(_, frames)| *frames >= SETTLE_FRAMES);
        let rested = self
            .changed_ms
            .is_none_or(|changed| now_ms >= changed.saturating_add(SEEN_CHANGE_MS));
        if !(settled && rested) {
            return false;
        }
        if let Some((line, _)) = self.candidate.take() {
            self.seen = line;
            self.changed_ms = Some(now_ms);
        }
        true
    }

    /// Forgets every frame; returns whether a line was shown.
    fn clear(&mut self, now_ms: u64) -> bool {
        let shown = !self.seen.is_empty();
        let clock_ms = self.clock_ms;
        *self = LiveDescription {
            changed_ms: shown.then_some(now_ms),
            clock_ms,
            ..LiveDescription::default()
        };
        shown
    }
}

impl Engine {
    /// Feeds what on-device analysis found in a frame captured at `at_ms`, and updates the live
    /// description as the module documentation describes. Frames older than the newest one are
    /// ignored. The snapshot is republished when the shown line changes: at once while an element
    /// shows `{{seen}}`, else with the next telemetry.
    pub fn observe_sighting(&mut self, sighting: &Sighting, at_ms: u64) -> Outcome {
        let live = &mut self.live;
        if live.observed_ms.is_some_and(|newest| at_ms < newest) {
            return Outcome::NONE;
        }
        live.observed_ms = Some(at_ms);
        live.clock_ms = live.clock_ms.max(at_ms);
        if sighting.mouth_open {
            live.mouth_open_ms = Some(at_ms);
        }
        let lingering = live
            .mouth_open_ms
            .is_some_and(|open| at_ms.saturating_sub(open) < MOUTH_LINGER_MS);
        let mouth_open = sighting.mouth_open || (sighting.people > 0 && lingering);
        let line = if mouth_open == sighting.mouth_open {
            sighting.describe()
        } else {
            Sighting {
                mouth_open,
                ..sighting.clone()
            }
            .describe()
        };
        if line == live.seen {
            live.candidate = None;
            return Outcome::NONE;
        }
        let frames = match &live.candidate {
            Some((waiting, frames)) if *waiting == line => frames + 1,
            _ => 1,
        };
        live.candidate = Some((line, frames));
        let changed = live.promote(at_ms);
        self.seen_outcome(changed, SEEN)
    }

    /// The live description that `{{seen}}` shows now.
    pub fn seen(&self) -> &str {
        &self.live.seen
    }

    /// Advances the live description to `now_ms`: shows a line that waited for the least time
    /// between changes, clears the line once frames stop for the detection stale time, and ages
    /// the caption.
    pub(crate) fn expire_seen(&mut self, now_ms: u64) -> Outcome {
        let stale_ms = self.project.detection.effective_stale_ms();
        let live = &mut self.live;
        live.clock_ms = live.clock_ms.max(now_ms);
        let stopped = live
            .observed_ms
            .is_some_and(|observed| now_ms.saturating_sub(observed) > stale_ms);
        let changed = if stopped {
            live.clear(now_ms)
        } else {
            live.promote(now_ms)
        };
        let seen = self.seen_outcome(changed, SEEN);
        let aged = self.refresh_scene_age();
        let aged = self.seen_outcome(aged, SCENE_AGE);
        Outcome {
            changed: seen.changed || aged.changed,
            telemetry: seen.telemetry || aged.telemetry,
            ..Outcome::NONE
        }
    }

    /// Clears the live description, as when camera analysis stops; true when a line was shown.
    pub(crate) fn forget_seen(&mut self, now_ms: u64) -> bool {
        self.live.clear(now_ms)
    }

    /// Updates what `{{scene:age}}` says about the caption's age from the engine's clock; true when
    /// it changed while there is a caption.
    pub(crate) fn refresh_scene_age(&mut self) -> bool {
        let age = if self.scene.caption.is_empty() {
            String::new()
        } else {
            age_label(self.live.clock_ms.saturating_sub(self.scene.at_ms))
        };
        if self.scene.age == age {
            return false;
        }
        self.scene.age = age;
        !self.scene.caption.is_empty()
    }

    /// Whether the text of any element, as rules, cards, and holds leave it, uses `placeholder`.
    pub(crate) fn shown_anywhere(&self, placeholder: &str) -> bool {
        self.project.elements.iter().any(|element| {
            self.layered_props(&element.id)
                .is_some_and(|props| shows_placeholder(&props.text, placeholder))
        })
    }

    /// A change the overlay shows when an element's text uses `placeholder`, else telemetry.
    fn seen_outcome(&mut self, changed: bool, placeholder: &str) -> Outcome {
        if !changed {
            Outcome::NONE
        } else if self.shown_anywhere(placeholder) {
            self.bump(Outcome::CHANGED)
        } else {
            Outcome {
                telemetry: true,
                ..Outcome::NONE
            }
        }
    }
}

/// How long ago a camera image was taken, in steps that change at most every ten seconds: "just
/// now", "10 s ago", "2 min ago", or "over an hour ago".
fn age_label(age_ms: u64) -> String {
    let seconds = age_ms / 1_000;
    match seconds {
        0..10 => "just now".to_owned(),
        10..60 => format!("{} s ago", seconds / 10 * 10),
        60..3_600 => format!("{} min ago", seconds / 60),
        _ => "over an hour ago".to_owned(),
    }
}
