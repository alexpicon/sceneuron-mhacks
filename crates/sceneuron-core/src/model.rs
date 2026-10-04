// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The persisted project: canvas, theme, overlay elements, AI conditions, rules, card decks, fun
//! reactions, and settings.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::decks::Deck;
use crate::props::{ElementProps, PropsPatch};
use crate::reactions::{Flood, Reactions};
use crate::settings::{AiPolicy, DetectionSettings};
use crate::theme::Theme;

/// Version of the persisted project format written by this build. Versions 1 to 3 load too.
pub const SCHEMA_VERSION: u32 = 4;

/// A project value that breaks a documented limit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationError(pub String);

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ValidationError {}

/// Overlay canvas size in pixels; overlay sources render at this size by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
}

impl Default for Canvas {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
        }
    }
}

/// How the overlay draws an element.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ElementKind {
    /// Text on an optional background.
    Text,
    /// A filled rectangle.
    Box,
    /// A border with a transparent inside, such as a camera frame.
    Frame,
}

/// One overlay graphic with its base (saved) properties.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub id: String,
    pub name: String,
    pub kind: ElementKind,
    #[serde(default)]
    pub props: ElementProps,
}

/// Something the vision model watches for, described in plain words by the streamer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Condition {
    pub id: String,
    pub name: String,
    /// What the vision model should decide is true or false in the camera image.
    pub prompt: String,
    /// What to report while the condition holds, such as "the product's name"; empty for nothing.
    #[serde(default)]
    pub detail: String,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    /// Minimum model confidence for a judgment to count as "holds".
    #[serde(default = "default_condition_confidence")]
    pub min_confidence: f64,
    /// Consecutive "holds" judgments before the condition turns on.
    #[serde(default = "one")]
    pub confirm: u32,
    /// Consecutive "does not hold" judgments before the condition turns off.
    #[serde(default = "two")]
    pub release: u32,
}

/// A presence change from the local face detector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PresenceTrigger {
    Away,
    Present,
}

/// Whether a condition turned on or off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConditionEdge {
    On,
    Off,
}

/// What starts a rule. JSON: `"away"`, `"present"`, or `{"condition": id, "state": "on"}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Trigger {
    Presence(PresenceTrigger),
    Condition { condition: String, state: ConditionEdge },
}

impl Trigger {
    pub fn away() -> Trigger {
        Trigger::Presence(PresenceTrigger::Away)
    }

    pub fn present() -> Trigger {
        Trigger::Presence(PresenceTrigger::Present)
    }

    pub fn condition(id: &str, state: ConditionEdge) -> Trigger {
        Trigger::Condition {
            condition: id.to_owned(),
            state,
        }
    }
}

/// A one-shot attention animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Effect {
    Shake,
    Pop,
    Flash,
    Bounce,
}

/// A typed change a rule makes to the shared overlay state.
// Rules hold a handful of actions, so boxing the larger variant buys nothing.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    SetProps {
        element: String,
        props: PropsPatch,
    },
    Effect {
        element: String,
        effect: Effect,
    },
    /// Plays a reaction preset: floods the whole overlay with its emojis for a few seconds, its
    /// banner line in the middle if it has one, above every element. Rules play floods at least the
    /// fun reactions' cooldown apart.
    Flood {
        flood: Flood,
    },
}

impl Action {
    /// The element this action changes; `None` for a flood, which covers the whole overlay.
    pub fn element(&self) -> Option<&str> {
        match self {
            Action::SetProps { element, .. } | Action::Effect { element, .. } => Some(element),
            Action::Flood { .. } => None,
        }
    }
}

/// Runs its actions when its trigger happens, while automation is enabled.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    pub when: Trigger,
    pub actions: Vec<Action>,
}

/// Everything Sceneuron saves: layout, theme, conditions, rules, card decks, fun reactions, settings,
/// and automation mode.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Project {
    pub version: u32,
    pub canvas: Canvas,
    /// How the overlay draws every element; projects saved before themes existed load with `None`.
    pub theme: Theme,
    pub elements: Vec<Element>,
    pub conditions: Vec<Condition>,
    pub rules: Vec<Rule>,
    pub decks: Vec<Deck>,
    pub reactions: Reactions,
    pub detection: DetectionSettings,
    pub ai: AiPolicy,
    pub automation_enabled: bool,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            version: SCHEMA_VERSION,
            canvas: Canvas::default(),
            theme: Theme::None,
            elements: Vec::new(),
            conditions: Vec::new(),
            rules: Vec::new(),
            decks: Vec::new(),
            reactions: Reactions::default(),
            detection: DetectionSettings::default(),
            ai: AiPolicy::default(),
            automation_enabled: true,
        }
    }
}

impl Project {
    /// Looks up an element by id.
    pub fn element(&self, id: &str) -> Option<&Element> {
        self.elements.iter().find(|element| element.id == id)
    }

    /// Looks up a condition by id.
    pub fn condition(&self, id: &str) -> Option<&Condition> {
        self.conditions.iter().find(|condition| condition.id == id)
    }
}

fn enabled_by_default() -> bool {
    true
}

fn default_condition_confidence() -> f64 {
    0.6
}

fn one() -> u32 {
    1
}

fn two() -> u32 {
    2
}
