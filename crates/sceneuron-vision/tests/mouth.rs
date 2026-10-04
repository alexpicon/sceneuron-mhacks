// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the mouth geometry and its hint: how far apart synthetic lip outlines are, about
//! the same for a slightly tilted head and the same in any frame shape, degenerate outlines, and
//! the open or closed hint that follows the face hint.

use std::f32::consts::PI;

use sceneuron_vision::{MOUTH_OPEN, Perception, Point, facts, mouth_fact, mouth_opening};

/// A 16:9 frame, as cameras send.
const ASPECT: f32 = 16.0 / 9.0;

/// `count` points around an ellipse centered at (0.5, 0.5), with half-axes in frame heights so
/// the shape is the same in any frame, turned by `turn` radians.
fn ellipse(half_width: f32, half_height: f32, count: usize, turn: f32, aspect: f32) -> Vec<Point> {
    (0..count)
        .map(|index| {
            let angle = 2.0 * PI * index as f32 / count as f32;
            let (x, y) = (half_width * angle.cos(), half_height * angle.sin());
            let (sin, cos) = turn.sin_cos();
            Point {
                x: 0.5 + (x * cos - y * sin) / aspect,
                y: 0.5 + x * sin + y * cos,
            }
        })
        .collect()
}

/// Lips parted by a gap whose height is `gap` mouth widths, in a mouth 0.1 frame heights wide.
fn lips(gap: f32, turn: f32, aspect: f32) -> (Vec<Point>, Vec<Point>) {
    let inner = ellipse(0.04, 0.05 * gap, 24, turn, aspect);
    let outer = ellipse(0.05, 0.03 + 0.05 * gap, 24, turn, aspect);
    (inner, outer)
}

fn measured(gap: f32, turn: f32, aspect: f32) -> f32 {
    let (inner, outer) = lips(gap, turn, aspect);
    mouth_opening(&inner, &outer, aspect).expect("lips are measured")
}

/// The mean gap of an elliptic opening is a quarter of pi times its height.
fn expected(gap: f32) -> f32 {
    PI / 4.0 * gap
}

#[test]
fn the_opening_is_the_mean_gap_between_the_lips_over_the_mouth_width() {
    for gap in [0.0, 0.1, 0.3, 0.6] {
        let opening = measured(gap, 0.0, ASPECT);
        assert!((opening - expected(gap)).abs() < 0.01, "gap {gap}: {opening}");
    }
}

#[test]
fn closed_lips_on_a_curve_measure_nothing() {
    let smile: Vec<Point> = (0..6)
        .map(|index| {
            let x = index as f32 / 5.0;
            Point {
                x: 0.45 + 0.1 * x,
                y: 0.6 - 0.03 * (1.0 - (2.0 * x - 1.0).powi(2)),
            }
        })
        .collect();
    let inner: Vec<Point> = smile.iter().chain(smile.iter().rev()).copied().collect();
    let (_, outer) = lips(0.0, 0.0, ASPECT);
    let opening = mouth_opening(&inner, &outer, ASPECT).unwrap();
    assert!(opening < 0.001, "{opening}");
}

#[test]
fn another_frame_shape_measures_the_same() {
    let wide = measured(0.3, 0.0, ASPECT);
    for aspect in [4.0 / 3.0, 1.0, 9.0 / 16.0] {
        let opening = measured(0.3, 0.0, aspect);
        assert!((opening - wide).abs() < 0.001, "aspect {aspect}: {opening}");
    }
}

#[test]
fn a_slightly_tilted_head_measures_about_the_same() {
    let upright = measured(0.3, 0.0, ASPECT);
    for turn in [0.26, -0.26] {
        let opening = measured(0.3, turn, ASPECT);
        assert!(
            opening >= upright && opening < upright * 1.1,
            "turn {turn}: {opening} vs {upright}"
        );
    }
}

#[test]
fn a_mouth_opened_taller_than_wide_measures_by_its_width() {
    let opening = measured(1.2, 0.0, 1.0);
    assert!((opening - expected(1.2).min(1.0)).abs() < 0.02, "{opening}");
}

#[test]
fn the_opening_is_at_most_one() {
    let inner = ellipse(0.05, 0.2, 24, 0.0, 1.0);
    let outer = ellipse(0.05, 0.25, 24, 0.0, 1.0);
    assert_eq!(mouth_opening(&inner, &outer, 1.0), Some(1.0));
}

#[test]
fn degenerate_outlines_are_not_measured() {
    let (inner, outer) = lips(0.3, 0.0, ASPECT);
    assert_eq!(mouth_opening(&inner[..2], &outer, ASPECT), None, "two inner points");
    assert_eq!(mouth_opening(&inner, &outer[..1], ASPECT), None, "one outer point");
    let mut broken = inner.clone();
    broken[3].y = f32::NAN;
    assert_eq!(
        mouth_opening(&broken, &outer, ASPECT),
        None,
        "a point that is not finite"
    );
    let dot = vec![Point { x: 0.5, y: 0.5 }; 4];
    assert_eq!(
        mouth_opening(&dot, &outer, ASPECT),
        None,
        "an inner outline without width"
    );
    assert_eq!(
        mouth_opening(&inner, &dot, ASPECT),
        None,
        "an outer outline without width"
    );
}

#[test]
fn the_hint_says_open_from_the_threshold_with_the_measure() {
    assert_eq!(mouth_fact(MOUTH_OPEN), format!("mouth open ({MOUTH_OPEN:.2})"));
    assert_eq!(mouth_fact(0.234), "mouth open (0.23)");
    assert_eq!(mouth_fact(0.01), "mouth closed (0.01)");
    let opening = measured(0.3, 0.0, ASPECT);
    assert!(mouth_fact(opening).starts_with("mouth open ("), "{opening}");
    assert!(mouth_fact(measured(0.0, 0.0, ASPECT)).starts_with("mouth closed ("));
}

#[test]
fn the_mouth_hint_follows_the_face_hint() {
    let perception = Perception {
        faces: vec![sceneuron_vision::FaceBox {
            x1: 0.4,
            y1: 0.2,
            x2: 0.6,
            y2: 0.5,
            score: 0.97,
        }],
        best_face: 0.97,
        mouth_opening: Some(0.21),
        ..Perception::default()
    };
    assert_eq!(facts(&perception), ["face in view (0.97)", "mouth open (0.21)"]);
    let unmeasured = Perception {
        mouth_opening: None,
        ..perception
    };
    assert_eq!(facts(&unmeasured), ["face in view (0.97)"]);
}
