// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the facepalm reaction in the engine: a hand held over the face for a second plays the
//! email preset with the reason in the event log, a shorter one plays nothing, holding on plays
//! once, the cooldown it shares with rules and the tongue, the switch, the email's camera switch,
//! automation paused, and frames that arrive out of order. facepalm_frames.rs covers missed and
//! late frames at each sample rate.

use sceneuron_core::{
    Engine, FACEPALM_HOLD_MS, Flood, MIN_FLOOD_COOLDOWN_MS, Project, TONGUE_OUT, Verdict, default_project,
};

/// Frames five times a second, as the camera samples them by default.
const FRAME_MS: u64 = 200;

fn engine(project: Project) -> Engine {
    Engine::new(project, 0).unwrap()
}

/// Feeds a frame every 200 ms from `from_ms` up to and including `to_ms`, each showing `seen`.
fn frames(engine: &mut Engine, seen: bool, from_ms: u64, to_ms: u64) {
    for at_ms in (from_ms..=to_ms).step_by(FRAME_MS as usize) {
        engine.observe_facepalm(seen, at_ms);
    }
}

fn camera_events(engine: &Engine) -> Vec<String> {
    let events = engine.snapshot(0).events.into_iter();
    events
        .filter(|event| event.source == "camera")
        .map(|event| event.detail)
        .collect()
}

#[test]
fn a_hand_over_the_face_for_a_second_plays_the_email_and_says_why() {
    let mut engine = engine(default_project());
    frames(&mut engine, false, 0, 400);
    frames(&mut engine, true, 600, 1_400);
    assert_eq!(engine.flood(), None, "800 ms is not enough");
    let outcome = engine.observe_facepalm(true, 1_600);
    assert!(outcome.changed && !outcome.project_dirty);
    let flood = engine.flood().expect("the email reaction");
    assert_eq!((flood.flood, flood.at_ms), (Flood::Email, 1_600));
    assert_eq!(
        camera_events(&engine),
        [r#"played "Could have been an email" because a hand covered the face for 1.0 s"#]
    );
    assert_eq!(FACEPALM_HOLD_MS, 1_000);
}

#[test]
fn holding_on_plays_once_and_the_hand_must_leave_and_come_back() {
    let mut project = default_project();
    project.reactions.cooldown_ms = MIN_FLOOD_COOLDOWN_MS;
    let mut engine = engine(project);
    frames(&mut engine, true, 0, 10_000);
    assert_eq!(engine.flood().map(|flood| flood.at_ms), Some(1_000));
    frames(&mut engine, true, 10_200, 30_000);
    assert_eq!(
        engine.flood().map(|flood| flood.at_ms),
        Some(1_000),
        "still the same facepalm"
    );
    assert_eq!(camera_events(&engine).len(), 1);
    frames(&mut engine, false, 30_200, 31_000);
    frames(&mut engine, true, 31_200, 32_200);
    assert_eq!(engine.flood().map(|flood| flood.at_ms), Some(32_200), "a new facepalm");
}

#[test]
fn the_facepalm_waits_for_the_cooldown_of_rules_and_the_tongue() {
    let mut engine = engine(default_project());
    let tongue = Verdict {
        condition: TONGUE_OUT.to_owned(),
        holds: true,
        confidence: 0.95,
        detail: String::new(),
        reason: "The tongue is out.".to_owned(),
    };
    engine.observe_verdicts(vec![tongue], 1_000);
    assert_eq!(engine.flood().unwrap().flood, Flood::Tongue);
    frames(&mut engine, true, 2_000, 13_000);
    assert_eq!(
        engine.flood().unwrap().flood,
        Flood::Tongue,
        "held back by the cooldown"
    );
    assert_eq!(
        camera_events(&engine),
        [r#"saw a hand over the face for 1.0 s, without "Could have been an email": the last flood began 2 s ago"#]
    );
    frames(&mut engine, false, 13_200, 14_000);
    frames(&mut engine, true, 20_000, 21_000);
    assert_eq!(
        engine.flood().map(|flood| (flood.flood, flood.at_ms)),
        Some((Flood::Email, 21_000))
    );
}

#[test]
fn a_manual_flood_restarts_the_cooldown_for_the_facepalm() {
    let mut project = default_project();
    project.reactions.cooldown_ms = 20_000;
    let mut engine = engine(project);
    engine.trigger_flood(Flood::ShipIt, 1_000);
    frames(&mut engine, true, 5_000, 6_000);
    assert_eq!(engine.flood().unwrap().flood, Flood::ShipIt);
}

#[test]
fn the_switch_the_emails_camera_and_a_pause_keep_the_camera_from_playing_it() {
    let mut off = default_project();
    off.reactions.enabled = false;
    let mut camera_off = default_project();
    camera_off.reactions.camera_off = vec![Flood::Email];
    let mut tongue_off = default_project();
    tongue_off.reactions.camera_off = vec![Flood::Tongue];
    for (project, plays) in [(off, false), (camera_off, false), (tongue_off, true)] {
        let mut engine = engine(project);
        frames(&mut engine, true, 0, 2_000);
        assert_eq!(engine.flood().is_some(), plays);
    }
    let mut paused = engine(default_project());
    paused.set_automation(false, 0);
    frames(&mut paused, true, 0, 2_000);
    assert_eq!(paused.flood(), None);
    assert!(camera_events(&paused).is_empty());
}

#[test]
fn a_pause_forgets_the_facepalm_so_the_hold_starts_over_after_it() {
    let mut engine = engine(default_project());
    frames(&mut engine, true, 0, 800);
    engine.set_automation(false, 900);
    engine.observe_facepalm(true, 1_000);
    engine.set_automation(true, 1_100);
    frames(&mut engine, true, 1_200, 2_000);
    assert_eq!(engine.flood(), None, "the hold began again at 1.2 s");
    engine.observe_facepalm(true, 2_200);
    assert_eq!(engine.flood().map(|flood| flood.at_ms), Some(2_200));
}

#[test]
fn switching_the_emails_camera_off_mid_facepalm_forgets_it() {
    let mut engine = engine(default_project());
    frames(&mut engine, true, 0, 800);
    let mut camera_off = default_project();
    camera_off.reactions.camera_off = vec![Flood::Email];
    engine.replace_project(camera_off, 900).unwrap();
    engine.observe_facepalm(true, 1_000);
    engine.replace_project(default_project(), 1_100).unwrap();
    frames(&mut engine, true, 1_200, 2_000);
    assert_eq!(engine.flood(), None);
    engine.observe_facepalm(true, 2_200);
    assert!(engine.flood().is_some());
}

#[test]
fn frames_older_than_the_newest_are_ignored() {
    let mut engine = engine(default_project());
    frames(&mut engine, true, 0, 600);
    for late_ms in [100, 500] {
        assert!(!engine.observe_facepalm(false, late_ms).changed);
    }
    engine.observe_facepalm(true, 800);
    assert!(
        engine.observe_facepalm(true, 1_000).changed,
        "the late frames broke nothing"
    );
    assert_eq!(engine.flood().map(|flood| flood.at_ms), Some(1_000));
}
