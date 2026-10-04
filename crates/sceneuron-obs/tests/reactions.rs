// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises how on-device analysis meets the built-in reactions inside OBS: the facts the
//! plugin gives the director show the tongue-out reaction's cue for an open mouth only.

use sceneuron_core::reactions;
use sceneuron_vision::{FaceBox, MOUTH_OPEN, Perception, facts};

fn face_with_mouth(opening: Option<f32>) -> Perception {
    Perception {
        faces: vec![FaceBox {
            x1: 0.4,
            y1: 0.2,
            x2: 0.6,
            y2: 0.5,
            score: 0.96,
        }],
        best_face: 0.96,
        mouth_opening: opening,
        ..Perception::default()
    }
}

#[test]
fn an_open_mouth_shows_the_tongue_reactions_cue() {
    let cue = reactions()[0].cue();
    assert!(cue.seen_in(&facts(&face_with_mouth(Some(0.27)))));
    assert!(cue.seen_in(&facts(&face_with_mouth(Some(MOUTH_OPEN)))));
}

#[test]
fn a_closed_or_unmeasured_mouth_does_not() {
    let cue = reactions()[0].cue();
    assert!(!cue.seen_in(&facts(&face_with_mouth(Some(0.02)))));
    assert!(!cue.seen_in(&facts(&face_with_mouth(None))));
    assert!(!cue.seen_in(&facts(&Perception::default())));
}
