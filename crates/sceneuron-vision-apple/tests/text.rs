// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises text recognition by the Apple Vision perceiver, on frames rendered with CoreText,
//! and the cadence that repeats the slow analyses between their runs.

mod support {
    pub mod render;
}

use sceneuron_vision::Perceiver;
use sceneuron_vision_apple::{AppleVision, Options};
use support::render::text_frame;

/// Lowercase letters and digits only, so recognition can be compared loosely.
fn letters(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_lowercase()
}

#[test]
fn recognizes_rendered_text() {
    let frame = text_frame(640, 360, 72.0, &[("OAT MILK", 140.0, 200.0), ("Barista", 200.0, 80.0)]);
    let mut vision = AppleVision::with_options(Options {
        slow_every: 1,
        ..Options::default()
    })
    .expect("Apple Vision starts");
    let perception = vision.perceive(&frame, 640, 360).expect("analysis runs");
    let mut lines: Vec<String> = perception.text.iter().map(|line| letters(&line.text)).collect();
    lines.sort();
    assert_eq!(lines, ["barista", "oatmilk"], "{perception:?}");
    assert!(
        perception.text.iter().all(|line| line.confidence >= 0.5),
        "{perception:?}"
    );
    assert!(perception.faces.is_empty(), "{perception:?}");
}

#[test]
fn slow_results_repeat_until_the_next_slow_frame() {
    let mut vision = AppleVision::with_options(Options {
        mouths: false,
        hands: false,
        bodies: false,
        animals: false,
        labels: false,
        text: true,
        slow_every: 2,
    })
    .expect("Apple Vision starts");
    let sign = text_frame(640, 360, 96.0, &[("LIVE", 200.0, 140.0)]);
    let blank = text_frame(640, 360, 96.0, &[]);
    let read = |vision: &mut AppleVision, frame: &[u8]| -> Vec<String> {
        let perception = vision.perceive(frame, 640, 360).expect("analysis runs");
        perception.text.iter().map(|line| letters(&line.text)).collect()
    };
    assert_eq!(read(&mut vision, &sign), ["live"]);
    assert!(vision.last_timing().expect("timed").slow);
    assert_eq!(read(&mut vision, &blank), ["live"], "repeated on a fast frame");
    assert!(!vision.last_timing().expect("timed").slow);
    assert!(read(&mut vision, &blank).is_empty(), "refreshed on the next slow frame");
}
