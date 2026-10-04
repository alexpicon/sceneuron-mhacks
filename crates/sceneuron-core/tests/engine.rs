// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the overlay engine: rules on presence changes, manual overrides
//! that automation cannot replace, pause and resume, effects, and snapshots.

use sceneuron_core::{Effect, Engine, EngineError, Presence, PropsPatch, VisionStatus, default_project};

fn engine() -> Engine {
    Engine::new(default_project(), 0).unwrap()
}

fn visible(engine: &Engine, id: &str) -> bool {
    engine.effective_props(id).unwrap().visible
}

/// Drives presence with real observations: faces until 1 s, then nobody.
fn leave_camera(engine: &mut Engine) {
    for t in (0..=1_000).step_by(200) {
        engine.observe(0.95, t);
    }
    for t in (1_200..=4_400).step_by(200) {
        engine.observe(0.05, t);
    }
}

#[test]
fn leaving_the_camera_shows_brb_and_returning_hides_it() {
    let mut engine = engine();
    assert!(!visible(&engine, "brb"));
    leave_camera(&mut engine);
    assert_eq!(engine.presence(), Presence::Away);
    assert!(visible(&engine, "brb"));
    for t in (4_600..=5_200).step_by(200) {
        engine.observe(0.95, t);
    }
    assert_eq!(engine.presence(), Presence::Present);
    assert!(!visible(&engine, "brb"));
    let flash = engine
        .snapshot(5_200)
        .elements
        .into_iter()
        .find(|e| e.id == "cam-frame")
        .unwrap();
    assert_eq!(flash.effect.unwrap().effect, Effect::Flash);
}

#[test]
fn manual_override_holds_against_rules_until_released() {
    let mut engine = engine();
    let hide = PropsPatch {
        visible: Some(false),
        ..Default::default()
    };
    engine.set_override("brb", &hide, 0).unwrap();
    leave_camera(&mut engine);
    assert_eq!(engine.presence(), Presence::Away);
    assert!(!visible(&engine, "brb"), "automation replaced a held override");
    let view = engine
        .snapshot(5_000)
        .elements
        .into_iter()
        .find(|e| e.id == "brb")
        .unwrap();
    assert_eq!(view.overridden, vec!["visible"]);
    assert_eq!(view.automated, vec!["visible"]);
    engine.release_overrides(Some("brb"), 5_000).unwrap();
    assert!(visible(&engine, "brb"), "release should reveal the automatic value");
}

#[test]
fn paused_automation_ignores_presence_and_resume_applies_current_state() {
    let mut engine = engine();
    let outcome = engine.set_automation(false, 0);
    assert!(outcome.changed && outcome.project_dirty);
    leave_camera(&mut engine);
    assert_eq!(engine.presence(), Presence::Away);
    assert!(!visible(&engine, "brb"));
    engine.set_automation(true, 5_000);
    assert!(visible(&engine, "brb"));
    assert_eq!(engine.set_automation(true, 5_100), Default::default());
}

#[test]
fn manual_changes_stay_in_effect_while_paused() {
    let mut engine = engine();
    engine.set_automation(false, 0);
    let text = PropsPatch {
        text: Some("Live from the lab".into()),
        ..Default::default()
    };
    engine.set_override("name-label", &text, 10).unwrap();
    leave_camera(&mut engine);
    assert_eq!(engine.effective_props("name-label").unwrap().text, "Live from the lab");
}

#[test]
fn rejects_invalid_manual_requests() {
    let mut engine = engine();
    let patch = PropsPatch {
        opacity: Some(2.0),
        ..Default::default()
    };
    assert!(matches!(
        engine.set_override("brb", &patch, 0),
        Err(EngineError::Invalid(_))
    ));
    assert_eq!(
        engine.set_override("brb", &PropsPatch::default(), 0),
        Err(EngineError::EmptyPatch)
    );
    assert!(matches!(
        engine.trigger_effect("nope", Effect::Pop, 0),
        Err(EngineError::UnknownElement(_))
    ));
    assert!(engine.release_overrides(Some("nope"), 0).is_err());
}

#[test]
fn release_without_overrides_changes_nothing() {
    let mut engine = engine();
    let revision = engine.revision();
    assert!(!engine.release_overrides(None, 0).unwrap().changed);
    assert_eq!(engine.revision(), revision);
}

#[test]
fn effects_get_increasing_nonces() {
    let mut engine = engine();
    engine.trigger_effect("name-label", Effect::Shake, 0).unwrap();
    engine.trigger_effect("name-label", Effect::Shake, 1).unwrap();
    let view = engine
        .snapshot(2)
        .elements
        .into_iter()
        .find(|e| e.id == "name-label")
        .unwrap();
    assert_eq!(view.effect.unwrap().nonce, 2);
}

#[test]
fn rule_effects_skip_elements_under_manual_control() {
    let mut engine = engine();
    let held = PropsPatch {
        visible: Some(true),
        ..Default::default()
    };
    engine.set_override("cam-frame", &held, 0).unwrap();
    engine.simulate_presence(Presence::Present, 10);
    let view = engine
        .snapshot(20)
        .elements
        .into_iter()
        .find(|e| e.id == "cam-frame")
        .unwrap();
    assert!(view.effect.is_none());
}

#[test]
fn simulated_presence_runs_rules_and_is_logged() {
    let mut engine = engine();
    assert!(engine.simulate_presence(Presence::Away, 50).changed);
    assert!(visible(&engine, "brb"));
    let snapshot = engine.snapshot(60);
    assert_eq!(snapshot.events[0].source, "rule");
    assert!(snapshot.events.iter().any(|e| e.source == "test"));
}

#[test]
fn stale_camera_does_not_trigger_away_rules() {
    let mut engine = engine();
    for t in (0..=1_000).step_by(200) {
        engine.observe(0.95, t);
    }
    assert!(engine.tick(10_000).changed);
    assert_eq!(engine.presence(), Presence::Unknown);
    assert!(!visible(&engine, "brb"));
}

#[test]
fn replacing_the_project_drops_layers_of_removed_elements() {
    let mut engine = engine();
    let held = PropsPatch {
        text: Some("held".into()),
        ..Default::default()
    };
    engine.set_override("name-label", &held, 0).unwrap();
    let mut project = default_project();
    project.elements.retain(|e| e.id != "name-label");
    let outcome = engine.replace_project(project, 1).unwrap();
    assert!(outcome.project_dirty);
    assert!(engine.effective_props("name-label").is_none());
    let mut bad = default_project();
    bad.canvas.height = 1;
    assert!(engine.replace_project(bad, 2).is_err());
    assert!(engine.project().element("brb").is_some());
}

#[test]
fn vision_status_changes_publish_but_latency_is_telemetry() {
    let mut engine = engine();
    let failed = VisionStatus::Failed {
        message: "model".into(),
    };
    assert!(engine.set_vision_status(failed, 0).changed);
    let running = |latency_ms| VisionStatus::Running {
        latency_ms,
        mouths: true,
        hands: true,
    };
    assert!(engine.set_vision_status(running(30.0), 1).changed);
    let outcome = engine.set_vision_status(running(31.0), 2);
    assert!(!outcome.changed && outcome.telemetry);
    let vision = &serde_json::to_value(engine.snapshot(3)).unwrap()["vision"];
    let expected = serde_json::json!({"state": "running", "latency_ms": 31.0, "mouths": true, "hands": true});
    assert_eq!(
        *vision, expected,
        "the editor reads whether analysis measures mouths and sees hands"
    );
}

#[test]
fn snapshot_serializes_to_the_documented_shape() {
    let engine = engine();
    let json = serde_json::to_value(engine.snapshot(0)).unwrap();
    for key in [
        "revision",
        "project_revision",
        "now_ms",
        "canvas",
        "theme",
        "automation_enabled",
        "presence",
        "vision",
        "ai",
        "scene",
        "conditions",
        "elements",
        "decks",
        "events",
    ] {
        assert!(json.get(key).is_some(), "missing {key}");
    }
    assert_eq!(json["presence"]["state"], "unknown");
    assert_eq!(json["vision"]["state"], "idle");
    assert_eq!(json["elements"][0]["props"]["enter"], "pop");
    assert_eq!(json["ai"]["state"], "off");
    assert_eq!(json["conditions"][0]["on"], false);
}

#[test]
fn manual_controls_keep_working_after_camera_analysis_fails() {
    let mut engine = engine();
    let failed = VisionStatus::Failed {
        message: "model crashed".into(),
    };
    engine.set_vision_status(failed, 0);
    let show = PropsPatch {
        visible: Some(true),
        ..Default::default()
    };
    assert!(engine.set_override("brb", &show, 1).unwrap().changed);
    assert!(visible(&engine, "brb"));
    assert!(engine.trigger_effect("cam-frame", Effect::Pop, 2).is_ok());
}
