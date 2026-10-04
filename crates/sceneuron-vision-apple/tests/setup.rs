// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises how the Apple Vision perceiver is set up and fed: analysis options, its name and
//! input size, frame sizes it accepts, and frames it rejects without harm.

mod support {
    pub mod fixtures;
}

use sceneuron_vision::{DetectError, InputSize, Perceiver};
use sceneuron_vision_apple::{AppleVision, INPUT_MAX_SIDE, Options};
use support::fixtures::{FIXTURE_HEIGHT, FIXTURE_WIDTH, fixture, single_face, vision};

#[test]
fn options_choose_the_analyses() {
    let faces_only = AppleVision::with_options(Options {
        mouths: false,
        hands: false,
        bodies: false,
        animals: false,
        labels: false,
        text: false,
        slow_every: 0,
    })
    .expect("Apple Vision starts");
    assert_eq!(faces_only.active(), ["faces"]);
    assert!(faces_only.skipped().is_empty());
    assert!(!faces_only.measures_mouths());
    let all = vision();
    assert_eq!(
        all.active(),
        ["faces", "mouths", "hands", "bodies", "animals", "labels", "text"]
    );
    assert!(all.skipped().is_empty(), "{:?}", all.skipped());
    assert!(all.measures_mouths());
}

#[test]
fn describes_itself_and_keeps_the_aspect_ratio() {
    let vision = vision();
    assert_eq!(vision.name(), "Apple Vision");
    let input = vision.input_size();
    assert_eq!(
        input,
        InputSize::Fit {
            max_width: INPUT_MAX_SIDE,
            max_height: INPUT_MAX_SIDE
        }
    );
    assert_eq!(input.for_frame(1920, 1080), (640, 360));
}

#[test]
fn accepts_any_reasonable_frame_size() {
    let mut vision = vision();
    for (width, height) in [(3, 3), (17, 9), (640, 360), (360, 640)] {
        let frame = vec![90u8; width as usize * height as usize * 3];
        let perception = vision.perceive(&frame, width, height).expect("analysis runs");
        assert!(perception.faces.is_empty(), "{width}x{height}: {perception:?}");
        let timing = vision.last_timing().expect("timed");
        assert!(timing.vision_ms > 0.0, "{timing:?}");
    }
}

#[test]
fn tiny_frames_fail_without_harm() {
    let mut vision = AppleVision::with_options(Options {
        slow_every: 1,
        ..Options::default()
    })
    .expect("Apple Vision starts");
    let tiny = vision.perceive(&[90; 3], 1, 1);
    assert!(
        matches!(&tiny, Err(DetectError::Inference(message)) if message.contains("too small")),
        "{tiny:?}"
    );
    let frame = fixture("face-close-320x240.rgb");
    let perception = vision
        .perceive(&frame, FIXTURE_WIDTH, FIXTURE_HEIGHT)
        .expect("analysis runs");
    single_face(&perception);
}

#[test]
fn rejects_buffers_that_do_not_match_their_size() {
    let mut vision = vision();
    let frame = fixture("face-320x240.rgb");
    let short = vision.perceive(&frame[..frame.len() - 1], FIXTURE_WIDTH, FIXTURE_HEIGHT);
    assert!(matches!(short, Err(DetectError::InputSize { .. })), "{short:?}");
    let empty = vision.perceive(&[], 0, 0);
    assert!(matches!(empty, Err(DetectError::InputSize { .. })), "{empty:?}");
}
