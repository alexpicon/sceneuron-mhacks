// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the live description in the engine: a new line shows once two frames in a row show
//! it, at most once every 400 ms, an open mouth lingers through speech, the line clears when
//! frames or camera analysis stop, and a change publishes at once only while an element shows it.

mod common;

use common::{engine, text, text_element, visible};
use sceneuron_core::{Engine, MOUTH_LINGER_MS, Project, SEEN_CHANGE_MS, Sighting, VisionStatus, default_project};

/// The default project with a visible text element "live" whose text is `text`.
fn showing(text: &str) -> Engine {
    let mut project: Project = default_project();
    project.elements.push(text_element("live", "Live description"));
    let element = project.elements.last_mut().unwrap();
    element.props.text = text.to_owned();
    element.props.visible = true;
    engine(project)
}

fn people(people: usize) -> Sighting {
    Sighting {
        people,
        ..Sighting::default()
    }
}

fn talking(open: bool) -> Sighting {
    Sighting {
        mouth_open: open,
        ..people(1)
    }
}

#[test]
fn a_new_line_shows_once_two_frames_in_a_row_show_it() {
    let mut engine = showing("Camera: {{seen}}");
    let revision = engine.revision();
    assert_eq!(
        engine.observe_sighting(&people(1), 0),
        Default::default(),
        "one frame is not enough"
    );
    assert_eq!(engine.seen(), "");
    assert_eq!(text(&engine, "live"), "Camera: ");
    assert!(engine.observe_sighting(&people(1), 200).changed);
    assert_eq!(engine.revision(), revision + 1);
    assert_eq!(engine.seen(), "1 person");
    assert_eq!(text(&engine, "live"), "Camera: 1 person");
    assert_eq!(engine.snapshot(200).seen, "1 person");
    assert_eq!(
        engine.observe_sighting(&people(1), 400),
        Default::default(),
        "the same line changes nothing"
    );
}

#[test]
fn a_single_odd_frame_never_reaches_the_line() {
    let mut engine = showing("{{seen}}");
    for at in [0, 200] {
        engine.observe_sighting(&people(1), at);
    }
    engine.observe_sighting(&people(0), 1_000);
    engine.observe_sighting(&people(1), 1_200);
    engine.observe_sighting(&people(0), 1_400);
    engine.observe_sighting(&people(1), 1_600);
    assert_eq!(engine.seen(), "1 person");
    engine.observe_sighting(&people(0), 1_800);
    assert!(engine.observe_sighting(&people(0), 2_000).changed);
    assert_eq!(engine.seen(), "Nobody in view");
}

#[test]
fn a_change_waits_for_the_least_time_since_the_last_one_and_the_tick_shows_it() {
    let mut engine = showing("{{seen}}");
    engine.observe_sighting(&people(1), 0);
    engine.observe_sighting(&people(1), 100);
    assert_eq!(engine.seen(), "1 person");
    engine.observe_sighting(&people(2), 200);
    assert!(
        !engine.observe_sighting(&people(2), 300).changed,
        "settled, but too soon"
    );
    assert_eq!(engine.seen(), "1 person");
    assert!(!engine.tick(100 + SEEN_CHANGE_MS - 1).changed);
    assert!(engine.tick(100 + SEEN_CHANGE_MS).changed);
    assert_eq!(engine.seen(), "2 people");
}

#[test]
fn a_flickering_camera_changes_the_line_at_most_two_and_a_half_times_a_second() {
    let mut engine = showing("{{seen}}");
    let mut changes = 0;
    // Five seconds at ten frames a second, the count of people changing every second frame.
    for frame in 0..50_u64 {
        let at = frame * 100;
        changes += u32::from(
            engine
                .observe_sighting(&people(1 + (frame / 2 % 2) as usize), at)
                .changed,
        );
        changes += u32::from(engine.tick(at + 50).changed);
    }
    let most = 5_000 / SEEN_CHANGE_MS as u32 + 1;
    assert!(changes > 1 && changes <= most, "{changes} changes, at most {most}");
}

#[test]
fn an_open_mouth_lingers_through_speech_and_goes_a_second_after_it_closes() {
    let mut engine = showing("{{seen}}");
    for (index, at) in (0..3_000).step_by(200).enumerate() {
        engine.observe_sighting(&talking(index % 2 == 0), at);
        if at >= 200 {
            assert_eq!(engine.seen(), "1 person; mouth open", "at {at} ms");
        }
    }
    // The mouth was last open at 2,800 ms.
    let closed_from = 2_800 + MOUTH_LINGER_MS;
    for at in (3_000..closed_from + 400).step_by(200) {
        engine.observe_sighting(&talking(false), at);
    }
    assert_eq!(engine.seen(), "1 person");
}

#[test]
fn an_open_mouth_does_not_linger_once_nobody_is_in_view() {
    let mut engine = showing("{{seen}}");
    for at in [0, 200] {
        engine.observe_sighting(&talking(true), at);
    }
    for at in [400, 600] {
        engine.observe_sighting(&people(0), at);
    }
    assert_eq!(engine.seen(), "Nobody in view");
}

#[test]
fn older_frames_are_ignored() {
    let mut engine = showing("{{seen}}");
    for at in [1_000, 1_200] {
        engine.observe_sighting(&people(1), at);
    }
    for at in [900, 1_100] {
        assert_eq!(engine.observe_sighting(&people(3), at), Default::default());
    }
    assert_eq!(engine.seen(), "1 person");
}

#[test]
fn the_line_clears_and_its_element_hides_once_frames_stop() {
    let mut engine = showing("{{seen}}");
    for at in [0, 200] {
        engine.observe_sighting(&people(1), at);
    }
    assert!(visible(&engine, "live"));
    let stale = engine.project().detection.effective_stale_ms();
    assert!(!engine.tick(200 + stale).changed);
    assert_eq!(engine.seen(), "1 person");
    assert!(engine.tick(200 + stale + 1).changed);
    assert_eq!(engine.seen(), "");
    assert!(!visible(&engine, "live"), "an empty live description hides its text");
    for at in [5_000, 5_200] {
        engine.observe_sighting(&people(0), at);
    }
    assert_eq!(engine.seen(), "Nobody in view", "frames bring it back");
}

#[test]
fn the_line_clears_when_camera_analysis_stops_or_fails() {
    for stopped in [
        VisionStatus::Idle,
        VisionStatus::Failed {
            message: "the camera went away".to_owned(),
        },
    ] {
        let mut engine = showing("{{seen}}");
        let running = VisionStatus::Running {
            latency_ms: 20.0,
            mouths: true,
            hands: true,
        };
        engine.set_vision_status(running, 0);
        for at in [0, 200] {
            engine.observe_sighting(&people(1), at);
        }
        assert!(engine.set_vision_status(stopped, 300).changed);
        assert_eq!(engine.seen(), "");
        assert_eq!(engine.snapshot(300).seen, "");
    }
}

#[test]
fn without_an_element_showing_it_a_change_is_telemetry() {
    let mut engine = engine(default_project());
    let revision = engine.revision();
    engine.observe_sighting(&people(1), 0);
    let outcome = engine.observe_sighting(&people(1), 200);
    assert!(!outcome.changed && outcome.telemetry, "{outcome:?}");
    assert_eq!(engine.revision(), revision);
    assert_eq!(engine.snapshot(200).seen, "1 person");
    let stale = engine.project().detection.effective_stale_ms();
    let cleared = engine.tick(200 + stale + 1);
    assert!(!cleared.changed && cleared.telemetry, "{cleared:?}");
}

#[test]
fn a_hold_or_a_card_that_shows_the_line_publishes_it_at_once() {
    let mut engine = engine(common::deck_project());
    let patch = sceneuron_core::PropsPatch {
        text: Some("{{seen}}".to_owned()),
        ..Default::default()
    };
    engine.set_override("brb", &patch, 0).unwrap();
    engine.observe_sighting(&people(2), 0);
    assert!(engine.observe_sighting(&people(2), 200).changed);
    assert_eq!(text(&engine, "brb"), "2 people");
}
