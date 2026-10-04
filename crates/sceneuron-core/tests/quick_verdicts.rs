// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises verdicts that come without a caption, such as quick judgments from on-device facts,
//! and how they combine with slower scene observations that describe older frames.

use sceneuron_core::{Engine, SceneObservation, Verdict, default_project};

fn verdict(holds: bool, detail: &str, reason: &str) -> Verdict {
    Verdict {
        condition: "showing-item".to_owned(),
        holds,
        confidence: 0.9,
        detail: detail.to_owned(),
        reason: reason.to_owned(),
    }
}

fn scene(verdicts: Vec<Verdict>) -> SceneObservation {
    SceneObservation {
        caption: "A person holds up a mug".to_owned(),
        verdicts,
    }
}

#[test]
fn a_quick_verdict_turns_a_condition_on_without_touching_the_caption() {
    let mut engine = Engine::new(default_project(), 0).unwrap();
    engine.observe_scene(scene(Vec::new()), 500);
    let outcome = engine.observe_verdicts(vec![verdict(true, "", "quick judgment from on-device facts")], 1_000);
    assert!(outcome.changed);
    assert!(engine.condition_on("showing-item"));
    let snapshot = engine.snapshot(1_000);
    assert_eq!(snapshot.scene.caption, "A person holds up a mug");
    assert!(
        snapshot
            .events
            .iter()
            .any(|e| e.detail.contains("quick judgment from on-device facts"))
    );
}

#[test]
fn a_slower_answer_about_an_older_frame_names_the_item_but_cannot_undo_newer_evidence() {
    let mut engine = Engine::new(default_project(), 0).unwrap();
    engine.observe_verdicts(vec![verdict(true, "", "quick")], 2_000);
    assert_eq!(engine.effective_props("now-showing").unwrap().text, "Now showing: ");
    engine.observe_scene(scene(vec![verdict(true, "coffee mug", "slow")]), 1_500);
    assert_eq!(
        engine.effective_props("now-showing").unwrap().text,
        "Now showing: coffee mug"
    );
    engine.observe_scene(scene(vec![verdict(false, "", "slow")]), 1_600);
    engine.observe_scene(scene(vec![verdict(false, "", "slow")]), 1_700);
    assert!(
        engine.condition_on("showing-item"),
        "older evidence must not turn it off"
    );
    engine.observe_verdicts(vec![verdict(false, "", "quick")], 3_000);
    engine.observe_verdicts(vec![verdict(false, "", "quick")], 3_100);
    assert!(!engine.condition_on("showing-item"));
}

#[test]
fn quick_verdicts_keep_conditions_alive_without_scene_observations() {
    let mut engine = Engine::new(default_project(), 0).unwrap();
    let stale_after = default_project().ai.stale_after_ms();
    engine.observe_verdicts(vec![verdict(true, "", "quick")], 1_000);
    engine.observe_verdicts(vec![verdict(true, "", "quick")], 1_000 + stale_after);
    assert!(!engine.tick(1_001 + stale_after).changed);
    assert!(engine.condition_on("showing-item"));
}
