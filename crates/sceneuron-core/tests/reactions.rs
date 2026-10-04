// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the built-in fun reactions in the engine: the switch and its defaults, which
//! reactions watch while automation runs and their preset's camera is on, the tongue-out condition
//! turning on a flood with the reason in the event log, the cooldown between floods, verdicts
//! ignored while a reaction does not watch, and the cue that waits for an open mouth.

use sceneuron_core::{
    Cue, DEFAULT_FLOOD_COOLDOWN_MS, Engine, Flood, MIN_FLOOD_COOLDOWN_MS, Project, Reactions, SceneObservation,
    TONGUE_OUT, Verdict, default_project, reactions,
};

fn engine(project: Project) -> Engine {
    Engine::new(project, 0).unwrap()
}

fn tongue(holds: bool, confidence: f64) -> SceneObservation {
    let reason = if holds {
        "The tongue sticks out past the lips."
    } else {
        "The mouth is closed."
    };
    SceneObservation {
        caption: String::new(),
        verdicts: vec![Verdict {
            condition: TONGUE_OUT.to_owned(),
            holds,
            confidence,
            detail: String::new(),
            reason: reason.to_owned(),
        }],
    }
}

/// The event log, newest first, as source and detail.
fn events(engine: &Engine) -> Vec<(String, String)> {
    let events = engine.snapshot(0).events.into_iter();
    events.map(|event| (event.source, event.detail)).collect()
}

fn flood_nonce(engine: &Engine) -> Option<u64> {
    engine.flood().map(|flood| flood.nonce)
}

#[test]
fn reactions_are_on_by_default_three_seconds_apart() {
    assert_eq!(
        default_project().reactions,
        Reactions {
            enabled: true,
            cooldown_ms: DEFAULT_FLOOD_COOLDOWN_MS,
            camera_off: Vec::new(),
            voice_off: Vec::new(),
        }
    );
    let older: Project = serde_json::from_str(r#"{"version": 4}"#).unwrap();
    assert_eq!(older.reactions, Reactions::default());
    assert_eq!((MIN_FLOOD_COOLDOWN_MS, DEFAULT_FLOOD_COOLDOWN_MS), (1_000, 3_000));
}

#[test]
fn the_cooldown_must_be_one_second_to_an_hour() {
    for (cooldown_ms, valid) in [(999, false), (1_000, true), (3_600_000, true), (3_600_001, false)] {
        let mut project = default_project();
        project.reactions.cooldown_ms = cooldown_ms;
        assert_eq!(project.validate().is_ok(), valid, "{cooldown_ms}");
    }
}

#[test]
fn the_tongue_reaction_waits_for_an_open_mouth_and_plays_the_tongue_flood() {
    let [reaction] = reactions() else {
        panic!("one built-in reaction");
    };
    assert_eq!(reaction.condition.id, TONGUE_OUT);
    assert_eq!(
        reaction.condition.prompt,
        "Someone in view is sticking their tongue out"
    );
    assert_eq!(
        reaction.cue(),
        Cue {
            condition: TONGUE_OUT.to_owned(),
            fact: "mouth open".to_owned()
        }
    );
    assert_eq!(
        serde_json::to_value(&reaction.rule.actions).unwrap(),
        serde_json::json!([{"type": "flood", "flood": "tongue"}])
    );
}

#[test]
fn reactions_watch_only_while_switched_on_and_automation_runs() {
    let mut engine = engine(default_project());
    assert_eq!(engine.active_reactions(), reactions().iter().collect::<Vec<_>>());
    engine.set_automation(false, 10);
    assert!(engine.active_reactions().is_empty());
    engine.set_automation(true, 20);
    let mut off = default_project();
    off.reactions.enabled = false;
    engine.replace_project(off, 30).unwrap();
    assert!(engine.active_reactions().is_empty());
    let mut camera_off = default_project();
    camera_off.reactions.camera_off = vec![Flood::Tongue];
    engine.replace_project(camera_off, 40).unwrap();
    assert!(engine.active_reactions().is_empty(), "the tongue's camera is off");
    let mut facepalm_off = default_project();
    facepalm_off.reactions.camera_off = vec![Flood::Email];
    engine.replace_project(facepalm_off, 50).unwrap();
    assert_eq!(engine.active_reactions().len(), 1, "only the facepalm's camera is off");
}

#[test]
fn a_tongue_out_floods_the_overlay_and_logs_why() {
    let mut engine = engine(default_project());
    assert_eq!(engine.snapshot(0).flood, None);
    let outcome = engine.observe_scene(tongue(true, 0.9), 1_000);
    assert!(outcome.changed);
    assert!(engine.condition_on(TONGUE_OUT));
    let flood = engine.snapshot(1_000).flood.expect("a flood");
    assert_eq!((flood.flood, flood.at_ms), (Flood::Tongue, 1_000));
    let log = events(&engine);
    assert_eq!(
        log[..2],
        [
            (
                "rule".to_owned(),
                r#"ran "Tongue-out flood" because Tongue out turned on"#.to_owned()
            ),
            (
                "ai".to_owned(),
                "Tongue out turned on: The tongue sticks out past the lips.".to_owned()
            ),
        ]
    );
    engine.observe_scene(tongue(false, 0.9), 2_000);
    assert!(!engine.condition_on(TONGUE_OUT), "one negative turns the reaction off");
    assert_eq!(events(&engine)[0].1, "Tongue out turned off: The mouth is closed.");
    assert_eq!(engine.flood().map(|flood| flood.at_ms), Some(1_000));
}

#[test]
fn an_unsure_tongue_plays_nothing() {
    let mut engine = engine(default_project());
    engine.observe_scene(tongue(true, 0.65), 1_000);
    assert!(!engine.condition_on(TONGUE_OUT));
    assert_eq!(engine.flood(), None);
}

#[test]
fn floods_from_rules_wait_for_the_cooldown() {
    let mut project = default_project();
    project.reactions.cooldown_ms = 20_000;
    let mut engine = engine(project);
    engine.observe_scene(tongue(true, 0.9), 1_000);
    let first = flood_nonce(&engine);
    engine.observe_scene(tongue(false, 0.9), 2_000);
    engine.observe_scene(tongue(true, 0.9), 13_500);
    assert!(engine.condition_on(TONGUE_OUT));
    assert_eq!(flood_nonce(&engine), first, "a flood within the cooldown");
    assert_eq!(
        events(&engine)[0].1,
        r#"ran "Tongue-out flood" because Tongue out turned on, without its flood: the last one began 12 s ago"#
    );
    engine.observe_scene(tongue(false, 0.9), 15_000);
    engine.observe_scene(tongue(true, 0.9), 21_000);
    assert!(flood_nonce(&engine) > first, "the cooldown passed");
    assert_eq!(engine.flood().unwrap().at_ms, 21_000);
}

#[test]
fn a_longer_cooldown_holds_floods_back_longer() {
    let mut project = default_project();
    project.reactions.cooldown_ms = 60_000;
    let mut engine = engine(project);
    engine.observe_scene(tongue(true, 0.9), 1_000);
    engine.observe_scene(tongue(false, 0.9), 2_000);
    engine.observe_scene(tongue(true, 0.9), 40_000);
    assert_eq!(engine.flood().unwrap().at_ms, 1_000);
    engine.observe_scene(tongue(false, 0.9), 41_000);
    engine.observe_scene(tongue(true, 0.9), 61_000);
    assert_eq!(engine.flood().unwrap().at_ms, 61_000);
}

#[test]
fn reactions_ignore_verdicts_while_switched_off_or_paused() {
    let mut off = default_project();
    off.reactions.enabled = false;
    let mut engine = engine(off);
    engine.observe_scene(tongue(true, 0.9), 1_000);
    assert!(!engine.condition_on(TONGUE_OUT));
    assert_eq!(engine.flood(), None);
    let mut paused = self::engine(default_project());
    paused.set_automation(false, 500);
    paused.observe_scene(tongue(true, 0.9), 1_000);
    assert!(!paused.condition_on(TONGUE_OUT));
    assert_eq!(paused.flood(), None);
}

#[test]
fn saving_the_project_keeps_the_reaction_on_and_a_reset_names_it() {
    let mut engine = engine(default_project());
    engine.observe_scene(tongue(true, 0.9), 1_000);
    engine.replace_project(default_project(), 2_000).unwrap();
    assert!(engine.condition_on(TONGUE_OUT));
    engine.reset_conditions("scene understanding stopped", 3_000);
    assert!(!engine.condition_on(TONGUE_OUT));
    assert_eq!(
        events(&engine)[0].1,
        "Tongue out turned off: scene understanding stopped"
    );
}

#[test]
fn a_pause_or_the_switch_turned_off_forgets_the_reaction_so_it_floods_anew() {
    let mut engine = engine(default_project());
    engine.observe_scene(tongue(true, 0.9), 1_000);
    engine.set_automation(false, 2_000);
    assert!(!engine.condition_on(TONGUE_OUT));
    engine.set_automation(true, 3_000);
    engine.observe_scene(tongue(true, 0.9), 30_000);
    assert_eq!(
        engine.flood().unwrap().at_ms,
        30_000,
        "the tongue after the pause floods"
    );
    let mut off = default_project();
    off.reactions.enabled = false;
    engine.replace_project(off, 31_000).unwrap();
    assert!(!engine.condition_on(TONGUE_OUT));
    engine.replace_project(default_project(), 32_000).unwrap();
    engine.observe_scene(tongue(true, 0.9), 60_000);
    assert_eq!(
        engine.flood().unwrap().at_ms,
        60_000,
        "the tongue after switching back on floods"
    );
}

#[test]
fn a_cue_is_its_fact_alone_or_with_a_measure() {
    let cue = reactions()[0].cue();
    let seen = |facts: &[&str]| cue.seen_in(&facts.iter().map(|fact| (*fact).to_owned()).collect::<Vec<_>>());
    assert!(seen(&["face in view (0.98)", "mouth open (0.21)"]));
    assert!(seen(&["mouth open"]));
    assert!(!seen(&["face in view (0.98)", "mouth closed (0.02)"]));
    assert!(!seen(&["mouth opened wide"]));
    assert!(!seen(&["text: \"mouth open\""]));
    assert!(!seen(&[]));
}
