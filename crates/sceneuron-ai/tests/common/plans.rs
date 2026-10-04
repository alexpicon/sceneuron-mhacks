// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Sample edit plans that use every operation, every optional field, and every enum value,
//! with variant lists that stop compiling when the core gains a variant.

use sceneuron_core::{
    Action, ConditionEdge, Deck, EditOp, EditPlan, Effect, Element, ElementKind, ElementProps, EnterAnimation,
    IdleAnimation, PresenceTrigger, PropsPatch, Rule, TextAlign, Trigger,
};
use serde_json::json;

use super::condition;

/// The serde names of every hand sign a card signal can name.
pub const HAND_SIGNS: [&str; 6] = ["thumbs_up", "thumbs_down", "open_palm", "peace", "pointing", "fist"];

// Each list is checked by an exhaustive match, so a new variant fails to compile until added.
pub fn kinds() -> Vec<ElementKind> {
    let all = vec![ElementKind::Text, ElementKind::Box, ElementKind::Frame];
    all.iter().for_each(|v| match v {
        ElementKind::Text | ElementKind::Box | ElementKind::Frame => {}
    });
    all
}

pub fn aligns() -> Vec<TextAlign> {
    let all = vec![TextAlign::Left, TextAlign::Center, TextAlign::Right];
    all.iter().for_each(|v| match v {
        TextAlign::Left | TextAlign::Center | TextAlign::Right => {}
    });
    all
}

pub fn idles() -> Vec<IdleAnimation> {
    use IdleAnimation::*;
    let all = vec![None, Pulse, Glow, Float];
    all.iter().for_each(|v| match v {
        None | Pulse | Glow | Float => {}
    });
    all
}

pub fn enters() -> Vec<EnterAnimation> {
    use EnterAnimation::*;
    let all = vec![None, Fade, SlideUp, Pop];
    all.iter().for_each(|v| match v {
        None | Fade | SlideUp | Pop => {}
    });
    all
}

pub fn effects() -> Vec<Effect> {
    let all = vec![Effect::Shake, Effect::Pop, Effect::Flash, Effect::Bounce];
    all.iter().for_each(|v| match v {
        Effect::Shake | Effect::Pop | Effect::Flash | Effect::Bounce => {}
    });
    all
}

pub fn triggers() -> Vec<Trigger> {
    let presence = [PresenceTrigger::Away, PresenceTrigger::Present];
    presence.iter().for_each(|v| match v {
        PresenceTrigger::Away | PresenceTrigger::Present => {}
    });
    let edges = [ConditionEdge::On, ConditionEdge::Off];
    edges.iter().for_each(|v| match v {
        ConditionEdge::On | ConditionEdge::Off => {}
    });
    let mut all: Vec<Trigger> = presence.into_iter().map(Trigger::Presence).collect();
    all.extend(edges.into_iter().map(|edge| Trigger::condition("waving", edge)));
    all
}

pub fn patch(n: usize) -> PropsPatch {
    PropsPatch {
        x: Some(10.5),
        y: Some(-4.0),
        width: Some(300.0),
        height: Some(90.0),
        visible: Some(n.is_multiple_of(2)),
        text: Some("Now showing: {{showing-item}}".to_owned()),
        font_size: Some(40.0),
        bold: Some(false),
        color: Some("#ffffff".to_owned()),
        background: Some("#141428cc".to_owned()),
        border_color: Some("#9b5cff".to_owned()),
        border_width: Some(4.0),
        corner_radius: Some(12.0),
        opacity: Some(0.9),
        align: Some(aligns()[n % 3]),
        idle: Some(idles()[n % 4]),
        enter: Some(enters()[n % 4]),
    }
}

pub fn props(n: usize) -> ElementProps {
    let mut props = ElementProps::default();
    props.apply(&patch(n));
    props
}

/// A deck shown by `el-0` with the camera on, a card that sets every prop it may, and a card for
/// every kind of signal and hand sign. It is built from JSON, so it carries every deck and card
/// field the core reads.
pub fn deck(name: &str) -> Deck {
    let mut props = patch(1);
    props.visible = None;
    let mut cards = vec![
        json!({"id": "raised", "name": "Hand raised", "props": props, "signal": "hand_raised"}),
        json!({"id": "three", "name": "Three", "props": {"text": "3"}, "signal": {"fingers": 3}}),
    ];
    cards.extend(HAND_SIGNS.iter().map(
        |sign| json!({"id": sign.replace('_', "-"), "name": sign, "props": {"text": sign}, "signal": {"sign": sign}}),
    ));
    let deck = json!({
        "id": "votes", "name": name, "element": "el-0", "cards": cards, "auto_hide_ms": 15_000, "camera": true,
    });
    serde_json::from_value(deck).unwrap()
}

/// A plan with every operation, every enum value, and every optional field set.
pub fn every_operation() -> EditPlan {
    let mut operations = Vec::new();
    for (n, kind) in kinds().into_iter().enumerate() {
        let id = format!("el-{n}");
        let element = Element {
            id,
            name: format!("Element {n}"),
            kind,
            props: props(n),
        };
        operations.push(EditOp::AddElement { element });
    }
    for n in 0..4 {
        operations.push(EditOp::UpdateElement {
            id: "el-0".to_owned(),
            name: Some(format!("Renamed {n}")),
            props: patch(n),
        });
    }
    operations.push(EditOp::RemoveElement { id: "el-2".to_owned() });
    operations.push(EditOp::AddCondition {
        condition: condition("waving", "The person waves at the camera.", "which hand"),
    });
    let mut changed = condition("waving", "The person waves a hand.", "");
    (
        changed.enabled,
        changed.min_confidence,
        changed.confirm,
        changed.release,
    ) = (false, 0.75, 2, 3);
    operations.push(EditOp::UpdateCondition { condition: changed });
    operations.push(EditOp::RemoveCondition { id: "old".to_owned() });
    for (n, when) in triggers().into_iter().enumerate() {
        let mut actions = vec![Action::SetProps {
            element: "el-0".to_owned(),
            props: patch(n),
        }];
        actions.extend(effects().into_iter().map(|effect| Action::Effect {
            element: "el-1".to_owned(),
            effect,
        }));
        let rule = Rule {
            id: format!("rule-{n}"),
            name: format!("Rule {n}"),
            enabled: n.is_multiple_of(2),
            when,
            actions,
        };
        operations.push(if n.is_multiple_of(2) {
            EditOp::AddRule { rule }
        } else {
            EditOp::UpdateRule { rule }
        });
    }
    operations.push(EditOp::RemoveRule {
        id: "rule-9".to_owned(),
    });
    operations.push(EditOp::AddDeck { deck: deck("Votes") });
    operations.push(EditOp::UpdateDeck {
        deck: deck("Quick votes"),
    });
    operations.push(EditOp::RemoveDeck { id: "votes".to_owned() });
    EditPlan {
        summary: "Adds a waving alert.".to_owned(),
        operations,
    }
}
