// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises how the facepalm reaction follows the camera's frames: a facepalm held for a second
//! plays at every sample rate the settings accept, one frame without the hand or a frame that a
//! slow analysis delays keeps it, two frames in a row without it end it, a fast camera keeps it
//! until the hand has been gone for 400 ms, and frames that stop for longer than the detection
//! stale time start it over.

use sceneuron_core::{Engine, Flood, default_project};

fn engine_at(fps: f64) -> Engine {
    let mut project = default_project();
    project.detection.sample_fps = fps;
    Engine::new(project, 0).unwrap()
}

/// Feeds a frame every `every_ms` from `from_ms` up to and including `to_ms`, each showing `seen`.
fn frames(engine: &mut Engine, seen: bool, from_ms: u64, to_ms: u64, every_ms: u64) {
    for at_ms in (from_ms..=to_ms).step_by(every_ms as usize) {
        engine.observe_facepalm(seen, at_ms);
    }
}

/// When the email reaction played, if it did.
fn played_at(engine: &Engine) -> Option<u64> {
    let flood = engine.flood().filter(|flood| flood.flood == Flood::Email);
    flood.map(|flood| flood.at_ms)
}

#[test]
fn a_held_facepalm_plays_once_at_every_sample_rate_the_settings_accept() {
    let rates = [
        (0.5, 2_000, 2_000),
        (1.0, 1_000, 1_000),
        (2.0, 500, 1_000),
        (5.0, 200, 1_000),
        (30.0, 34, 1_020),
    ];
    for (fps, every_ms, plays_ms) in rates {
        let mut engine = engine_at(fps);
        frames(&mut engine, true, 0, 8_000, every_ms);
        assert_eq!(played_at(&engine), Some(plays_ms), "{fps} frames a second");
    }
}

#[test]
fn one_frame_without_the_hand_keeps_the_facepalm_and_two_in_a_row_end_it() {
    for (fps, every_ms) in [(0.5, 2_000), (2.0, 500), (5.0, 200)] {
        let hold_ms = 1_000_u64.div_ceil(every_ms) * every_ms;

        let mut kept = engine_at(fps);
        kept.observe_facepalm(true, 0);
        kept.observe_facepalm(false, every_ms);
        frames(&mut kept, true, 2 * every_ms, 10_000, every_ms);
        let kept_ms = hold_ms.max(2 * every_ms);
        assert_eq!(
            played_at(&kept),
            Some(kept_ms),
            "one missed frame at {fps} frames a second"
        );

        let mut ended = engine_at(fps);
        ended.observe_facepalm(true, 0);
        frames(&mut ended, false, every_ms, 2 * every_ms, every_ms);
        frames(&mut ended, true, 3 * every_ms, 20_000, every_ms);
        let ended_ms = 3 * every_ms + hold_ms;
        assert_eq!(
            played_at(&ended),
            Some(ended_ms),
            "two missed frames at {fps} frames a second"
        );
    }
}

#[test]
fn a_frame_that_a_slow_analysis_delays_keeps_the_facepalm() {
    let mut engine = engine_at(5.0);
    for at_ms in [0, 200, 400, 860] {
        engine.observe_facepalm(true, at_ms);
    }
    assert_eq!(played_at(&engine), None);
    engine.observe_facepalm(true, 1_060);
    assert_eq!(played_at(&engine), Some(1_060));
}

#[test]
fn a_fast_camera_keeps_the_facepalm_until_the_hand_is_gone_for_400_ms() {
    let mut flicker = engine_at(25.0);
    frames(&mut flicker, true, 0, 400, 40);
    frames(&mut flicker, false, 440, 520, 40);
    frames(&mut flicker, true, 560, 2_000, 40);
    assert_eq!(played_at(&flicker), Some(1_000), "three missed frames, 120 ms");

    let mut gone = engine_at(25.0);
    frames(&mut gone, true, 0, 400, 40);
    frames(&mut gone, false, 440, 800, 40);
    frames(&mut gone, true, 840, 3_000, 40);
    assert_eq!(played_at(&gone), Some(1_840), "the hand was gone for 400 ms");
}

#[test]
fn frames_that_stop_for_longer_than_the_stale_time_start_the_facepalm_over() {
    let stale_ms = default_project().detection.effective_stale_ms();
    let mut kept = engine_at(5.0);
    kept.observe_facepalm(true, 0);
    kept.observe_facepalm(true, stale_ms);
    assert_eq!(played_at(&kept), Some(stale_ms), "no frame for the stale time");

    let mut stalled = engine_at(5.0);
    stalled.observe_facepalm(true, 0);
    stalled.observe_facepalm(true, stale_ms + 1);
    assert_eq!(played_at(&stalled), None);
    frames(&mut stalled, true, stale_ms + 201, stale_ms + 1_001, 200);
    assert_eq!(played_at(&stalled), Some(stale_ms + 1_001));
}
