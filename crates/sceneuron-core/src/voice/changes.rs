// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What voice changes besides cards and reactions: the theme and edits to the saved project, which
//! the editor then catches up with, and visibility and text held on an element until it is
//! released, as the control API holds them.

use super::acting::Done;
use super::commands::theme_name;
use crate::engine::Engine;
use crate::plan::{EditOp, EditPlan};
use crate::props::PropsPatch;
use crate::theme::Theme;

impl Engine {
    pub(super) fn theme_by_voice(&mut self, theme: Theme, now_ms: u64) -> Done {
        let name = theme_name(theme);
        if self.project.theme == theme {
            let result = match theme {
                Theme::None => "no theme was on".to_owned(),
                _ => format!("the {name} theme was already on"),
            };
            return Done::said(result, false);
        }
        self.project.theme = theme;
        self.project_revision += 1;
        let result = match theme {
            Theme::None => "turned the theme off".to_owned(),
            _ => format!("switched the overlay to the {name} theme"),
        };
        self.record(now_ms, "voice", &result);
        Done {
            saved: true,
            ..Done::said(result, true)
        }
    }

    pub(super) fn edit_by_voice(&mut self, summary: &str, operations: &[EditOp], now_ms: u64) -> Done {
        let plan = EditPlan {
            summary: summary.to_owned(),
            operations: operations.to_vec(),
        };
        let applied = plan
            .apply(&self.project)
            .and_then(|next| self.adopt_project(next, now_ms));
        if let Err(error) = applied {
            return Done::said(format!("the project change was refused: {}", error.0), false);
        }
        let result = format!("changed the project: {}", summary.trim());
        self.record(now_ms, "voice", &result);
        Done {
            saved: true,
            ..Done::said(result, true)
        }
    }

    pub(super) fn show_by_voice(&mut self, element: &str, visible: bool, now_ms: u64) -> Done {
        let patch = PropsPatch {
            visible: Some(visible),
            ..PropsPatch::default()
        };
        let verb = if visible { "showed" } else { "hid" };
        self.hold_by_voice(element, &patch, |name| format!("{verb} {name}"), now_ms)
    }

    pub(super) fn text_by_voice(&mut self, element: &str, text: &str, now_ms: u64) -> Done {
        let patch = PropsPatch {
            text: Some(text.to_owned()),
            ..PropsPatch::default()
        };
        self.hold_by_voice(element, &patch, |name| format!("{name} says {text:?}"), now_ms)
    }

    pub(super) fn release_by_voice(&mut self, element: &str, now_ms: u64) -> Done {
        let Some(name) = self.project.element(element).map(|found| found.name.clone()) else {
            return Done::said(format!("there is no element {element:?}"), false);
        };
        if self.manual.remove(element).is_none() {
            return Done::said(format!("nothing was held on {name}"), false);
        }
        let result = format!("released {name}, so rules and cards decide again");
        self.record(now_ms, "voice", &result);
        Done::said(result, true)
    }

    /// Holds `patch` on an element until it is released; `said` words what it did by the
    /// element's name.
    fn hold_by_voice(&mut self, element: &str, patch: &PropsPatch, said: impl Fn(&str) -> String, now_ms: u64) -> Done {
        let Some(name) = self.project.element(element).map(|found| found.name.clone()) else {
            return Done::said(format!("there is no element {element:?}"), false);
        };
        if let Err(error) = patch.validate() {
            return Done::said(format!("{name} cannot take that: {}", error.0), false);
        }
        self.manual.entry(element.to_owned()).or_default().merge(patch);
        let result = said(&name);
        self.record(now_ms, "voice", &result);
        Done::said(result, true)
    }
}
