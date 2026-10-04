// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Project validation: limits, id and name rules, property values, conditions, references, decks,
//! and fun reactions.

use std::collections::BTreeSet;

use crate::model::{Action, Condition, Project, SCHEMA_VERSION, Trigger, ValidationError};
use crate::settings::{AiPolicy, DetectionSettings};

const MAX_ELEMENTS: usize = 64;
const MAX_CONDITIONS: usize = 16;
const MAX_RULES: usize = 64;
const MAX_ACTIONS: usize = 16;
const MAX_ID_CHARS: usize = 40;
const MAX_NAME_CHARS: usize = 80;
const MAX_PROMPT_CHARS: usize = 300;
const MAX_DETAIL_CHARS: usize = 120;
/// Placeholder names for the scene caption, the live description, the shared room's summary, live
/// subtitles, and OBS's program, so no condition may use them as ids.
pub(crate) const RESERVED_PLACEHOLDERS: [(&str, &str); 6] = [
    ("scene", "the scene caption"),
    ("seen", "the live description"),
    ("room", "the room summary"),
    ("subtitles", "live subtitles"),
    ("on-air", "OBS's scene and whether it is live"),
    ("on-air-time", "the time on air"),
];

impl Project {
    /// Checks limits, id uniqueness, property values, conditions, rule references, decks, and the
    /// fun reactions' cooldown.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.version != SCHEMA_VERSION {
            let message = format!("unsupported project version {}", self.version);
            return Err(ValidationError(message));
        }
        let canvas_ok = |side: u32| (16..=8192).contains(&side);
        if !canvas_ok(self.canvas.width) || !canvas_ok(self.canvas.height) {
            return Err(ValidationError("canvas sides must be 16 to 8192 pixels".to_owned()));
        }
        check_count("elements", self.elements.len(), MAX_ELEMENTS)?;
        check_count("conditions", self.conditions.len(), MAX_CONDITIONS)?;
        check_count("rules", self.rules.len(), MAX_RULES)?;
        let mut ids = BTreeSet::new();
        for element in &self.elements {
            check_id(&element.id)?;
            check_name(&element.name)?;
            if !ids.insert(element.id.as_str()) {
                let message = format!("duplicate element id {:?}", element.id);
                return Err(ValidationError(message));
            }
            element.props.validate().map_err(|error| prefix(&element.id, error))?;
        }
        let mut condition_ids = BTreeSet::new();
        for condition in &self.conditions {
            condition.validate()?;
            if !condition_ids.insert(condition.id.as_str()) {
                let message = format!("duplicate condition id {:?}", condition.id);
                return Err(ValidationError(message));
            }
        }
        let mut rule_ids = BTreeSet::new();
        for rule in &self.rules {
            check_id(&rule.id)?;
            check_name(&rule.name)?;
            if !rule_ids.insert(rule.id.as_str()) {
                let message = format!("duplicate rule id {:?}", rule.id);
                return Err(ValidationError(message));
            }
            if let Trigger::Condition { condition, .. } = &rule.when
                && !condition_ids.contains(condition.as_str())
            {
                let message = format!("rule {:?} watches unknown condition {condition:?}", rule.id);
                return Err(ValidationError(message));
            }
            check_count("actions per rule", rule.actions.len(), MAX_ACTIONS)?;
            for action in &rule.actions {
                if let Some(element) = action.element()
                    && !ids.contains(element)
                {
                    let message = format!("rule {:?} uses unknown element {element:?}", rule.id);
                    return Err(ValidationError(message));
                }
                if let Action::SetProps { props, .. } = action {
                    props.validate().map_err(|error| prefix(&rule.id, error))?;
                }
            }
        }
        self.validate_decks()?;
        self.reactions.validate()?;
        self.detection.validate()?;
        self.ai.validate()
    }
}

impl Condition {
    fn validate(&self) -> Result<(), ValidationError> {
        check_id(&self.id)?;
        if let Some((name, meaning)) = RESERVED_PLACEHOLDERS.iter().find(|(name, _)| *name == self.id) {
            let message = format!("condition id {name:?} is reserved for {meaning}");
            return Err(ValidationError(message));
        }
        check_name(&self.name)?;
        check_text(&self.id, "prompt", &self.prompt, 1, MAX_PROMPT_CHARS)?;
        check_text(&self.id, "detail", &self.detail, 0, MAX_DETAIL_CHARS)?;
        if !in_range(self.min_confidence, 0.0, 1.0) {
            return Err(prefix(
                &self.id,
                ValidationError("min_confidence must be 0 to 1".to_owned()),
            ));
        }
        if !(1..=10).contains(&self.confirm) || !(1..=10).contains(&self.release) {
            return Err(prefix(
                &self.id,
                ValidationError("confirm and release must be 1 to 10".to_owned()),
            ));
        }
        Ok(())
    }
}

impl AiPolicy {
    fn validate(&self) -> Result<(), ValidationError> {
        if !(1_000..=600_000).contains(&self.min_interval_ms) {
            return Err(ValidationError("ai.min_interval_ms must be 1000 to 600000".to_owned()));
        }
        if !(self.min_interval_ms..=3_600_000).contains(&self.refresh_ms) {
            let message = "ai.refresh_ms must be at least ai.min_interval_ms and at most 3600000";
            return Err(ValidationError(message.to_owned()));
        }
        if !in_range(self.change_threshold, 0.0, 1.0) {
            return Err(ValidationError("ai.change_threshold must be 0 to 1".to_owned()));
        }
        if !(1..=60).contains(&self.max_calls_per_minute) {
            return Err(ValidationError("ai.max_calls_per_minute must be 1 to 60".to_owned()));
        }
        if !in_range(self.max_cost_per_hour, 0.0, 100.0) {
            return Err(ValidationError("ai.max_cost_per_hour must be 0 to 100".to_owned()));
        }
        Ok(())
    }
}

impl DetectionSettings {
    fn validate(&self) -> Result<(), ValidationError> {
        if !in_range(self.min_confidence, 0.05, 0.99) {
            return Err(ValidationError("min_confidence must be 0.05 to 0.99".to_owned()));
        }
        if !in_range(self.sample_fps, 0.5, 30.0) {
            return Err(ValidationError("sample_fps must be 0.5 to 30".to_owned()));
        }
        let durations = [self.present_after_ms, self.away_after_ms, self.stale_after_ms];
        if durations.iter().any(|ms| *ms > 600_000) {
            return Err(ValidationError("detection delays must be at most 600000 ms".to_owned()));
        }
        Ok(())
    }
}

fn in_range(value: f64, min: f64, max: f64) -> bool {
    value.is_finite() && (min..=max).contains(&value)
}

pub(crate) fn check_count(what: &str, count: usize, max: usize) -> Result<(), ValidationError> {
    if count <= max {
        Ok(())
    } else {
        Err(ValidationError(format!("at most {max} {what} are allowed")))
    }
}

/// Ids are 1-40 characters of lowercase ASCII letters, digits, `-`, or `_`.
pub(crate) fn check_id(id: &str) -> Result<(), ValidationError> {
    let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_';
    if !id.is_empty() && id.len() <= MAX_ID_CHARS && id.chars().all(allowed) {
        Ok(())
    } else {
        let message = format!("id {id:?} must be 1-{MAX_ID_CHARS} characters of a-z, 0-9, - or _");
        Err(ValidationError(message))
    }
}

pub(crate) fn check_name(name: &str) -> Result<(), ValidationError> {
    let count = name.chars().count();
    if (1..=MAX_NAME_CHARS).contains(&count) && !name.chars().any(char::is_control) {
        Ok(())
    } else {
        let message = format!("name {name:?} must be 1-{MAX_NAME_CHARS} printable characters");
        Err(ValidationError(message))
    }
}

/// Free text sent to the vision model: bounded length and no control characters.
fn check_text(owner: &str, field: &str, text: &str, min: usize, max: usize) -> Result<(), ValidationError> {
    let count = text.chars().count();
    if (min..=max).contains(&count) && !text.chars().any(char::is_control) {
        Ok(())
    } else {
        let message = format!("{owner}: {field} must be {min}-{max} characters on one line");
        Err(ValidationError(message))
    }
}

pub(crate) fn prefix(owner: &str, error: ValidationError) -> ValidationError {
    ValidationError(format!("{owner}: {}", error.0))
}
