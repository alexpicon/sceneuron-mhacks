// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Turns raw per-anchor face detector output into a [`Perception`]: thresholding, clamping,
//! and greedy non-maximum suppression.

use crate::perception::{FaceBox, Perception};

/// Lowest face probability reported as a face.
const MIN_FACE_SCORE: f32 = 0.5;
/// A candidate overlapping an already kept face by more than this IoU is suppressed.
const MAX_OVERLAP: f32 = 0.3;
/// Most faces reported per frame.
const MAX_FACES: usize = 32;

/// Builds a perception with faces only from every anchor's raw box and face probability.
pub(crate) fn decode(anchors: impl IntoIterator<Item = FaceBox>) -> Perception {
    let mut best_score = 0.0f32;
    let mut candidates = Vec::new();
    for anchor in anchors {
        if anchor.score.is_finite() {
            best_score = best_score.max(anchor.score);
        }
        if anchor.score >= MIN_FACE_SCORE
            && let Some(face) = clamped(anchor)
        {
            candidates.push(face);
        }
    }
    candidates.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut faces: Vec<FaceBox> = Vec::new();
    for candidate in candidates {
        if faces.len() == MAX_FACES {
            break;
        }
        if faces.iter().all(|kept| iou(kept, &candidate) <= MAX_OVERLAP) {
            faces.push(candidate);
        }
    }
    Perception {
        faces,
        best_face: best_score,
        ..Perception::default()
    }
}

/// Clamps a box to the frame; `None` when it is not finite or has no area inside it.
fn clamped(face: FaceBox) -> Option<FaceBox> {
    let coords = [face.x1, face.y1, face.x2, face.y2];
    if !coords.iter().all(|value| value.is_finite()) {
        return None;
    }
    let [x1, y1, x2, y2] = coords.map(|value| value.clamp(0.0, 1.0));
    (x2 > x1 && y2 > y1).then_some(FaceBox {
        x1,
        y1,
        x2,
        y2,
        score: face.score.min(1.0),
    })
}

fn area(face: &FaceBox) -> f32 {
    (face.x2 - face.x1).max(0.0) * (face.y2 - face.y1).max(0.0)
}

fn iou(a: &FaceBox, b: &FaceBox) -> f32 {
    let width = (a.x2.min(b.x2) - a.x1.max(b.x1)).max(0.0);
    let height = (a.y2.min(b.y2) - a.y1.max(b.y1)).max(0.0);
    let intersection = width * height;
    let union = area(a) + area(b) - intersection;
    if union > 0.0 { intersection / union } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face(x1: f32, y1: f32, x2: f32, y2: f32, score: f32) -> FaceBox {
        FaceBox { x1, y1, x2, y2, score }
    }

    #[test]
    fn best_score_counts_anchors_below_the_threshold() {
        let detection = decode([face(0.1, 0.1, 0.2, 0.2, 0.3), face(0.5, 0.5, 0.6, 0.6, 0.1)]);
        assert!(detection.faces.is_empty());
        assert_eq!(detection.best_face, 0.3);
    }

    #[test]
    fn overlapping_boxes_keep_the_highest_score() {
        let detection = decode([
            face(0.10, 0.10, 0.50, 0.50, 0.8),
            face(0.12, 0.12, 0.52, 0.52, 0.95),
            face(0.60, 0.60, 0.90, 0.90, 0.7),
        ]);
        let scores: Vec<f32> = detection.faces.iter().map(|face| face.score).collect();
        assert_eq!(scores, vec![0.95, 0.7]);
        assert_eq!(detection.best_face, 0.95);
    }

    #[test]
    fn light_overlap_is_not_suppressed() {
        let detection = decode([face(0.0, 0.0, 0.4, 0.4, 0.9), face(0.3, 0.0, 0.7, 0.4, 0.9)]);
        assert_eq!(detection.faces.len(), 2);
    }

    #[test]
    fn boxes_are_clamped_and_degenerate_ones_dropped() {
        let detection = decode([
            face(-0.2, -0.1, 0.5, 1.3, 0.9),
            face(0.7, 0.7, 0.7, 0.9, 0.9),
            face(f32::NAN, 0.0, 0.1, 0.1, 0.9),
            face(0.0, 0.0, 0.1, 0.1, f32::NAN),
        ]);
        assert_eq!(detection.faces, vec![face(0.0, 0.0, 0.5, 1.0, 0.9)]);
        assert_eq!(detection.best_face, 0.9);
    }

    #[test]
    fn at_most_max_faces_are_reported() {
        let grid = (0..100).map(|index| {
            let x = (index % 10) as f32 / 10.0;
            let y = (index / 10) as f32 / 10.0;
            face(x, y, x + 0.09, y + 0.09, 0.6 + index as f32 / 1000.0)
        });
        let detection = decode(grid);
        assert_eq!(detection.faces.len(), MAX_FACES);
        assert!(detection.faces.windows(2).all(|pair| pair[0].score >= pair[1].score));
    }

    #[test]
    fn no_anchors_yield_an_empty_detection() {
        assert_eq!(decode(std::iter::empty()), Perception::default());
    }
}
