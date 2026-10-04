// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Scene understanding: verdicts from the vision model become condition changes and rule runs.

use crate::engine::Engine;
use crate::model::{ConditionEdge, Trigger};
use crate::reactions;
use crate::scene::{SceneObservation, Verdict};
use crate::template::SCENE_AGE;
use crate::types::{AiStatus, Outcome};

impl Engine {
    /// Applies one analyzed camera image taken at `at_ms`.
    ///
    /// Each verdict updates its condition's debounced state; a condition that turns on or off
    /// runs its rules, and the model's reason is recorded so the streamer can see why. The caption
    /// changes only for an image newer than the last one. Verdicts for unknown or disabled
    /// conditions are ignored, and so are verdicts for a built-in reaction that is not watching;
    /// see [`Engine::observe_verdicts`] for older evidence.
    pub fn observe_scene(&mut self, observation: SceneObservation, at_ms: u64) -> Outcome {
        let observation = observation.sanitized();
        let newest = at_ms >= self.scene.at_ms;
        let mut changed = false;
        if newest {
            changed = self.scene.caption != observation.caption;
            self.scene.caption = observation.caption;
            self.scene.at_ms = at_ms;
            changed |= self.refresh_scene_age() && self.shown_anywhere(SCENE_AGE);
        }
        changed |= self.apply_verdicts(&observation.verdicts, at_ms);
        self.finish(changed, newest)
    }

    /// Applies verdicts without a caption, such as quick judgments from on-device facts.
    ///
    /// A verdict about evidence older than a condition's newest one does not change whether the
    /// condition holds; it can still name what the condition shows when no name is known yet,
    /// because a slower model may describe a moment a quicker one already judged.
    pub fn observe_verdicts(&mut self, verdicts: Vec<Verdict>, at_ms: u64) -> Outcome {
        let observation = SceneObservation {
            caption: String::new(),
            verdicts,
        }
        .sanitized();
        let changed = self.apply_verdicts(&observation.verdicts, at_ms);
        self.finish(changed, true)
    }

    fn finish(&mut self, changed: bool, current: bool) -> Outcome {
        if changed {
            self.bump(Outcome::CHANGED)
        } else if current {
            Outcome {
                telemetry: true,
                ..Outcome::NONE
            }
        } else {
            Outcome::NONE
        }
    }

    /// Judges each condition once; returns whether anything the overlay shows changed.
    fn apply_verdicts(&mut self, verdicts: &[Verdict], at_ms: u64) -> bool {
        self.last_report_ms = self.last_report_ms.max(at_ms);
        let mut changed = false;
        let mut seen = Vec::new();
        let mut flips = Vec::new();
        for verdict in verdicts {
            if seen.contains(&verdict.condition) {
                continue;
            }
            seen.push(verdict.condition.clone());
            let condition = self
                .project
                .condition(&verdict.condition)
                .or_else(|| reactions::watched_condition(&self.project, &verdict.condition));
            let Some(condition) = condition.filter(|c| c.enabled) else {
                continue;
            };
            let state = self.conditions.entry(condition.id.clone()).or_default();
            if state.has_newer_evidence(at_ms) {
                changed |= state.adopt_name(verdict);
                continue;
            }
            let detail_before = state.detail.clone();
            if let Some(on) = state.judge(condition, verdict, at_ms) {
                flips.push((condition.id.clone(), condition.name.clone(), on, state.reason.clone()));
            }
            changed |= state.detail != detail_before;
        }
        for (id, name, on, reason) in flips {
            let (edge, verb) = if on {
                (ConditionEdge::On, "on")
            } else {
                (ConditionEdge::Off, "off")
            };
            let explained = if reason.is_empty() {
                String::new()
            } else {
                format!(": {reason}")
            };
            self.record(at_ms, "ai", &format!("{name} turned {verb}{explained}"));
            let because = format!(" because {name} turned {verb}");
            self.run_rules(&Trigger::condition(&id, edge), &because, at_ms);
            changed = true;
        }
        changed
    }

    /// Turns every condition off and forgets the scene, running the "off" rules of conditions
    /// that were on. Hosts call this when scene understanding stops, so the overlay never keeps
    /// claiming something the model can no longer see.
    pub fn reset_conditions(&mut self, why: &str, now_ms: u64) -> Outcome {
        let mut changed = !self.scene.caption.is_empty();
        self.scene.caption.clear();
        self.scene.age.clear();
        let mut turned_off = Vec::new();
        for (id, state) in &mut self.conditions {
            changed |= state.judged;
            if state.reset(now_ms) {
                turned_off.push(id.clone());
            }
        }
        for id in turned_off {
            let name = self
                .project
                .condition(&id)
                .or_else(|| reactions::builtin(&id).map(|reaction| &reaction.condition))
                .map_or_else(|| id.clone(), |c| c.name.clone());
            self.record(now_ms, "ai", &format!("{name} turned off: {why}"));
            let because = format!(" because {name} turned off");
            self.run_rules(&Trigger::condition(&id, ConditionEdge::Off), &because, now_ms);
        }
        if changed {
            self.bump(Outcome::CHANGED)
        } else {
            Outcome::NONE
        }
    }

    /// Turns conditions off when no verdict arrived for the policy's stale time.
    pub(crate) fn expire_conditions(&mut self, now_ms: u64) -> Outcome {
        let any_on = self.conditions.values().any(|state| state.on);
        let stale = now_ms.saturating_sub(self.last_report_ms) > self.project.ai.stale_after_ms();
        if any_on && stale {
            self.reset_conditions("the vision model stopped reporting", now_ms)
        } else {
            Outcome::NONE
        }
    }

    /// Updates what the vision model is doing; a new state is logged, counters are telemetry.
    pub fn set_ai_status(&mut self, status: AiStatus, now_ms: u64) -> Outcome {
        if self.ai == status {
            return Outcome::NONE;
        }
        let state_changed = self.ai.state != status.state || self.ai.message != status.message;
        let noteworthy = matches!(status.state.as_str(), "failed" | "paused" | "needs_setup");
        if state_changed && noteworthy {
            let detail = format!("vision model {}: {}", status.state.replace('_', " "), status.message);
            self.record(now_ms, "ai", &detail);
        }
        // Watching and thinking alternate on every call; only other transitions are published.
        let routine = |state: &str| matches!(state, "watching" | "thinking");
        let routine_change = routine(&self.ai.state) && routine(&status.state);
        self.ai = status;
        if state_changed && !routine_change {
            self.bump(Outcome::CHANGED)
        } else {
            Outcome {
                telemetry: true,
                ..Outcome::NONE
            }
        }
    }
}
