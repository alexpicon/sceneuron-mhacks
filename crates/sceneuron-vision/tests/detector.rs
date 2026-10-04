// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the bundled UltraFace perceiver on real photo fixtures: faces are found with
//! sane boxes, a frame without a face reports none, only faces are reported, and wrong
//! input sizes are rejected.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use sceneuron_vision::{DetectError, FaceBox, INPUT_HEIGHT, INPUT_WIDTH, InputSize, Perceiver, Perception, UltraFace};

const FRAME_BYTES: usize = (INPUT_WIDTH * INPUT_HEIGHT * 3) as usize;

/// One detector shared by every test, since loading the model dominates test time.
fn detector() -> MutexGuard<'static, UltraFace> {
    static DETECTOR: OnceLock<Mutex<UltraFace>> = OnceLock::new();
    DETECTOR
        .get_or_init(|| Mutex::new(UltraFace::new().expect("bundled model loads")))
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

fn fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    assert_eq!(bytes.len(), FRAME_BYTES, "{name} is not a 320x240 RGB24 frame");
    bytes
}

fn detect(name: &str) -> Perception {
    detector()
        .perceive(&fixture(name), INPUT_WIDTH, INPUT_HEIGHT)
        .expect("analysis runs")
}

/// Asserts one confident face whose box lies inside the frame with a plausible size.
fn single_face(perception: &Perception) -> FaceBox {
    assert!(perception.best_face >= 0.9, "best face {}", perception.best_face);
    assert_eq!(perception.faces.len(), 1, "{perception:?}");
    let face = perception.faces[0];
    assert_eq!(face.score, perception.best_face);
    for value in [face.x1, face.y1, face.x2, face.y2] {
        assert!((0.0..=1.0).contains(&value), "{face:?}");
    }
    let (width, height) = (face.x2 - face.x1, face.y2 - face.y1);
    assert!(width > 0.03 && height > 0.03, "box too small: {face:?}");
    assert!(width < 0.9 && height < 0.9, "box too large: {face:?}");
    face
}

fn center(face: &FaceBox) -> (f32, f32) {
    ((face.x1 + face.x2) / 2.0, (face.y1 + face.y2) / 2.0)
}

#[test]
fn close_crop_has_one_face() {
    let face = single_face(&detect("face-close-320x240.rgb"));
    let (x, y) = center(&face);
    assert!((0.25..0.75).contains(&x) && (0.15..0.75).contains(&y), "{face:?}");
}

#[test]
fn full_portrait_has_one_smaller_face_in_the_upper_half() {
    let face = single_face(&detect("face-320x240.rgb"));
    let close = single_face(&detect("face-close-320x240.rgb"));
    let area = |face: &FaceBox| (face.x2 - face.x1) * (face.y2 - face.y1);
    assert!(area(&face) < area(&close), "{face:?} vs {close:?}");
    assert!(center(&face).1 < 0.5, "{face:?}");
}

#[test]
fn frame_without_a_face_has_none() {
    let perception = detect("empty-320x240.rgb");
    assert!(perception.best_face < 0.5, "best face {}", perception.best_face);
    assert!(perception.faces.is_empty(), "{perception:?}");
}

#[test]
fn reports_only_faces_at_a_fixed_input_size() {
    let perception = detect("face-close-320x240.rgb");
    let only_faces = Perception {
        faces: perception.faces.clone(),
        best_face: perception.best_face,
        ..Perception::default()
    };
    assert_eq!(perception, only_faces);
    let detector = detector();
    assert_eq!(detector.name(), "UltraFace");
    assert_eq!(
        detector.input_size(),
        InputSize::Fixed {
            width: INPUT_WIDTH,
            height: INPUT_HEIGHT
        }
    );
}

#[test]
fn repeated_detection_is_deterministic() {
    let first = detect("face-320x240.rgb");
    detect("empty-320x240.rgb");
    assert_eq!(detect("face-320x240.rgb"), first);
}

#[test]
fn wrong_input_size_is_rejected() {
    let frame = fixture("face-320x240.rgb");
    let mut detector = detector();
    let half = detector.perceive(&frame[..FRAME_BYTES / 4], INPUT_WIDTH / 2, INPUT_HEIGHT / 2);
    assert!(
        matches!(
            half,
            Err(DetectError::InputSize {
                width: 160,
                height: 120,
                ..
            })
        ),
        "{half:?}"
    );
    let short = detector.perceive(&frame[..FRAME_BYTES - 1], INPUT_WIDTH, INPUT_HEIGHT);
    assert!(
        matches!(short, Err(DetectError::InputSize { len, .. }) if len == FRAME_BYTES - 1),
        "{short:?}"
    );
    let message = short.unwrap_err().to_string();
    assert!(message.contains("320x240"), "{message}");
}
