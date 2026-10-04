// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises OBS's program as the engine keeps it: the scene and whether OBS streams or records,
//! since when it is on air, `{{on-air}}` and `{{on-air-time}}` in overlay text, the elements that
//! show them, the snapshot's on-air view, and the placeholder names that conditions may not take.

mod common;

use common::{deck_project, engine, text};
use sceneuron_core::{Condition, Engine, OnAirView, Project, default_project};

fn on_air_project() -> Project {
    let mut project = deck_project();
    for (id, words) in [("pill", "{{ on-air }}"), ("timer", "On air {{on-air-time}}")] {
        project.elements.push(common::text_element(id, id));
        project.elements.last_mut().unwrap().props.text = words.to_owned();
    }
    project
}

fn view(engine: &Engine) -> OnAirView {
    engine.snapshot(0).on_air
}

#[test]
fn before_any_report_nothing_is_on_air() {
    let engine = engine(on_air_project());
    assert_eq!(view(&engine), OnAirView::default());
    assert_eq!(text(&engine, "pill"), "");
    let json = serde_json::to_value(engine.snapshot(0)).unwrap();
    assert_eq!(
        json["on_air"],
        serde_json::json!({"scene": "", "streaming": false, "recording": false, "since_ms": null})
    );
}

#[test]
fn the_scene_shows_alone_off_air_and_with_live_on_air() {
    let mut engine = engine(on_air_project());
    assert!(engine.set_on_air("Scene 01", false, false, 100).changed);
    assert_eq!(text(&engine, "pill"), "Scene 01");
    assert!(engine.set_on_air("Scene 01", true, false, 200).changed);
    assert_eq!(text(&engine, "pill"), "Scene 01 \u{b7} LIVE");
    assert!(engine.set_on_air("", false, true, 300).changed);
    assert_eq!(
        text(&engine, "pill"),
        "LIVE",
        "recording counts as live, with no scene to name"
    );
}

#[test]
fn on_air_counts_from_whichever_output_started_first_while_either_runs() {
    let mut engine = engine(on_air_project());
    engine.set_on_air("Talk", false, true, 1_000);
    assert_eq!(view(&engine).since_ms, Some(1_000));
    engine.set_on_air("Talk", true, true, 5_000);
    assert_eq!(
        view(&engine),
        OnAirView {
            scene: "Talk".to_owned(),
            streaming: true,
            recording: true,
            since_ms: Some(1_000),
        }
    );
    engine.set_on_air("Talk", true, false, 9_000);
    assert_eq!(
        view(&engine).since_ms,
        Some(5_000),
        "the stream has run since it started"
    );
    engine.set_on_air("Talk", false, false, 12_000);
    assert_eq!(view(&engine).since_ms, None);
    engine.set_on_air("Talk", true, false, 20_000);
    assert_eq!(
        view(&engine).since_ms,
        Some(20_000),
        "a new stream counts from its own start"
    );
}

#[test]
fn the_same_report_changes_nothing_and_going_live_or_off_is_logged() {
    let mut engine = engine(on_air_project());
    engine.set_on_air("Scene", true, false, 10);
    let revision = engine.revision();
    assert!(!engine.set_on_air("Scene", true, false, 20).changed);
    assert_eq!(engine.revision(), revision);
    assert_eq!(view(&engine).since_ms, Some(10));
    assert_eq!(common::last_event(&engine, 20), ("obs".to_owned(), "on air".to_owned()));
    assert!(
        engine.set_on_air("Interview", true, false, 30).changed,
        "a scene change is a change"
    );
    assert_eq!(
        common::last_event(&engine, 30).1,
        "on air",
        "a scene change is not logged"
    );
    engine.set_on_air("Interview", false, false, 40);
    assert_eq!(
        common::last_event(&engine, 40),
        ("obs".to_owned(), "off air".to_owned())
    );
}

#[test]
fn scene_names_are_kept_on_one_short_line() {
    let mut engine = engine(on_air_project());
    engine.set_on_air("  Main\n\tcamera\u{7}  ", false, false, 0);
    assert_eq!(view(&engine).scene, "Main camera");
    engine.set_on_air(&"x".repeat(200), false, false, 0);
    assert_eq!(view(&engine).scene.chars().count(), 80);
}

#[test]
fn the_time_on_air_is_left_for_the_overlay_to_count() {
    let mut engine = engine(on_air_project());
    engine.set_on_air("Scene", true, false, 0);
    assert_eq!(text(&engine, "timer"), "On air {{on-air-time}}");
    let snapshot = engine.snapshot(5_000);
    let flags = |id: &str| {
        let element = snapshot.elements.iter().find(|element| element.id == id).unwrap();
        (element.on_air, element.clock, element.props.text.clone())
    };
    assert_eq!(flags("pill"), (true, false, "Scene \u{b7} LIVE".to_owned()));
    assert_eq!(flags("timer"), (false, true, "On air {{on-air-time}}".to_owned()));
    assert_eq!(flags("slot"), (false, false, "base".to_owned()));
}

#[test]
fn conditions_may_not_take_the_on_air_placeholder_names() {
    for id in ["on-air", "on-air-time"] {
        let mut project = default_project();
        project.conditions.push(Condition {
            id: id.to_owned(),
            ..project.conditions[0].clone()
        });
        let error = project.validate().unwrap_err().to_string();
        assert!(error.contains(&format!("condition id {id:?} is reserved")), "{error}");
    }
}
