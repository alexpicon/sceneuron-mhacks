// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises which face hands are measured against while a hand may hide it: the largest face clear
//! of hands, remembered for a while after the hand covers or hides it, and the face in view when no
//! clear face was seen lately.

mod support {
    pub mod face_hands;
}

use sceneuron_vision::{FACE_MEMORY_MS, FaceBox, FaceMemory};
use support::face_hands::{FACE, hand};

const OPEN: [bool; 4] = [true; 4];

fn face(x1: f32, size: f32) -> FaceBox {
    FaceBox {
        x1,
        y1: 0.2,
        x2: x1 + size,
        y2: 0.2 + size,
        score: 0.9,
    }
}

#[test]
fn a_face_clear_of_hands_is_measured_and_remembered() {
    let mut memory = FaceMemory::default();
    let (small, large) = (face(0.1, 0.1), face(0.5, 0.2));
    assert_eq!(memory.anchor(&[small, large], &[], 0), Some(large), "the largest face");
    assert_eq!(memory.anchor(&[], &[], 1_000), Some(large), "remembered while hidden");
    assert_eq!(memory.anchor(&[], &[], FACE_MEMORY_MS), Some(large));
    assert_eq!(memory.anchor(&[], &[], FACE_MEMORY_MS + 1), None, "forgotten");
}

#[test]
fn a_hand_in_the_face_keeps_the_face_from_before_it() {
    let mut memory = FaceMemory::default();
    assert_eq!(memory.anchor(&[FACE], &[], 0), Some(FACE));
    let picard = hand((1040.0, 590.0), -25.0, 180.0, OPEN);
    let shifted = FaceBox { y1: 0.45, ..FACE };
    assert_eq!(memory.anchor(&[shifted], &[picard], 600), Some(FACE));
    assert_eq!(memory.anchor(&[], &[picard], 1_200), Some(FACE), "hidden by the hand");
    assert_eq!(
        memory.anchor(&[shifted], &[picard], 2_100),
        Some(shifted),
        "too long ago"
    );
}

#[test]
fn without_a_clear_face_the_face_in_view_is_measured() {
    let mut memory = FaceMemory::default();
    let picard = hand((1040.0, 590.0), -25.0, 180.0, OPEN);
    assert_eq!(memory.anchor(&[FACE], &[picard], 0), Some(FACE));
    assert_eq!(memory.anchor(&[], &[picard], 100), None, "never seen clear");
    let elsewhere = hand((300.0, 900.0), 0.0, 150.0, OPEN);
    assert_eq!(memory.anchor(&[FACE], &[elsewhere], 200), Some(FACE));
    assert_eq!(
        memory.anchor(&[], &[], 300),
        Some(FACE),
        "remembered: no hand was in it"
    );
}
