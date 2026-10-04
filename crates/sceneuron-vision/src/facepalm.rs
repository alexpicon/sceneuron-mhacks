// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Facepalm geometry: whether a hand covers the upper half of a face, from the hand's joints and
//! the face's box, and which face to measure while the hand hides it from the face detector. The
//! functions are pure so that any pose model can share them.
//!
//! The hand's outline is the convex hull of its located joints, which spans the palm and the
//! fingers. It covers a face when all of these hold:
//!
//! - It covers [`FACEPALM_COVER`] of the upper half of the face's box, where the eyes and brows
//!   are. A hand under the chin, over the mouth, or cupping a jaw stays below that half.
//! - It covers [`FACEPALM_MIDDLE_COVER`] of that half's middle third, between the eyes. A hand
//!   scratching an ear, propping up a temple, or holding a cheek stays on one side of the face.
//! - At least [`FACEPALM_MIN_JOINTS`] joints were located, so the outline is the hand's.
//! - The outline is at most [`FACEPALM_MAX_HAND_AREA`] times the face's box: a larger hand is
//!   near the camera, not on the face.
//!
//! Coverage is measured on a grid of points, and every measure is a share of an area, so it does
//! not depend on the frame's aspect ratio.

use crate::perception::FaceBox;
use crate::pose::{HandJoints, Point};

/// The least share of the upper half of the face's box a hand covers in a facepalm.
pub const FACEPALM_COVER: f32 = 0.3;
/// The least share of the middle third of that upper half a hand covers in a facepalm.
pub const FACEPALM_MIDDLE_COVER: f32 = 0.4;
/// The fewest located joints, of 21, that outline a hand.
pub const FACEPALM_MIN_JOINTS: usize = 8;
/// The largest hand outline, as a multiple of the face box's area, that can lie on the face.
pub const FACEPALM_MAX_HAND_AREA: f32 = 2.0;
/// How long a face seen with no hand in front of it is measured against after it was last seen,
/// since a hand over the eyes often hides the face from the face detector.
pub const FACE_MEMORY_MS: u64 = 2_000;

/// Grid points across the face's box and down its upper half; the middle third is the middle
/// `GRID_COLUMNS / 3` columns.
const GRID_COLUMNS: usize = 24;
const GRID_ROWS: usize = 12;

/// How much of a face one hand covers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceCover {
    /// The share of the upper half of the face's box the hand's outline covers, `0.0..=1.0`.
    pub upper: f32,
    /// The share of the middle third of that upper half it covers, `0.0..=1.0`.
    pub middle: f32,
    /// The outline's area as a multiple of the face box's area.
    pub area: f32,
    /// How many of the hand's joints were located.
    pub joints: usize,
}

impl FaceCover {
    /// Whether this is a facepalm; see the module's description.
    pub fn is_facepalm(&self) -> bool {
        self.joints >= FACEPALM_MIN_JOINTS
            && self.area <= FACEPALM_MAX_HAND_AREA
            && self.upper >= FACEPALM_COVER
            && self.middle >= FACEPALM_MIDDLE_COVER
    }
}

/// How much of `face` the hand with `joints` covers; `None` when the face's box has no area or
/// fewer than three joints that are not all in a line were located.
pub fn face_cover(joints: &HandJoints, face: &FaceBox) -> Option<FaceCover> {
    let (width, height) = (face.x2 - face.x1, face.y2 - face.y1);
    if !(width > 0.0 && height > 0.0) {
        return None;
    }
    let points = located(joints);
    let hull = convex_hull(&points);
    let area = polygon_area(&hull);
    if hull.len() < 3 || area <= 0.0 {
        return None;
    }
    let (mut upper, mut middle) = (0, 0);
    for row in 0..GRID_ROWS {
        for column in 0..GRID_COLUMNS {
            let point = Point {
                x: face.x1 + width * (column as f32 + 0.5) / GRID_COLUMNS as f32,
                y: face.y1 + height / 2.0 * (row as f32 + 0.5) / GRID_ROWS as f32,
            };
            if inside(&hull, point) {
                upper += 1;
                middle += usize::from((GRID_COLUMNS / 3..2 * GRID_COLUMNS / 3).contains(&column));
            }
        }
    }
    Some(FaceCover {
        upper: upper as f32 / (GRID_COLUMNS * GRID_ROWS) as f32,
        middle: middle as f32 / (GRID_COLUMNS / 3 * GRID_ROWS) as f32,
        area: area / (width * height),
        joints: points.len(),
    })
}

/// Whether the hand with `joints` covers the upper half of `face`; see the module's description.
pub fn covers_face(joints: &HandJoints, face: &FaceBox) -> bool {
    face_cover(joints, face).is_some_and(|cover| cover.is_facepalm())
}

/// Which face hands are measured against, frame after frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FaceMemory {
    /// The largest face of the last frame in which no hand reached into it, and when that was.
    clear: Option<(FaceBox, u64)>,
}

impl FaceMemory {
    /// The face to measure the `hands` of a frame taken at `now_ms` against. The largest of
    /// `faces` when no located joint lies inside its box, which is also remembered; else the face
    /// remembered within the last [`FACE_MEMORY_MS`], since a hand in front of a face shifts or
    /// hides its box; else the largest of `faces`.
    pub fn anchor(&mut self, faces: &[FaceBox], hands: &[HandJoints], now_ms: u64) -> Option<FaceBox> {
        let largest = faces.iter().copied().max_by(|a, b| box_area(a).total_cmp(&box_area(b)));
        if let Some(face) = largest {
            let reached = hands.iter().flat_map(located).any(|point| in_box(&face, point));
            if !reached {
                self.clear = Some((face, now_ms));
                return Some(face);
            }
        }
        let remembered = self
            .clear
            .filter(|(_, seen_ms)| now_ms >= *seen_ms && now_ms - seen_ms <= FACE_MEMORY_MS);
        remembered.map(|(face, _)| face).or(largest)
    }
}

fn located(joints: &HandJoints) -> Vec<Point> {
    let fingers = [joints.thumb, joints.index, joints.middle, joints.ring, joints.little];
    std::iter::once(joints.wrist)
        .chain(fingers.into_iter().flatten())
        .flatten()
        .filter(|point| point.x.is_finite() && point.y.is_finite())
        .collect()
}

fn box_area(face: &FaceBox) -> f32 {
    let area = (face.x2 - face.x1) * (face.y2 - face.y1);
    if area.is_finite() { area } else { 0.0 }
}

fn in_box(face: &FaceBox, point: Point) -> bool {
    (face.x1..=face.x2).contains(&point.x) && (face.y1..=face.y2).contains(&point.y)
}

/// The z component of `(b - a) x (c - a)`; positive when `a`, `b`, `c` turn one way.
fn cross(a: Point, b: Point, c: Point) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// The convex hull of `points` in one winding order, without collinear points (monotone chain).
fn convex_hull(points: &[Point]) -> Vec<Point> {
    let mut sorted = points.to_vec();
    sorted.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    sorted.dedup();
    if sorted.len() < 3 {
        return sorted;
    }
    let mut hull: Vec<Point> = Vec::with_capacity(sorted.len() * 2);
    for pass in [sorted.clone(), sorted.into_iter().rev().collect()] {
        let start = hull.len();
        for point in pass {
            while hull.len() >= start + 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], point) <= 0.0 {
                hull.pop();
            }
            hull.push(point);
        }
        hull.pop();
    }
    hull
}

/// The shoelace area of the polygon through `points`.
fn polygon_area(points: &[Point]) -> f32 {
    let twice: f32 = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| a.x * b.y - b.x * a.y)
        .sum();
    twice.abs() / 2.0
}

/// Whether `point` lies inside or on the convex `hull`, which turns one way throughout.
fn inside(hull: &[Point], point: Point) -> bool {
    hull.iter()
        .zip(hull.iter().cycle().skip(1))
        .all(|(a, b)| cross(*a, *b, point) >= 0.0)
}
