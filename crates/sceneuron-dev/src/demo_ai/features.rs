// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Building blocks of simulated plans: a feature pairs a condition with an overlay element and the
//! rules that show and hide it. The planner reuses what the project already has for the same topic,
//! so asking twice never duplicates a lower third.

use std::collections::BTreeSet;

use sceneuron_core::{
    Action, Condition, ConditionEdge, EditOp, EditPlan, Effect, Element, ElementKind, Project, PropsPatch, Rule,
    Trigger,
};

use super::recipes::{Color, ID_MARK};
use super::scenes::{Topic, topic_of};

/// A condition, the element that reacts to it, and the names of the rules between them.
pub(crate) struct Feature {
    pub(crate) topic: Topic,
    pub(crate) condition: Condition,
    /// Its text may name the condition id as `@id`.
    pub(crate) element: Element,
    pub(crate) effect: Option<Effect>,
    pub(crate) show_rule: &'static str,
    pub(crate) hide_rule: &'static str,
    /// What the feature does, for the plan summary.
    pub(crate) summary: String,
}

/// Collects operations for one plan and the ids they take.
pub(crate) struct Planner<'a> {
    project: &'a Project,
    ops: Vec<EditOp>,
    conditions: BTreeSet<String>,
    elements: BTreeSet<String>,
    rules: BTreeSet<String>,
}

impl<'a> Planner<'a> {
    pub(crate) fn new(project: &'a Project) -> Self {
        let ids = |ids: Vec<&String>| ids.into_iter().cloned().collect();
        Self {
            project,
            ops: Vec::new(),
            conditions: ids(project.conditions.iter().map(|c| &c.id).collect()),
            elements: ids(project.elements.iter().map(|e| &e.id).collect()),
            rules: ids(project.rules.iter().map(|r| &r.id).collect()),
        }
    }

    /// Adds the feature, or adapts the condition and element the project already has for its topic.
    /// `tailor` rewrites an existing condition's words to the feature's.
    pub(crate) fn add(&mut self, mut feature: Feature, color: Option<Color>, tailor: bool) -> String {
        let existing = self
            .project
            .conditions
            .iter()
            .find(|condition| topic_of(condition) == feature.topic && feature.topic != Topic::Other);
        let condition = match existing {
            Some(existing) => {
                let wanted = Condition {
                    prompt: feature.condition.prompt.clone(),
                    detail: feature.condition.detail.clone(),
                    ..existing.clone()
                };
                if tailor && wanted != *existing {
                    self.ops.push(EditOp::UpdateCondition {
                        condition: wanted.clone(),
                    });
                }
                if tailor { wanted } else { existing.clone() }
            }
            None => {
                feature.condition.id = unique(&feature.condition.id, &mut self.conditions);
                self.ops.push(EditOp::AddCondition {
                    condition: feature.condition.clone(),
                });
                feature.condition
            }
        };
        if let Some(color) = color {
            feature.element.props.background = color.background.to_owned();
            feature.element.props.color = color.text.to_owned();
        }
        feature.element.props.text = feature.element.props.text.replace(ID_MARK, &condition.id);
        let element = match self.element_for(&condition.id) {
            Some(existing) => {
                self.update_element(existing, &feature.element, color.is_some(), tailor);
                existing.clone()
            }
            None => {
                feature.element.id = unique(&feature.element.id, &mut self.elements);
                self.ops.push(EditOp::AddElement {
                    element: feature.element.clone(),
                });
                feature.element
            }
        };
        let names = [feature.show_rule, feature.hide_rule];
        self.ensure_rules(&condition, &element, feature.effect, names);
        feature.summary
    }

    /// An element whose text shows the condition's detail, or that a rule shows when it turns on.
    fn element_for(&self, condition: &str) -> Option<&'a Element> {
        let placeholder = format!("{{{{{condition}}}}}");
        let on = Trigger::condition(condition, ConditionEdge::On);
        let shown = |id: &str| {
            self.project.rules.iter().any(|rule| {
                rule.when == on
                    && rule.actions.iter().any(|action| {
                        matches!(action, Action::SetProps { element, props } if element == id && props.visible == Some(true))
                    })
            })
        };
        let project = self.project;
        project
            .elements
            .iter()
            .find(|element| element.props.text.contains(&placeholder))
            .or_else(|| project.elements.iter().find(|element| shown(&element.id)))
    }

    fn update_element(&mut self, existing: &Element, wanted: &Element, recolor: bool, tailor: bool) {
        let mut props = PropsPatch::default();
        if recolor && existing.props.background != wanted.props.background {
            props.background = Some(wanted.props.background.clone());
            props.color = Some(wanted.props.color.clone());
        }
        if tailor && existing.kind == ElementKind::Text && existing.props.text != wanted.props.text {
            props.text = Some(wanted.props.text.clone());
        }
        if !props.is_empty() {
            self.ops.push(EditOp::UpdateElement {
                id: existing.id.clone(),
                name: None,
                props,
            });
        }
    }

    /// Adds a rule that shows the element when the condition turns on, playing `effect`, and one
    /// that hides it when the condition turns off, unless the project has them; `names` names both.
    fn ensure_rules(&mut self, condition: &Condition, element: &Element, effect: Option<Effect>, names: [&str; 2]) {
        for (edge, visible, name) in [
            (ConditionEdge::On, true, names[0]),
            (ConditionEdge::Off, false, names[1]),
        ] {
            let when = Trigger::condition(&condition.id, edge);
            let exists = self.project.rules.iter().any(|rule| {
                rule.when == when
                    && rule.actions.iter().any(|action| {
                        matches!(action, Action::SetProps { element: id, props } if *id == element.id && props.visible == Some(visible))
                    })
            });
            if exists {
                continue;
            }
            let mut actions = vec![show(&element.id, visible)];
            if let (true, Some(effect)) = (visible, effect) {
                actions.push(Action::Effect {
                    element: element.id.clone(),
                    effect,
                });
            }
            let base = format!("{}-{}", condition.id, if visible { "show" } else { "hide" });
            let rule = Rule {
                id: unique(&base, &mut self.rules),
                name: name.to_owned(),
                enabled: true,
                when,
                actions,
            };
            self.ops.push(EditOp::AddRule { rule });
        }
    }

    /// Puts `title` on the stream's name label: the element with id "name-label", or else the first
    /// visible text element that shows no placeholder. Returns whether it changes anything.
    pub(crate) fn retitle(&mut self, title: &str) -> bool {
        let plain = |element: &&Element| {
            element.kind == ElementKind::Text && element.props.visible && !element.props.text.contains("{{")
        };
        let label = self
            .project
            .element("name-label")
            .or_else(|| self.project.elements.iter().find(plain))
            .filter(|label| label.props.text != title);
        let Some(label) = label else {
            return false;
        };
        let props = PropsPatch {
            text: Some(title.to_owned()),
            ..PropsPatch::default()
        };
        self.ops.push(EditOp::UpdateElement {
            id: label.id.clone(),
            name: None,
            props,
        });
        true
    }

    /// The project the plan changes.
    pub(crate) fn project(&self) -> &'a Project {
        self.project
    }

    /// Adds an operation that needs nothing reused or renamed.
    pub(crate) fn push(&mut self, op: EditOp) {
        self.ops.push(op);
    }

    /// `base`, or `base-2`, `base-3`, and so on: the first id no element of the project or this plan
    /// has. The plan takes it.
    pub(crate) fn element_id(&mut self, base: &str) -> String {
        unique(base, &mut self.elements)
    }

    pub(crate) fn finish(self, summary: String) -> EditPlan {
        EditPlan {
            summary,
            operations: self.ops,
        }
    }
}

/// `base`, or `base-2`, `base-3`, and so on: the first id not taken, never `scene`.
pub(crate) fn unique(base: &str, taken: &mut BTreeSet<String>) -> String {
    let id = std::iter::once(base.to_owned())
        .chain((2..).map(|n| format!("{base}-{n}")))
        .find(|id| id != "scene" && !taken.contains(id))
        .unwrap_or_default();
    taken.insert(id.clone());
    id
}

pub(crate) fn show(element: &str, visible: bool) -> Action {
    Action::SetProps {
        element: element.to_owned(),
        props: PropsPatch {
            visible: Some(visible),
            ..PropsPatch::default()
        },
    }
}
