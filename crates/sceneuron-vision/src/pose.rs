// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Hand and body pose geometry: turns joint positions into raised hands, hand gestures,
//! extended finger counts, and raised arms. The functions are pure so that any pose model
//! can share them.
//!
//! Joints use normalized frame coordinates with the origin at the top left. Distances are
//! measured after scaling `x` by the frame's aspect ratio, so they are isotropic.

use crate::hand_shape::{extended, gesture, shape};
use crate::perception::{Body, Chirality, Hand};

/// A point in normalized frame coordinates, origin at the top left, `y` growing downward.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// The 21 joints of one hand; `None` for joints the model did not locate confidently.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HandJoints {
    pub wrist: Option<Point>,
    /// From the base outward: carpometacarpal, metacarpophalangeal, interphalangeal, tip.
    pub thumb: [Option<Point>; 4],
    /// From the knuckle outward: metacarpophalangeal (knuckle), proximal and distal
    /// interphalangeal, tip. The other fingers use the same order.
    pub index: [Option<Point>; 4],
    pub middle: [Option<Point>; 4],
    pub ring: [Option<Point>; 4],
    pub little: [Option<Point>; 4],
}

/// The body joints arm poses need; `None` for joints the model did not locate confidently.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BodyJoints {
    pub nose: Option<Point>,
    pub left_shoulder: Option<Point>,
    pub right_shoulder: Option<Point>,
    pub left_wrist: Option<Point>,
    pub right_wrist: Option<Point>,
}

/// A raised hand has every located joint above this fraction of the frame height.
pub const RAISED_HAND_LINE: f32 = 0.7;
/// A raised hand points up: the direction from its wrist to the middle of its knuckles is
/// at most this many degrees from vertical, so a hand resting on something is not raised.
pub const RAISED_HAND_MAX_TILT_DEGREES: f32 = 50.0;
/// Shorter distances, in frame heights, are treated as degenerate.
pub(crate) const EPSILON: f32 = 1e-4;

/// Describes one hand from its joints. `aspect` is the frame's width divided by its height. It
/// leaves [`Hand::over_face`] unset, since that takes the face (see
/// [`covers_face`](crate::covers_face)).
pub fn hand_pose(joints: &HandJoints, chirality: Chirality, aspect: f32) -> Hand {
    let space = Space::new(aspect);
    let shape = shape(joints, space);
    Hand {
        chirality,
        raised: raised(joints, space),
        gesture: shape.and_then(gesture),
        extended: shape.and_then(extended),
        over_face: false,
    }
}

/// Describes one body's arms from its joints.
pub fn body_pose(joints: &BodyJoints) -> Body {
    let up = |wrist: Option<Point>, shoulder: Option<Point>| match (wrist, joints.nose.or(shoulder)) {
        (Some(wrist), Some(reference)) => wrist.y < reference.y,
        _ => false,
    };
    let left_arm_up = up(joints.left_wrist, joints.left_shoulder);
    let right_arm_up = up(joints.right_wrist, joints.right_shoulder);
    Body {
        left_arm_up,
        right_arm_up,
        arms_up: left_arm_up && right_arm_up,
    }
}

fn raised(joints: &HandJoints, space: Space) -> bool {
    let Some(wrist) = joints.wrist else {
        return false;
    };
    let knuckles: Vec<Point> = [joints.index, joints.middle, joints.ring, joints.little]
        .iter()
        .filter_map(|finger| finger[0])
        .collect();
    if knuckles.is_empty() {
        return false;
    }
    let count = knuckles.len() as f32;
    let center = Point {
        x: knuckles.iter().map(|point| point.x).sum::<f32>() / count,
        y: knuckles.iter().map(|point| point.y).sum::<f32>() / count,
    };
    let (dx, dy) = space.vector(wrist, center);
    let upright = -dy > EPSILON && -dy >= dx.abs() / RAISED_HAND_MAX_TILT_DEGREES.to_radians().tan();
    upright && all_points(joints).all(|point| point.y < RAISED_HAND_LINE)
}

fn all_points(joints: &HandJoints) -> impl Iterator<Item = Point> + '_ {
    let fingers = [joints.thumb, joints.index, joints.middle, joints.ring, joints.little];
    std::iter::once(joints.wrist)
        .chain(fingers.into_iter().flatten())
        .flatten()
}

/// Measures in frame heights by stretching `x` by the aspect ratio.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Space {
    aspect: f32,
}

impl Space {
    pub(crate) fn new(aspect: f32) -> Space {
        let aspect = if aspect.is_finite() && aspect > 0.0 {
            aspect
        } else {
            1.0
        };
        Space { aspect }
    }

    pub(crate) fn vector(self, from: Point, to: Point) -> (f32, f32) {
        ((to.x - from.x) * self.aspect, to.y - from.y)
    }

    pub(crate) fn distance(self, a: Point, b: Point) -> f32 {
        let (dx, dy) = self.vector(a, b);
        dx.hypot(dy)
    }
}
