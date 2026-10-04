// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Changes to saved projects that a schema version does not mark: a condition saved with an id
//! that later became a placeholder name, such as "room" for the shared room's summary, is renamed
//! to a free id, with the rules that watch it and its placeholders in every text; and a theme this
//! build does not draw, such as one a newer Sceneuron added, becomes none.

use serde_json::Value;

use crate::model::{Action, Project, Trigger};
use crate::theme::Theme;
use crate::validate::RESERVED_PLACEHOLDERS;

/// Sets a saved theme this build does not know to none, so the rest of the project still loads;
/// returns a note when it did.
pub(crate) fn forget_unknown_theme(project: &mut Value) -> Option<String> {
    let theme = project.get_mut("theme")?;
    if theme.as_str().and_then(Theme::from_id).is_some() {
        return None;
    }
    let note = format!("the overlay theme {theme} is not one this build draws, so it shows none");
    *theme = Value::String(Theme::None.id().to_owned());
    Some(note)
}

/// Renames every condition whose id is a reserved placeholder name; returns a note for each.
pub(crate) fn free_reserved_ids(project: &mut Project) -> Vec<String> {
    let mut notes = Vec::new();
    for index in 0..project.conditions.len() {
        let old = project.conditions[index].id.clone();
        let Some((_, meaning)) = RESERVED_PLACEHOLDERS.iter().find(|(name, _)| *name == old) else {
            continue;
        };
        // A project has at most a few conditions, so a free number comes soon.
        let new = (2..)
            .map(|n| format!("{old}-{n}"))
            .find(|candidate| project.condition(candidate).is_none())
            .unwrap_or_default();
        rename_condition(project, &old, &new);
        notes.push(format!(
            "renamed condition {old:?} to {new:?}, since {{{{{old}}}}} now shows {meaning}"
        ));
    }
    notes
}

/// Renames condition `old` to `new` in the conditions, the rules that watch it, and every text
/// that shows it as a placeholder.
fn rename_condition(project: &mut Project, old: &str, new: &str) {
    for condition in project.conditions.iter_mut().filter(|condition| condition.id == old) {
        condition.id = new.to_owned();
    }
    for rule in &mut project.rules {
        if let Trigger::Condition { condition, .. } = &mut rule.when
            && condition == old
        {
            *condition = new.to_owned();
        }
        for action in &mut rule.actions {
            if let Action::SetProps { props, .. } = action
                && let Some(text) = &mut props.text
            {
                *text = rename_placeholder(text, old, new);
            }
        }
    }
    for element in &mut project.elements {
        element.props.text = rename_placeholder(&element.props.text, old, new);
    }
    let card_texts = project.decks.iter_mut().flat_map(|deck| &mut deck.cards);
    for text in card_texts.filter_map(|card| card.props.text.as_mut()) {
        *text = rename_placeholder(text, old, new);
    }
}

/// `text` with every `{{old}}`, in any spacing inside the braces, written as `{{new}}`; other
/// placeholders and unclosed braces stay as they are, as the template reads them.
fn rename_placeholder(text: &str, old: &str, new: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            break;
        };
        out.push_str(&rest[..start]);
        if after[..end].trim() == old {
            out.push_str(&format!("{{{{{new}}}}}"));
        } else {
            out.push_str(&rest[start..start + 2 + end + 2]);
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}
