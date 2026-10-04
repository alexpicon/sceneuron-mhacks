// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The default project: a "be right back" card driven by presence, a name label, a camera frame,
//! and two cards driven by what the vision model sees: an object held up to the camera and a pet.

use crate::model::{Action, Condition, ConditionEdge, Effect, Element, ElementKind, Project, Rule, Trigger};
use crate::props::{ElementProps, EnterAnimation, IdleAnimation, PropsPatch};

/// The project used when the saved project cannot be read or the starter project cannot be used,
/// and by `sceneuron-dev` without `--project`. A fresh install starts from the starter project
/// instead (see [`starter_project`](crate::starter_project)).
pub fn default_project() -> Project {
    let brb = Element {
        id: "brb".to_owned(),
        name: "Be right back".to_owned(),
        kind: ElementKind::Text,
        props: ElementProps {
            x: 560.0,
            y: 440.0,
            width: 800.0,
            height: 200.0,
            visible: false,
            text: "Be right back".to_owned(),
            font_size: 96.0,
            background: "#141428cc".to_owned(),
            border_color: "#9b5cffff".to_owned(),
            border_width: 6.0,
            corner_radius: 32.0,
            idle: IdleAnimation::Pulse,
            enter: EnterAnimation::Pop,
            ..ElementProps::default()
        },
    };
    let label = Element {
        id: "name-label".to_owned(),
        name: "Name label".to_owned(),
        kind: ElementKind::Text,
        props: ElementProps {
            x: 60.0,
            y: 900.0,
            width: 640.0,
            height: 110.0,
            text: "Sceneuron".to_owned(),
            font_size: 56.0,
            background: "#9b5cffdd".to_owned(),
            corner_radius: 18.0,
            enter: EnterAnimation::SlideUp,
            ..ElementProps::default()
        },
    };
    let frame = Element {
        id: "cam-frame".to_owned(),
        name: "Camera frame".to_owned(),
        kind: ElementKind::Frame,
        props: ElementProps {
            x: 1260.0,
            y: 660.0,
            width: 620.0,
            height: 360.0,
            border_color: "#9b5cffff".to_owned(),
            border_width: 8.0,
            corner_radius: 24.0,
            idle: IdleAnimation::Glow,
            ..ElementProps::default()
        },
    };
    let showing = Element {
        id: "now-showing".to_owned(),
        name: "Now showing".to_owned(),
        kind: ElementKind::Text,
        props: ElementProps {
            x: 60.0,
            y: 770.0,
            width: 900.0,
            height: 100.0,
            visible: false,
            text: "Now showing: {{showing-item}}".to_owned(),
            font_size: 52.0,
            background: "#ff5c8add".to_owned(),
            corner_radius: 18.0,
            enter: EnterAnimation::SlideUp,
            ..ElementProps::default()
        },
    };
    let pet = Element {
        id: "pet-alert".to_owned(),
        name: "Pet alert".to_owned(),
        kind: ElementKind::Text,
        props: ElementProps {
            x: 1380.0,
            y: 60.0,
            width: 480.0,
            height: 110.0,
            visible: false,
            text: "{{pet-in-view}} on stream!".to_owned(),
            font_size: 52.0,
            color: "#1a1a1aff".to_owned(),
            background: "#ffc233ee".to_owned(),
            corner_radius: 55.0,
            idle: IdleAnimation::Float,
            enter: EnterAnimation::Pop,
            ..ElementProps::default()
        },
    };
    let conditions = vec![
        condition(
            "showing-item",
            "Showing something",
            "The person is deliberately holding an object up toward the camera to show it to viewers.",
            "the object's name in one to four words, such as \"coffee mug\"",
        ),
        condition(
            "pet-in-view",
            "Pet on camera",
            "A cat, dog, or other pet animal is clearly visible in the image.",
            "the kind of animal as one capitalized word, such as \"Cat\"",
        ),
    ];
    let on = |id: &str| Trigger::condition(id, ConditionEdge::On);
    let off = |id: &str| Trigger::condition(id, ConditionEdge::Off);
    let flash = Action::Effect {
        element: "cam-frame".to_owned(),
        effect: Effect::Flash,
    };
    let bounce = Action::Effect {
        element: "cam-frame".to_owned(),
        effect: Effect::Bounce,
    };
    let rules = vec![
        rule(
            "away-show-brb",
            "Show BRB when I leave",
            Trigger::away(),
            vec![show("brb", true)],
        ),
        rule(
            "present-hide-brb",
            "Hide BRB when I return",
            Trigger::present(),
            vec![show("brb", false), flash],
        ),
        rule(
            "item-show",
            "Name what I hold up",
            on("showing-item"),
            vec![show("now-showing", true)],
        ),
        rule(
            "item-hide",
            "Hide it when I put it down",
            off("showing-item"),
            vec![show("now-showing", false)],
        ),
        rule(
            "pet-show",
            "Announce my pet",
            on("pet-in-view"),
            vec![show("pet-alert", true), bounce],
        ),
        rule(
            "pet-hide",
            "Hide the pet alert",
            off("pet-in-view"),
            vec![show("pet-alert", false)],
        ),
    ];
    Project {
        elements: vec![brb, label, frame, showing, pet],
        conditions,
        rules,
        ..Project::default()
    }
}

fn condition(id: &str, name: &str, prompt: &str, detail: &str) -> Condition {
    Condition {
        id: id.to_owned(),
        name: name.to_owned(),
        prompt: prompt.to_owned(),
        detail: detail.to_owned(),
        enabled: true,
        min_confidence: 0.6,
        confirm: 1,
        release: 2,
    }
}

fn rule(id: &str, name: &str, when: Trigger, actions: Vec<Action>) -> Rule {
    Rule {
        id: id.to_owned(),
        name: name.to_owned(),
        enabled: true,
        when,
        actions,
    }
}

fn show(element: &str, visible: bool) -> Action {
    Action::SetProps {
        element: element.to_owned(),
        props: PropsPatch {
            visible: Some(visible),
            ..PropsPatch::default()
        },
    }
}
