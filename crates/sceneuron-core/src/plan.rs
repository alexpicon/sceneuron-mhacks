// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Typed project edits proposed by the AI assistant and reviewed by the streamer before saving.
//!
//! A plan never touches the live state directly: [`EditPlan::apply`] returns a new project that
//! must pass validation, and the editor saves it like any other edit.

use serde::{Deserialize, Serialize};

use crate::decks::Deck;
use crate::model::{Condition, Element, Project, Rule, Trigger, ValidationError};
use crate::props::PropsPatch;

/// Upper bound on operations in one plan, so a runaway answer cannot rewrite everything.
pub const MAX_PLAN_OPERATIONS: usize = 40;

/// One change to the project.
// Plans hold a few dozen operations at most, so boxing the larger variants buys nothing.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum EditOp {
    AddElement {
        element: Element,
    },
    /// Changes an element's name and the properties the patch sets.
    UpdateElement {
        id: String,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        props: PropsPatch,
    },
    /// Removes an element, every rule action that uses it, and the deck it shows.
    RemoveElement {
        id: String,
    },
    AddCondition {
        condition: Condition,
    },
    /// Replaces the condition with the same id.
    UpdateCondition {
        condition: Condition,
    },
    /// Removes a condition and every rule it triggers.
    RemoveCondition {
        id: String,
    },
    AddRule {
        rule: Rule,
    },
    /// Replaces the rule with the same id.
    UpdateRule {
        rule: Rule,
    },
    RemoveRule {
        id: String,
    },
    AddDeck {
        deck: Deck,
    },
    /// Replaces the deck with the same id; only the element that shows it cannot change.
    UpdateDeck {
        deck: Deck,
    },
    /// Removes a deck; the element that showed it stays.
    RemoveDeck {
        id: String,
    },
}

/// A summary for the streamer plus the operations that carry it out.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EditPlan {
    pub summary: String,
    pub operations: Vec<EditOp>,
}

impl EditPlan {
    /// Applies every operation in order to a copy of `project` and validates the result.
    pub fn apply(&self, project: &Project) -> Result<Project, ValidationError> {
        if self.operations.len() > MAX_PLAN_OPERATIONS {
            let message = format!("a plan may have at most {MAX_PLAN_OPERATIONS} operations");
            return Err(ValidationError(message));
        }
        let mut next = project.clone();
        for (index, operation) in self.operations.iter().enumerate() {
            apply_one(&mut next, operation)
                .map_err(|error| ValidationError(format!("operation {}: {}", index + 1, error.0)))?;
        }
        next.validate()?;
        Ok(next)
    }
}

fn apply_one(project: &mut Project, operation: &EditOp) -> Result<(), ValidationError> {
    match operation {
        EditOp::AddElement { element } => {
            refuse_existing("element", &element.id, project.element(&element.id).is_some())?;
            project.elements.push(element.clone());
        }
        EditOp::UpdateElement { id, name, props } => {
            let element = find(&mut project.elements, |e| &e.id == id, "element", id)?;
            if let Some(name) = name {
                element.name.clone_from(name);
            }
            element.props.apply(props);
        }
        EditOp::RemoveElement { id } => {
            remove(&mut project.elements, |e| &e.id == id, "element", id)?;
            for rule in &mut project.rules {
                rule.actions.retain(|action| action.element() != Some(id.as_str()));
            }
            project.rules.retain(|rule| !rule.actions.is_empty());
            project.decks.retain(|deck| &deck.element != id);
        }
        EditOp::AddCondition { condition } => {
            refuse_existing("condition", &condition.id, project.condition(&condition.id).is_some())?;
            project.conditions.push(condition.clone());
        }
        EditOp::UpdateCondition { condition } => {
            let id = &condition.id;
            *find(&mut project.conditions, |c| &c.id == id, "condition", id)? = condition.clone();
        }
        EditOp::RemoveCondition { id } => {
            remove(&mut project.conditions, |c| &c.id == id, "condition", id)?;
            project
                .rules
                .retain(|rule| !matches!(&rule.when, Trigger::Condition { condition, .. } if condition == id));
        }
        EditOp::AddRule { rule } => {
            let exists = project.rules.iter().any(|r| r.id == rule.id);
            refuse_existing("rule", &rule.id, exists)?;
            project.rules.push(rule.clone());
        }
        EditOp::UpdateRule { rule } => {
            let id = &rule.id;
            *find(&mut project.rules, |r| &r.id == id, "rule", id)? = rule.clone();
        }
        EditOp::RemoveRule { id } => remove(&mut project.rules, |r| &r.id == id, "rule", id)?,
        EditOp::AddDeck { deck } => {
            refuse_existing("deck", &deck.id, project.deck(&deck.id).is_some())?;
            project.decks.push(deck.clone());
        }
        EditOp::UpdateDeck { deck } => {
            let id = &deck.id;
            let existing = find(&mut project.decks, |d| &d.id == id, "deck", id)?;
            if existing.element != deck.element {
                let message = format!("deck {id:?} must keep showing element {:?}", existing.element);
                return Err(ValidationError(message));
            }
            *existing = deck.clone();
        }
        EditOp::RemoveDeck { id } => remove(&mut project.decks, |d| &d.id == id, "deck", id)?,
    }
    Ok(())
}

fn refuse_existing(kind: &str, id: &str, exists: bool) -> Result<(), ValidationError> {
    if exists {
        Err(ValidationError(format!("{kind} {id:?} already exists")))
    } else {
        Ok(())
    }
}

fn find<'a, T>(
    items: &'a mut [T],
    matches: impl Fn(&T) -> bool,
    kind: &str,
    id: &str,
) -> Result<&'a mut T, ValidationError> {
    items
        .iter_mut()
        .find(|item| matches(item))
        .ok_or_else(|| ValidationError(format!("unknown {kind} {id:?}")))
}

fn remove<T>(items: &mut Vec<T>, matches: impl Fn(&T) -> bool, kind: &str, id: &str) -> Result<(), ValidationError> {
    let before = items.len();
    items.retain(|item| !matches(item));
    if items.len() == before {
        return Err(ValidationError(format!("unknown {kind} {id:?}")));
    }
    Ok(())
}
