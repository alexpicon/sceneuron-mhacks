// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The simulated planner's answer to requests it has no feature for: it restyles the element the
//! request names, with the color or quoted text it mentions, and says plainly that it is simulated.

use sceneuron_core::{EditOp, EditPlan, Element, ElementKind, ElementProps, Project, PropsPatch};

use super::recipes::{find_color, next_color};

/// Changes the colors or text of the element the request names, or of the first text element;
/// `words` is the request in lowercase.
pub(crate) fn restyle(project: &Project, request: &str, words: &str) -> EditPlan {
    let named = project
        .elements
        .iter()
        .find(|element| words.contains(&element.name.to_lowercase()));
    let first_text = project
        .elements
        .iter()
        .find(|element| element.kind == ElementKind::Text);
    let Some(target) = named.or(first_text).or(project.elements.first()) else {
        return add_first_text(request);
    };
    let mut props = PropsPatch::default();
    let quoted = quoted(request);
    if let Some(text) = &quoted {
        props.text = Some(text.clone());
    }
    let color = find_color(words);
    let text_color = words.contains("text color") || words.contains("font color");
    match (color, text_color) {
        (Some(color), true) => props.color = Some(color.background.to_owned()),
        (Some(color), false) => {
            props.background = Some(color.background.to_owned());
            props.color = Some(color.text.to_owned());
        }
        (None, _) if quoted.is_none() => {
            let color = next_color(&target.props.background);
            props.background = Some(color.background.to_owned());
            props.color = Some(color.text.to_owned());
        }
        (None, _) => {}
    }
    let summary = if named.is_some() || quoted.is_some() || color.is_some() {
        format!("Simulated plan: restyle {} as you asked.", target.name)
    } else {
        format!(
            "Simulated plan: no known request matched, so this gives {} a new color. The real assistant \
             understands any request; the simulated one knows waving, holding things up, pets, arms up, \
             meetings, planning poker, classes, colors, and quoted text.",
            target.name
        )
    };
    EditPlan {
        summary,
        operations: vec![EditOp::UpdateElement {
            id: target.id.clone(),
            name: None,
            props,
        }],
    }
}

fn add_first_text(request: &str) -> EditPlan {
    let text = quoted(request).unwrap_or_else(|| "Hello stream".to_owned());
    let element = Element {
        id: "title".to_owned(),
        name: "Title".to_owned(),
        kind: ElementKind::Text,
        props: ElementProps {
            x: 60.0,
            y: 900.0,
            width: 900.0,
            height: 110.0,
            text,
            font_size: 56.0,
            background: "#9b5cffdd".to_owned(),
            corner_radius: 18.0,
            ..ElementProps::default()
        },
    };
    EditPlan {
        summary: "Simulated plan: add a title card to start your layout.".to_owned(),
        operations: vec![EditOp::AddElement { element }],
    }
}

/// Text between straight or curly double quotes, if any.
fn quoted(request: &str) -> Option<String> {
    let start = request.find(['"', '\u{201c}'])?;
    let rest = &request[start..];
    let open = rest.chars().next()?;
    let rest = &rest[open.len_utf8()..];
    let end = rest.find(['"', '\u{201d}'])?;
    let text = rest[..end].trim();
    (!text.is_empty()).then(|| text.chars().take(200).collect())
}

#[cfg(test)]
mod tests {
    use sceneuron_core::default_project;

    use super::*;

    fn apply(project: &Project, request: &str) -> (EditPlan, Project) {
        let plan = restyle(project, request, &request.to_lowercase());
        let applied = plan.apply(project).unwrap_or_else(|error| panic!("{request}: {error}"));
        (plan, applied)
    }

    #[test]
    fn named_elements_get_the_quoted_text_or_named_color() {
        let project = default_project();
        let (plan, applied) = apply(&project, "Make the name label say \"Live now\"");
        assert_eq!(plan.operations.len(), 1);
        assert_eq!(applied.element("name-label").unwrap().props.text, "Live now");
        let (_, applied) = apply(&project, "Camera frame text color yellow");
        assert_eq!(applied.element("cam-frame").unwrap().props.color, "#ffc233ee");
    }

    #[test]
    fn anything_else_recolors_the_first_text_element_and_says_why() {
        let project = default_project();
        let (plan, applied) = apply(&project, "make it fancier");
        assert!(plan.summary.contains("no known request matched"), "{}", plan.summary);
        assert_ne!(
            applied.elements[0].props.background,
            project.elements[0].props.background
        );
        let (plan, applied) = apply(&Project::default(), "anything");
        assert!(matches!(plan.operations[..], [EditOp::AddElement { .. }]));
        assert_eq!(applied.elements.len(), 1);
    }

    #[test]
    fn quotes_may_be_straight_or_curly() {
        assert_eq!(
            quoted("say \u{201c}Back soon\u{201d} please"),
            Some("Back soon".to_owned())
        );
        assert_eq!(quoted("say \"\" please"), None);
        assert_eq!(quoted("no quotes"), None);
    }
}
