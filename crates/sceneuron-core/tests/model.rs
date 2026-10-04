// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises project validation, property patches, and the JSON shape of the model.

use sceneuron_core::{
    Action, Effect, ElementKind, ElementProps, Project, PropsPatch, SCHEMA_VERSION, Trigger, default_project,
};

#[test]
fn default_project_is_valid_and_has_presence_rules() {
    let project = default_project();
    project.validate().unwrap();
    assert_eq!(project.version, SCHEMA_VERSION);
    let triggers: Vec<_> = project.rules.iter().map(|r| r.when.clone()).collect();
    assert!(triggers.contains(&Trigger::away()) && triggers.contains(&Trigger::present()));
    assert!(!project.element("brb").unwrap().props.visible);
}

#[test]
fn rejects_duplicate_element_ids() {
    let mut project = default_project();
    project.elements.push(project.elements[0].clone());
    let error = project.validate().unwrap_err();
    assert!(error.0.contains("duplicate element id"), "{error}");
}

#[test]
fn rejects_rules_that_reference_missing_elements() {
    let mut project = default_project();
    project.rules[0].actions.push(Action::Effect {
        element: "ghost".into(),
        effect: Effect::Pop,
    });
    assert!(project.validate().unwrap_err().0.contains("unknown element \"ghost\""));
}

#[test]
fn rejects_bad_ids_names_and_canvas() {
    for id in ["", "Upper", "has space", &"x".repeat(41)] {
        let mut project = default_project();
        project.elements[0].id = id.to_owned();
        project.rules.clear();
        assert!(project.validate().is_err(), "accepted id {id:?}");
    }
    let mut project = default_project();
    project.elements[0].name = String::new();
    assert!(project.validate().is_err());
    let mut project = default_project();
    project.canvas.width = 8;
    assert!(project.validate().is_err());
}

#[test]
fn rejects_out_of_range_properties() {
    let cases = [
        PropsPatch {
            opacity: Some(1.5),
            ..Default::default()
        },
        PropsPatch {
            width: Some(0.0),
            ..Default::default()
        },
        PropsPatch {
            x: Some(f64::NAN),
            ..Default::default()
        },
        PropsPatch {
            color: Some("red".into()),
            ..Default::default()
        },
        PropsPatch {
            color: Some("#12345".into()),
            ..Default::default()
        },
        PropsPatch {
            text: Some("bell\u{7}".into()),
            ..Default::default()
        },
        PropsPatch {
            text: Some("x".repeat(1001)),
            ..Default::default()
        },
    ];
    for patch in cases {
        assert!(patch.validate().is_err(), "accepted {patch:?}");
    }
    let ok = PropsPatch {
        color: Some("#AbC".into()),
        text: Some("two\nlines".into()),
        ..Default::default()
    };
    ok.validate().unwrap();
}

#[test]
fn rejects_unsupported_versions_and_detection_settings() {
    let mut project = default_project();
    project.version = SCHEMA_VERSION + 1;
    assert!(project.validate().is_err());
    let mut project = default_project();
    project.detection.min_confidence = 1.0;
    assert!(project.validate().is_err());
    let mut project = default_project();
    project.detection.sample_fps = 0.0;
    assert!(project.validate().is_err());
}

#[test]
fn patch_merge_apply_and_field_names() {
    let mut held = PropsPatch {
        text: Some("A".into()),
        ..Default::default()
    };
    held.merge(&PropsPatch {
        visible: Some(false),
        text: Some("B".into()),
        ..Default::default()
    });
    assert_eq!(held.field_names(), vec!["visible", "text"]);
    let mut props = ElementProps::default();
    props.apply(&held);
    assert_eq!((props.visible, props.text.as_str()), (false, "B"));
    assert!(PropsPatch::default().is_empty());
}

#[test]
fn json_uses_documented_names() {
    let action = Action::SetProps {
        element: "brb".into(),
        props: PropsPatch {
            visible: Some(true),
            ..Default::default()
        },
    };
    let json = serde_json::to_value(&action).unwrap();
    assert_eq!(
        json,
        serde_json::json!({"type": "set_props", "element": "brb", "props": {"visible": true}})
    );
    let kind = serde_json::to_value(ElementKind::Frame).unwrap();
    assert_eq!(kind, "frame");
}

#[test]
fn patch_rejects_unknown_fields() {
    let parsed: Result<PropsPatch, _> = serde_json::from_str(r#"{"visble": true}"#);
    assert!(parsed.is_err());
}

#[test]
fn missing_fields_take_defaults() {
    let project: Project = serde_json::from_str(r#"{"version": 4}"#).unwrap();
    project.validate().unwrap();
    assert!(project.automation_enabled);
    assert!(project.elements.is_empty() && project.decks.is_empty());
}

#[test]
fn triggers_use_plain_words_for_presence_and_objects_for_conditions() {
    let away = serde_json::to_value(Trigger::away()).unwrap();
    assert_eq!(away, serde_json::json!("away"));
    let on = Trigger::condition("pet-in-view", sceneuron_core::ConditionEdge::On);
    let json = serde_json::to_value(&on).unwrap();
    assert_eq!(json, serde_json::json!({"condition": "pet-in-view", "state": "on"}));
    assert_eq!(serde_json::from_value::<Trigger>(json).unwrap(), on);
    assert!(serde_json::from_str::<Trigger>(r#""sideways""#).is_err());
}

#[test]
fn rejects_invalid_conditions() {
    let check = |edit: fn(&mut sceneuron_core::Condition)| {
        let mut project = default_project();
        edit(&mut project.conditions[0]);
        project.validate().unwrap_err().0
    };
    assert!(check(|c| c.id = "scene".into()).contains("reserved"));
    assert!(check(|c| c.id = "room".into()).contains("reserved for the room summary"));
    assert!(check(|c| c.prompt = String::new()).contains("prompt"));
    assert!(check(|c| c.prompt = "x".repeat(301)).contains("prompt"));
    assert!(check(|c| c.detail = "a\nb".into()).contains("detail"));
    assert!(check(|c| c.min_confidence = f64::NAN).contains("min_confidence"));
    assert!(check(|c| c.confirm = 0).contains("confirm"));
    let mut project = default_project();
    project.conditions.push(project.conditions[0].clone());
    assert!(project.validate().unwrap_err().0.contains("duplicate condition"));
}

#[test]
fn rejects_rules_for_unknown_conditions_and_invalid_ai_policy() {
    let mut project = default_project();
    project.conditions.retain(|c| c.id != "pet-in-view");
    assert!(project.validate().unwrap_err().0.contains("unknown condition"));
    let check = |edit: fn(&mut sceneuron_core::AiPolicy)| {
        let mut project = default_project();
        edit(&mut project.ai);
        project.validate().unwrap_err().0
    };
    assert!(check(|ai| ai.min_interval_ms = 10).contains("min_interval_ms"));
    assert!(check(|ai| ai.refresh_ms = 1_000).contains("refresh_ms"));
    assert!(check(|ai| ai.change_threshold = 2.0).contains("change_threshold"));
    assert!(check(|ai| ai.max_calls_per_minute = 0).contains("max_calls_per_minute"));
    assert!(check(|ai| ai.max_cost_per_hour = -1.0).contains("max_cost_per_hour"));
}
