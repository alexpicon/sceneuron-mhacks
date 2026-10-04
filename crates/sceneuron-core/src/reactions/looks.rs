// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The reactions the vision model sees: a built-in condition it judges only while on-device
//! analysis reports the condition's [`Cue`], and a built-in rule that plays the preset when the
//! condition turns on. Sticking out a tongue, once the mouth is open, plays the tongue preset.

use std::sync::LazyLock;

use super::presets::Flood;
use crate::model::{Action, Condition, ConditionEdge, Project, Rule, Trigger};

/// The id of the tongue-out reaction's condition. Project ids cannot hold a colon, so no
/// condition of the streamer's can take it.
pub const TONGUE_OUT: &str = "reaction:tongue-out";

/// A condition the vision model judges only while on-device analysis reports a fact, such as
/// `"mouth open"`. While the fact is missing, the condition cannot hold: nobody is asked about it
/// and no look is made for it alone. Facts cannot show what it is about, so quick judgments
/// leave it to the vision model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cue {
    /// The condition's id.
    pub condition: String,
    /// The fact the condition waits for.
    pub fact: String,
}

impl Cue {
    /// Whether `facts` report the cue: one of them is the fact, alone or followed by a measure in
    /// parentheses, as in `mouth open (0.21)`.
    pub fn seen_in(&self, facts: &[String]) -> bool {
        facts.iter().any(|fact| {
            fact.strip_prefix(self.fact.as_str())
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(" ("))
        })
    }
}

/// A reaction the vision model sees: its preset, the condition the model judges, the fact it
/// waits for, and the rule that plays the preset.
#[derive(Clone, Debug, PartialEq)]
pub struct Reaction {
    pub flood: Flood,
    pub condition: Condition,
    pub fact: &'static str,
    pub rule: Rule,
}

impl Reaction {
    /// The cue the vision model's director waits for.
    pub fn cue(&self) -> Cue {
        Cue {
            condition: self.condition.id.clone(),
            fact: self.fact.to_owned(),
        }
    }
}

static REACTIONS: LazyLock<Vec<Reaction>> = LazyLock::new(|| vec![tongue_out()]);

fn tongue_out() -> Reaction {
    let condition = Condition {
        id: TONGUE_OUT.to_owned(),
        name: "Tongue out".to_owned(),
        prompt: "Someone in view is sticking their tongue out".to_owned(),
        detail: String::new(),
        enabled: true,
        min_confidence: 0.7,
        confirm: 1,
        release: 1,
    };
    let rule = Rule {
        id: TONGUE_OUT.to_owned(),
        name: "Tongue-out flood".to_owned(),
        enabled: true,
        when: Trigger::condition(TONGUE_OUT, ConditionEdge::On),
        actions: vec![Action::Flood { flood: Flood::Tongue }],
    };
    Reaction {
        flood: Flood::Tongue,
        condition,
        fact: "mouth open",
        rule,
    }
}

/// Every reaction the vision model sees, whether or not it watches the camera now: one for each
/// preset whose camera is [`Camera::Look`](super::Camera::Look).
pub fn reactions() -> &'static [Reaction] {
    &REACTIONS
}

/// The reaction whose condition is `id`.
pub(crate) fn builtin(id: &str) -> Option<&'static Reaction> {
    reactions().iter().find(|reaction| reaction.condition.id == id)
}

/// The reactions that watch the camera for `project`: while its fun reactions and automation run,
/// those whose preset's camera is not switched off.
pub(crate) fn watching(project: &Project) -> Vec<&'static Reaction> {
    reactions()
        .iter()
        .filter(|reaction| project.automation_enabled && project.reactions.watches(reaction.flood))
        .collect()
}

/// The condition `id` of a reaction that watches the camera for `project`.
pub(crate) fn watched_condition(project: &Project, id: &str) -> Option<&'static Condition> {
    watching(project)
        .into_iter()
        .find(|reaction| reaction.condition.id == id)
        .map(|reaction| &reaction.condition)
}
