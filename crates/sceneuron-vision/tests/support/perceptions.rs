// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Builds perceptions for the hint and novelty tests.

use sceneuron_vision::{Chirality, FaceBox, Gesture, Hand, Label, Perception, TextLine};

pub fn face(score: f32) -> FaceBox {
    FaceBox {
        x1: 0.4,
        y1: 0.2,
        x2: 0.6,
        y2: 0.5,
        score,
    }
}

pub fn faces(scores: &[f32]) -> Perception {
    Perception {
        faces: scores.iter().copied().map(face).collect(),
        best_face: scores.iter().copied().fold(0.0, f32::max),
        ..Perception::default()
    }
}

pub fn label(name: &str, confidence: f32) -> Label {
    Label {
        name: name.to_owned(),
        confidence,
    }
}

pub fn hand(chirality: Chirality, raised: bool, gesture: Option<Gesture>) -> Hand {
    Hand {
        chirality,
        raised,
        gesture,
        extended: None,
        over_face: false,
    }
}

/// `hand` showing `extended` fingers.
pub fn counted(hand: Hand, extended: u8) -> Hand {
    Hand {
        extended: Some(extended),
        ..hand
    }
}

pub fn text(lines: &[&str]) -> Perception {
    Perception {
        text: lines
            .iter()
            .map(|line| TextLine {
                text: (*line).to_owned(),
                confidence: 1.0,
            })
            .collect(),
        ..Perception::default()
    }
}
