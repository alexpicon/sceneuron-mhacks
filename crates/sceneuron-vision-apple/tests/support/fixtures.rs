// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The shared photo fixtures of the vision crate, one perceiver shared per test binary, and
//! the face assertions both use.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use sceneuron_vision::{FaceBox, Perception};
use sceneuron_vision_apple::AppleVision;

pub const FIXTURE_WIDTH: u32 = 320;
pub const FIXTURE_HEIGHT: u32 = 240;

/// One perceiver with default options shared by a test binary, since starting one loads
/// every model.
pub fn vision() -> MutexGuard<'static, AppleVision> {
    static VISION: OnceLock<Mutex<AppleVision>> = OnceLock::new();
    VISION
        .get_or_init(|| Mutex::new(AppleVision::new().expect("Apple Vision starts")))
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

/// A raw 320x240 RGB24 fixture from `crates/sceneuron-vision/tests/fixtures`.
pub fn fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../sceneuron-vision/tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
}

/// Asserts one face that presence would count (the default minimum is 0.7) inside the frame.
pub fn single_face(perception: &Perception) -> FaceBox {
    assert!(perception.best_face >= 0.7, "{perception:?}");
    assert_eq!(perception.faces.len(), 1, "{perception:?}");
    let face = perception.faces[0];
    assert_eq!(face.score, perception.best_face);
    for value in [face.x1, face.y1, face.x2, face.y2] {
        assert!((0.0..=1.0).contains(&value), "{face:?}");
    }
    face
}
