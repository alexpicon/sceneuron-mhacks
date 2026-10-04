// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises scene understanding in the engine: debounced conditions, condition rules,
//! placeholders that show what the model saw, explanations, resets, expiry, and AI status.

use sceneuron_core::{AiStatus, Engine, SceneObservation, Verdict, default_project};

fn engine() -> Engine {
    Engine::new(default_project(), 0).unwrap()
}

fn verdict(condition: &str, holds: bool, confidence: f64, detail: &str) -> Verdict {
    Verdict {
        condition: condition.to_owned(),
        holds,
        confidence,
        detail: detail.to_owned(),
        reason: format!("{condition} judged {holds}"),
    }
}

fn scene(caption: &str, verdicts: Vec<Verdict>) -> SceneObservation {
    SceneObservation {
        caption: caption.to_owned(),
        verdicts,
    }
}

fn holding(engine: &mut Engine, item: &str, at_ms: u64) {
    let seen = scene(
        "A person holds something up",
        vec![verdict("showing-item", true, 0.9, item)],
    );
    engine.observe_scene(seen, at_ms);
}

fn put_down(engine: &mut Engine, at_ms: u64) {
    let seen = scene("A person at a desk", vec![verdict("showing-item", false, 0.9, "")]);
    engine.observe_scene(seen, at_ms);
}

fn visible(engine: &Engine, id: &str) -> bool {
    engine.effective_props(id).unwrap().visible
}

#[test]
fn holding_an_item_shows_its_name_and_putting_it_down_hides_it() {
    let mut engine = engine();
    assert!(!visible(&engine, "now-showing"));
    holding(&mut engine, "coffee mug", 1_000);
    assert!(engine.condition_on("showing-item"));
    assert!(visible(&engine, "now-showing"));
    assert_eq!(
        engine.effective_props("now-showing").unwrap().text,
        "Now showing: coffee mug"
    );
    put_down(&mut engine, 2_000);
    assert!(engine.condition_on("showing-item"), "one negative must not release");
    put_down(&mut engine, 3_000);
    assert!(!engine.condition_on("showing-item"));
    assert!(!visible(&engine, "now-showing"));
}

#[test]
fn every_change_is_explained_in_the_event_log() {
    let mut engine = engine();
    holding(&mut engine, "guitar", 1_000);
    let events = engine.snapshot(1_000).events;
    assert!(
        events
            .iter()
            .any(|e| e.source == "ai" && e.detail.contains("Showing something turned on"))
    );
    assert!(events.iter().any(|e| e.detail.contains("showing-item judged true")));
    assert!(
        events
            .iter()
            .any(|e| e.source == "rule" && e.detail.contains("because Showing something turned on"))
    );
}

#[test]
fn low_confidence_counts_as_not_holding() {
    let mut engine = engine();
    let unsure = scene("", vec![verdict("pet-in-view", true, 0.3, "Cat")]);
    engine.observe_scene(unsure, 1_000);
    assert!(!engine.condition_on("pet-in-view"));
    let sure = scene("", vec![verdict("pet-in-view", true, 0.8, "Cat")]);
    engine.observe_scene(sure, 2_000);
    assert!(engine.condition_on("pet-in-view"));
    assert_eq!(engine.effective_props("pet-alert").unwrap().text, "Cat on stream!");
}

#[test]
fn confirm_requires_consecutive_positive_judgments() {
    let mut project = default_project();
    project.conditions[0].confirm = 2;
    let mut engine = Engine::new(project, 0).unwrap();
    holding(&mut engine, "book", 1_000);
    assert!(!engine.condition_on("showing-item"));
    put_down(&mut engine, 2_000);
    holding(&mut engine, "book", 3_000);
    assert!(
        !engine.condition_on("showing-item"),
        "the streak must restart after a negative"
    );
    holding(&mut engine, "book", 4_000);
    assert!(engine.condition_on("showing-item"));
}

#[test]
fn unknown_disabled_duplicate_and_old_verdicts_are_ignored() {
    let mut project = default_project();
    project.conditions[1].enabled = false;
    let mut engine = Engine::new(project, 0).unwrap();
    let noisy = scene(
        "",
        vec![
            verdict("no-such-condition", true, 1.0, ""),
            verdict("pet-in-view", true, 1.0, "Dog"),
            verdict("showing-item", false, 1.0, ""),
            verdict("showing-item", true, 1.0, "lamp"),
        ],
    );
    engine.observe_scene(noisy, 5_000);
    assert!(
        !engine.condition_on("pet-in-view"),
        "disabled conditions are not judged"
    );
    assert!(
        !engine.condition_on("showing-item"),
        "only the first verdict per condition counts"
    );
    let stale = engine.observe_scene(scene("old", vec![verdict("showing-item", true, 1.0, "x")]), 4_000);
    assert_eq!(stale, Default::default());
    assert_eq!(engine.snapshot(5_000).scene.caption, "");
}

#[test]
fn model_text_is_sanitized_before_it_reaches_the_overlay() {
    let mut engine = engine();
    let long = "x".repeat(500);
    let messy = scene(
        &format!("line one\nline\ttwo {long}"),
        vec![verdict("showing-item", true, 7.0, "a\u{7}b")],
    );
    engine.observe_scene(messy, 1_000);
    let snapshot = engine.snapshot(1_000);
    assert!(snapshot.scene.caption.starts_with("line one line two x"));
    assert_eq!(snapshot.scene.caption.chars().count(), 200);
    let condition = snapshot.conditions.iter().find(|c| c.id == "showing-item").unwrap();
    assert_eq!(condition.detail, "a b");
    assert_eq!(condition.confidence, 1.0);
}

#[test]
fn caption_placeholder_shows_the_latest_scene() {
    let mut engine = engine();
    let patch = sceneuron_core::PropsPatch {
        text: Some("AI sees: {{scene}}{{ unknown }}".to_owned()),
        ..Default::default()
    };
    engine.set_override("name-label", &patch, 0).unwrap();
    engine.observe_scene(scene("A cat on a keyboard", Vec::new()), 1_000);
    assert_eq!(
        engine.effective_props("name-label").unwrap().text,
        "AI sees: A cat on a keyboard"
    );
}

#[test]
fn paused_automation_tracks_conditions_without_running_rules() {
    let mut engine = engine();
    engine.set_automation(false, 0);
    holding(&mut engine, "mug", 1_000);
    assert!(engine.condition_on("showing-item"));
    assert!(!visible(&engine, "now-showing"));
    engine.set_automation(true, 2_000);
    assert!(
        visible(&engine, "now-showing"),
        "resuming applies the current condition states"
    );
}

#[test]
fn reset_turns_conditions_off_and_runs_their_off_rules() {
    let mut engine = engine();
    holding(&mut engine, "mug", 1_000);
    assert!(
        engine
            .reset_conditions("scene understanding was turned off", 2_000)
            .changed
    );
    assert!(!engine.condition_on("showing-item"));
    assert!(!visible(&engine, "now-showing"));
    assert_eq!(engine.effective_props("now-showing").unwrap().text, "Now showing: ");
    let events = engine.snapshot(2_000).events;
    assert!(
        events[1]
            .detail
            .contains("turned off: scene understanding was turned off")
    );
    assert_eq!(engine.reset_conditions("again", 3_000), Default::default());
}

#[test]
fn conditions_expire_when_the_model_stops_reporting() {
    let mut engine = engine();
    holding(&mut engine, "mug", 1_000);
    let stale_after = default_project().ai.stale_after_ms();
    assert!(!engine.tick(1_000 + stale_after).changed);
    assert!(engine.condition_on("showing-item"));
    assert!(engine.tick(1_001 + stale_after).changed);
    assert!(!engine.condition_on("showing-item"));
    assert!(!visible(&engine, "now-showing"));
}

#[test]
fn ai_status_transitions_publish_and_counters_are_telemetry() {
    let mut engine = engine();
    let status = |state: &str, calls: u64| AiStatus {
        state: state.to_owned(),
        calls,
        ..AiStatus::default()
    };
    assert!(engine.set_ai_status(status("watching", 0), 0).changed);
    let thinking = engine.set_ai_status(status("thinking", 0), 1);
    assert!(!thinking.changed && thinking.telemetry);
    let counted = engine.set_ai_status(status("watching", 1), 2);
    assert!(!counted.changed && counted.telemetry);
    let failed = AiStatus {
        message: "invalid API key".to_owned(),
        ..status("failed", 1)
    };
    assert!(engine.set_ai_status(failed.clone(), 3).changed);
    assert_eq!(engine.set_ai_status(failed, 4), Default::default());
    let snapshot = engine.snapshot(4);
    assert_eq!(snapshot.ai.state, "failed");
    assert!(
        snapshot.events[0]
            .detail
            .contains("vision model failed: invalid API key")
    );
}
