// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Automatic behavior: presence changes from the camera, rules (the built-in reactions' rules
//! included), and the pause switch.

use crate::engine::Engine;
use crate::model::{Action, ConditionEdge, PresenceTrigger, Trigger};
use crate::presence::Presence;
use crate::reactions;
use crate::types::Outcome;

impl Engine {
    /// Enables or pauses rules. Enabling re-applies the rules for the current state. Pausing makes
    /// the built-in reactions forget what they saw, so a reaction after the pause plays anew.
    pub fn set_automation(&mut self, enabled: bool, now_ms: u64) -> Outcome {
        self.switch_automation(enabled, "manual", now_ms)
    }

    /// [`Engine::set_automation`], logged as coming from `source`.
    pub(crate) fn switch_automation(&mut self, enabled: bool, source: &str, now_ms: u64) -> Outcome {
        if self.project.automation_enabled == enabled {
            return Outcome::NONE;
        }
        self.project.automation_enabled = enabled;
        let verb = if enabled { "resumed" } else { "paused" };
        self.record(now_ms, source, &format!("{verb} automation"));
        if enabled {
            self.rebuild_auto_layer();
        } else {
            self.conditions.retain(|id, _| reactions::builtin(id).is_none());
        }
        self.bump(Outcome::SAVED)
    }

    /// Feeds one detector result: the best face score in a frame taken at `at_ms`.
    pub fn observe(&mut self, score: f64, at_ms: u64) -> Outcome {
        let settings = self.project.detection.clone();
        match self.presence.observe(score, at_ms, &settings) {
            Some(state) => self.presence_changed(state, "camera", at_ms),
            None => Outcome {
                telemetry: true,
                ..Outcome::NONE
            },
        }
    }

    /// Advances time; presence becomes `unknown` when frames stop arriving, conditions turn off
    /// when the vision model has not reported for too long, raised cards past their deck's
    /// auto-hide delay are lowered, and so are cards the camera raised once its signals stop. Votes
    /// a press opened end when their window runs out. A push-to-talk control held too long without
    /// news lets go, subtitles clear a while after they last changed, the live description shows a
    /// line that waited, clears when frames stop, and ages the caption, and voice requests the host
    /// never answered give up.
    pub fn tick(&mut self, now_ms: u64) -> Outcome {
        let settings = self.project.detection.clone();
        let presence = match self.presence.tick(now_ms, &settings) {
            Some(state) => self.presence_changed(state, "camera", now_ms),
            None => Outcome::NONE,
        };
        let conditions = self.expire_conditions(now_ms);
        let cards = self.expire_cards(now_ms);
        let signals = self.expire_signals(now_ms);
        let votes = self.expire_votes(now_ms);
        let talk = self.expire_talk(now_ms);
        let subtitles = self.expire_subtitles(now_ms);
        let seen = self.expire_seen(now_ms);
        let requests = self.expire_requests(now_ms);
        let changed = [
            presence, conditions, cards, signals, votes, talk, subtitles, seen, requests,
        ];
        Outcome {
            changed: changed.iter().any(|outcome| outcome.changed),
            telemetry: seen.telemetry,
            ..Outcome::NONE
        }
    }

    /// Forces a presence change so users can preview and test their rules.
    pub fn simulate_presence(&mut self, state: Presence, now_ms: u64) -> Outcome {
        match self.presence.force(state, now_ms) {
            Some(state) => self.presence_changed(state, "test", now_ms),
            None => Outcome::NONE,
        }
    }

    fn presence_changed(&mut self, state: Presence, source: &str, now_ms: u64) -> Outcome {
        self.record(now_ms, source, &format!("presence is now {state:?}").to_lowercase());
        if let Some(trigger) = presence_trigger(state) {
            self.run_rules(&trigger, "", now_ms);
        }
        self.bump(Outcome::CHANGED)
    }

    /// Runs the enabled rules for one trigger, the active reactions' rules after the project's;
    /// `because` explains it in the event log, which also says when the cooldown held back a flood.
    pub(crate) fn run_rules(&mut self, trigger: &Trigger, because: &str, now_ms: u64) {
        if !self.project.automation_enabled {
            return;
        }
        let builtin = reactions::watching(&self.project)
            .into_iter()
            .map(|reaction| &reaction.rule);
        let rules: Vec<_> = self
            .project
            .rules
            .iter()
            .chain(builtin)
            .filter(|rule| rule.enabled && &rule.when == trigger)
            .cloned()
            .collect();
        for rule in rules {
            let mut held_back = None;
            for action in &rule.actions {
                match action {
                    Action::SetProps { element, props } => {
                        self.auto.entry(element.clone()).or_default().merge(props);
                    }
                    Action::Effect { element, effect } if !self.manual.contains_key(element) => {
                        self.start_effect(element, *effect);
                    }
                    Action::Effect { .. } => {}
                    Action::Flood { flood } => {
                        if let Err(ago_ms) = self.rule_flood(*flood, now_ms) {
                            held_back = Some(ago_ms);
                        }
                    }
                }
            }
            let note = held_back.map_or_else(String::new, |ago_ms| {
                format!(", without its flood: the last one began {} s ago", ago_ms / 1_000)
            });
            self.record(now_ms, "rule", &format!("ran {:?}{because}{note}", rule.name));
        }
    }

    /// Recomputes the automatic layer from the current presence and condition states.
    ///
    /// States are replayed in the order they began, so the most recent change wins where two
    /// rules set the same property. Effects are not replayed. While automation is paused the
    /// layer stays frozen, apart from values for elements that no longer exist.
    pub(crate) fn rebuild_auto_layer(&mut self) {
        if !self.project.automation_enabled {
            let project = &self.project;
            self.auto.retain(|id, _| project.element(id).is_some());
            return;
        }
        self.auto.clear();
        let mut active: Vec<(u64, Trigger)> = Vec::new();
        if let Some(trigger) = presence_trigger(self.presence.state()) {
            active.push((self.presence.since_ms(), trigger));
        }
        for condition in &self.project.conditions {
            let Some(state) = self.conditions.get(&condition.id).filter(|state| state.has_flipped()) else {
                continue;
            };
            let edge = if state.on {
                ConditionEdge::On
            } else {
                ConditionEdge::Off
            };
            active.push((state.since_ms, Trigger::condition(&condition.id, edge)));
        }
        active.sort_by_key(|(since_ms, _)| *since_ms);
        for (_, trigger) in active {
            for rule in self
                .project
                .rules
                .iter()
                .filter(|rule| rule.enabled && rule.when == trigger)
            {
                for action in &rule.actions {
                    if let Action::SetProps { element, props } = action {
                        self.auto.entry(element.clone()).or_default().merge(props);
                    }
                }
            }
        }
    }
}

fn presence_trigger(state: Presence) -> Option<Trigger> {
    match state {
        Presence::Present => Some(Trigger::Presence(PresenceTrigger::Present)),
        Presence::Away => Some(Trigger::Presence(PresenceTrigger::Away)),
        Presence::Unknown => None,
    }
}
