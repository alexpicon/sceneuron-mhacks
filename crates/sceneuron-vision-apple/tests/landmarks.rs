// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises, without running Vision, how face landmarks become the mouth hint: landmark points
//! normalized to their face's box move into the frame, the largest confident face is the one
//! measured, and synthetic lips in Vision's convention read as an open or a closed mouth.

use sceneuron_vision::{MOUTH_OPEN, Point, mouth_fact, mouth_opening};
use sceneuron_vision_apple::select::{
    MAX_MEASURED_MOUTHS, MIN_FACE_CONFIDENCE, VisionRect, landmark_point, largest_face, measured_faces, widest_opening,
};

const WIDTH: f64 = 640.0;
const HEIGHT: f64 = 360.0;

fn rect(x: f64, y: f64, width: f64, height: f64) -> VisionRect {
    VisionRect { x, y, width, height }
}

/// A face box in Vision's convention, 0.3 of the frame wide and as tall in pixels.
fn face() -> VisionRect {
    rect(0.35, 0.2, 0.3, 0.3 * WIDTH / HEIGHT)
}

/// `count` points around an ellipse in pixels, centered on the face's mouth, in Vision's
/// landmark convention: normalized to the face box, origin at its bottom left.
fn outline(face: VisionRect, half_width_px: f64, half_height_px: f64, count: usize) -> Vec<(f64, f64)> {
    let (box_width, box_height) = (face.width * WIDTH, face.height * HEIGHT);
    (0..count)
        .map(|index| {
            let angle = std::f64::consts::TAU * index as f64 / count as f64;
            let x = box_width * 0.5 + half_width_px * angle.cos();
            let y = box_height * 0.25 + half_height_px * angle.sin();
            (x / box_width, y / box_height)
        })
        .collect()
}

fn in_frame(face: VisionRect, points: &[(f64, f64)]) -> Vec<Point> {
    points.iter().map(|&(x, y)| landmark_point(face, x, y)).collect()
}

/// The measure of lips whose inner outline is `gap_px` pixels high in a mouth 60 pixels wide.
fn measure(gap_px: f64) -> f32 {
    let face = face();
    let inner = in_frame(face, &outline(face, 24.0, gap_px / 2.0, 24));
    let outer = in_frame(face, &outline(face, 30.0, gap_px / 2.0 + 8.0, 24));
    mouth_opening(&inner, &outer, (WIDTH / HEIGHT) as f32).expect("lips are measured")
}

fn hint(gap_px: f64) -> String {
    mouth_fact(measure(gap_px))
}

#[test]
fn landmark_points_move_from_their_face_box_into_the_frame() {
    let face = rect(0.2, 0.4, 0.4, 0.5);
    assert_eq!(landmark_point(face, 0.0, 0.0), Point { x: 0.2, y: 0.6 });
    assert_eq!(landmark_point(face, 1.0, 1.0), Point { x: 0.6, y: 0.1 });
    assert_eq!(landmark_point(face, 0.5, 0.2), Point { x: 0.4, y: 0.5 });
}

#[test]
fn the_largest_confident_face_is_measured() {
    let small = (rect(0.1, 0.1, 0.1, 0.1), 0.99);
    let large = (rect(0.5, 0.2, 0.3, 0.4), 0.8);
    let larger_but_unsure = (rect(0.0, 0.0, 0.6, 0.6), MIN_FACE_CONFIDENCE - 0.01);
    assert_eq!(largest_face([small, large, larger_but_unsure]), Some(1));
    assert_eq!(largest_face([small]), Some(0));
    assert_eq!(largest_face([larger_but_unsure]), None);
    assert_eq!(largest_face([]), None);
    let broken = (rect(0.1, 0.1, f64::NAN, 0.5), 0.9);
    assert_eq!(largest_face([broken, small]), Some(1));
}

#[test]
fn every_confident_face_has_its_mouth_measured_largest_first() {
    let small = (rect(0.1, 0.1, 0.1, 0.1), 0.99);
    let large = (rect(0.5, 0.2, 0.3, 0.4), 0.8);
    let unsure = (rect(0.0, 0.0, 0.6, 0.6), MIN_FACE_CONFIDENCE - 0.01);
    assert_eq!(
        measured_faces([small, large, unsure]),
        [1, 0],
        "a second person's mouth counts too"
    );
    assert_eq!(measured_faces([unsure]), Vec::<usize>::new());
    let many: Vec<_> = (0..10)
        .map(|n| (rect(0.0, 0.0, 0.05 + n as f64 * 0.01, 0.1), 0.9))
        .collect();
    assert_eq!(measured_faces(many), [9, 8, 7, 6]);
    assert_eq!(MAX_MEASURED_MOUTHS, 4);
}

#[test]
fn the_widest_open_mouth_in_view_is_the_one_reported() {
    assert_eq!(widest_opening([Some(0.02), None, Some(0.21), Some(0.1)]), Some(0.21));
    assert_eq!(widest_opening([None, None]), None);
    assert_eq!(widest_opening([Some(f32::NAN), Some(0.05)]), Some(0.05));
}

#[test]
fn closed_synthetic_lips_read_as_a_closed_mouth() {
    assert_eq!(hint(0.0), "mouth closed (0.00)");
    assert!(hint(2.0).starts_with("mouth closed ("), "{}", hint(2.0));
}

#[test]
fn parted_synthetic_lips_read_as_an_open_mouth() {
    // A 12-pixel gap in a 60-pixel mouth: the mean gap of an ellipse is a quarter of pi of it.
    let opening = measure(12.0);
    let expected = std::f32::consts::FRAC_PI_4 * 0.2;
    assert!((opening - expected).abs() < 0.005, "{opening}");
    assert_eq!(hint(12.0), format!("mouth open ({opening:.2})"));
    assert!(measure(30.0) > opening);
    let threshold_px = f64::from(MOUTH_OPEN) * 60.0 * 4.0 / std::f64::consts::PI;
    assert!(hint(threshold_px + 1.0).starts_with("mouth open ("));
    assert!(hint(threshold_px - 1.0).starts_with("mouth closed ("));
}
