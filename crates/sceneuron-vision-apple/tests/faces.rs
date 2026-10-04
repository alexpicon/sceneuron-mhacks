// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises face and person detection by the Apple Vision perceiver, on the photo fixtures
//! shared with the vision crate, directly and inside the background worker, and how far apart the
//! lips of the face are, closed as photographed and open once an open mouth with a tongue is
//! painted on.

mod support {
    pub mod fixtures;
}

use std::sync::mpsc::channel;
use std::time::Duration;

use sceneuron_vision::{FaceBox, MOUTH_OPEN, Perceiver, Perception, Sample, VisionWorker, WorkerEvent};
use sceneuron_vision_apple::AppleVision;
use support::fixtures::{FIXTURE_HEIGHT, FIXTURE_WIDTH, fixture, single_face, vision};

fn perceive_fixture(name: &str) -> Perception {
    vision()
        .perceive(&fixture(name), FIXTURE_WIDTH, FIXTURE_HEIGHT)
        .expect("analysis runs")
}

fn center(face: &FaceBox) -> (f32, f32) {
    ((face.x1 + face.x2) / 2.0, (face.y1 + face.y2) / 2.0)
}

fn area(face: &FaceBox) -> f32 {
    (face.x2 - face.x1) * (face.y2 - face.y1)
}

#[test]
fn close_crop_has_one_face_in_the_middle() {
    let face = single_face(&perceive_fixture("face-close-320x240.rgb"));
    let (x, y) = center(&face);
    assert!((0.4..0.8).contains(&x) && (0.4..0.9).contains(&y), "{face:?}");
}

#[test]
fn full_portrait_has_one_smaller_face_in_the_upper_half() {
    let face = single_face(&perceive_fixture("face-320x240.rgb"));
    let close = single_face(&perceive_fixture("face-close-320x240.rgb"));
    assert!(area(&face) < area(&close), "{face:?} vs {close:?}");
    assert!(center(&face).1 < 0.5, "{face:?}");
}

#[test]
fn frame_without_a_face_has_none() {
    let perception = perceive_fixture("empty-320x240.rgb");
    assert!(perception.faces.is_empty(), "{perception:?}");
    assert!(perception.best_face < 0.5, "{perception:?}");
}

#[test]
fn the_full_portrait_shows_one_person_with_arms_down() {
    let perception = perceive_fixture("face-320x240.rgb");
    assert_eq!(perception.bodies.len(), 1, "{perception:?}");
    assert!(!perception.bodies[0].arms_up, "{perception:?}");
    assert!(perception.hands.len() <= 2, "{perception:?}");
}

/// The close-up portrait with an open mouth painted over the lips: a dark opening with a pink
/// tongue over its lower edge.
fn open_mouth() -> Vec<u8> {
    let mut rgb = fixture("face-close-320x240.rgb");
    let (cx, cy, rx, ry) = (185.0, 182.0, 15.0, 8.0);
    let inside =
        |x: f32, y: f32, cx: f32, cy: f32, rx: f32, ry: f32| ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) <= 1.0;
    for y in 0..FIXTURE_HEIGHT {
        for x in 0..FIXTURE_WIDTH {
            let (px, py) = (x as f32, y as f32);
            let color = if inside(px, py, cx, cy + ry * 0.9, rx * 0.6, ry * 1.1 + 4.0) {
                [214, 92, 110]
            } else if inside(px, py, cx, cy, rx, ry) {
                [40, 12, 14]
            } else {
                continue;
            };
            let at = (y * FIXTURE_WIDTH + x) as usize * 3;
            rgb[at..at + 3].copy_from_slice(&color);
        }
    }
    rgb
}

#[test]
fn the_photographed_lips_are_closed() {
    for name in ["face-close-320x240.rgb", "face-320x240.rgb"] {
        let perception = perceive_fixture(name);
        let opening = perception.mouth_opening.expect("the lips are located");
        assert!((0.0..MOUTH_OPEN).contains(&opening), "{name}: {perception:?}");
    }
}

#[test]
fn a_painted_open_mouth_with_a_tongue_is_open() {
    let perception = vision()
        .perceive(&open_mouth(), FIXTURE_WIDTH, FIXTURE_HEIGHT)
        .expect("analysis runs");
    single_face(&perception);
    let opening = perception.mouth_opening.expect("the lips are located");
    assert!(opening >= MOUTH_OPEN, "{perception:?}");
}

#[test]
fn no_face_has_no_mouth() {
    assert_eq!(perceive_fixture("empty-320x240.rgb").mouth_opening, None);
}

#[test]
fn runs_in_the_background_worker() {
    let (sender, events) = channel();
    let worker = VisionWorker::spawn(AppleVision::new, move |event| {
        let _ = sender.send(event);
    });
    let wait = Duration::from_secs(120);
    assert_eq!(
        events.recv_timeout(wait).expect("ready"),
        WorkerEvent::Ready {
            perceiver: "Apple Vision"
        }
    );
    assert!(worker.try_submit(Sample {
        rgb: fixture("face-close-320x240.rgb"),
        width: FIXTURE_WIDTH,
        height: FIXTURE_HEIGHT,
        captured_ms: 11,
    }));
    let WorkerEvent::Detected {
        perception,
        captured_ms,
        ..
    } = events.recv_timeout(wait).expect("perception")
    else {
        panic!("expected a perception");
    };
    assert_eq!(captured_ms, 11);
    single_face(&perception);
    worker.shutdown();
}
