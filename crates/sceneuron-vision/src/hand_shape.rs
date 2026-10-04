// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Hand shape classification: whether each finger is extended, curled, or half bent, where
//! the thumb points, and the gestures and finger counts those shapes make.

use crate::perception::Gesture;
use crate::pose::{EPSILON, HandJoints, Point, Space};

/// A finger is extended when its tip is at least this many times as far from the wrist as
/// its proximal interphalangeal joint; a straight finger measures about 1.4.
const EXTENDED_RATIO: f32 = 1.2;
/// A finger is curled when its tip is at most this many times as far from the wrist as its
/// proximal interphalangeal joint; a fist measures about 0.65.
const CURLED_RATIO: f32 = 0.95;
/// A thumb is straight when the distance from its base to its tip is at least this fraction
/// of the length along its joints.
const THUMB_STRAIGHTNESS: f32 = 0.85;
/// An extended thumb's tip is at least this many palm lengths (wrist to middle knuckle)
/// from the index knuckle; a thumb folded over the fingers stays closer.
const THUMB_REACH: f32 = 0.55;
/// A hand pinches, as when it holds up a small object, when its thumb and index tips are at
/// most this many palm lengths apart; fingers counted for a vote stay well apart.
const PINCH_REACH: f32 = 0.45;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Finger {
    Extended,
    Curled,
    Bent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Thumb {
    Up,
    Down,
    Out,
    Folded,
}

/// The four fingers from the index outward, and the thumb when its joints were located.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Shape {
    fingers: [Finger; 4],
    thumb: Option<Thumb>,
    pinch: bool,
}

/// `None` when the wrist or a joint that one of the four fingers needs is missing.
pub(crate) fn shape(joints: &HandJoints, space: Space) -> Option<Shape> {
    let wrist = joints.wrist?;
    let finger = |chain: [Option<Point>; 4]| finger(space, wrist, chain);
    Some(Shape {
        fingers: [
            finger(joints.index)?,
            finger(joints.middle)?,
            finger(joints.ring)?,
            finger(joints.little)?,
        ],
        thumb: thumb(joints, space, wrist),
        pinch: pinch(joints, space, wrist),
    })
}

pub(crate) fn gesture(Shape { fingers, thumb, .. }: Shape) -> Option<Gesture> {
    use Finger::{Curled as C, Extended as E};
    match fingers {
        [E, E, E, E] => Some(Gesture::OpenPalm),
        [E, E, C, C] => Some(Gesture::Peace),
        [E, C, C, C] => Some(Gesture::Pointing),
        // An occluded thumb is common in a fist, so an unknown thumb counts as folded.
        [C, C, C, C] => match thumb.unwrap_or(Thumb::Folded) {
            Thumb::Up => Some(Gesture::ThumbsUp),
            Thumb::Down => Some(Gesture::ThumbsDown),
            Thumb::Folded => Some(Gesture::Fist),
            Thumb::Out => None,
        },
        _ => None,
    }
}

/// Unlike a gesture, a count treats an unknown thumb as unknown: guessing it folded would
/// turn a hidden thumb into a confident count that is one short. A hand whose thumb meets an
/// index finger that is not curled holds something rather than counting, so it gives no
/// count; in a fist the thumb may rest on the curled index and the count stays 0.
pub(crate) fn extended(Shape { fingers, thumb, pinch }: Shape) -> Option<u8> {
    if pinch && fingers[0] != Finger::Curled {
        return None;
    }
    let thumb = match thumb? {
        Thumb::Up | Thumb::Down | Thumb::Out => 1,
        Thumb::Folded => 0,
    };
    fingers.into_iter().try_fold(thumb, |count, finger| match finger {
        Finger::Extended => Some(count + 1),
        Finger::Curled => Some(count),
        Finger::Bent => None,
    })
}

fn finger(space: Space, wrist: Point, [_, pip, _, tip]: [Option<Point>; 4]) -> Option<Finger> {
    let near = space.distance(wrist, pip?);
    if near < EPSILON {
        return None;
    }
    let ratio = space.distance(wrist, tip?) / near;
    let state = if ratio >= EXTENDED_RATIO {
        Finger::Extended
    } else if ratio <= CURLED_RATIO {
        Finger::Curled
    } else {
        Finger::Bent
    };
    Some(state)
}

/// Whether the thumb and index tips nearly meet; false when a joint it needs is missing.
fn pinch(joints: &HandJoints, space: Space, wrist: Point) -> bool {
    let (Some(thumb_tip), Some(index_tip), Some(middle_knuckle)) = (joints.thumb[3], joints.index[3], joints.middle[0])
    else {
        return false;
    };
    let palm = space.distance(wrist, middle_knuckle);
    palm >= EPSILON && space.distance(thumb_tip, index_tip) / palm <= PINCH_REACH
}

/// `None` when a joint the thumb needs is missing or not a finite point, so a bad joint never
/// reads as a folded thumb.
fn thumb(joints: &HandJoints, space: Space, wrist: Point) -> Option<Thumb> {
    let [Some(base), Some(mp), Some(ip), Some(tip)] = joints.thumb else {
        return None;
    };
    let index_knuckle = joints.index[0]?;
    let middle_knuckle = joints.middle[0]?;
    let used = [wrist, base, mp, ip, tip, index_knuckle, middle_knuckle];
    if !used.iter().all(|point| point.x.is_finite() && point.y.is_finite()) {
        return None;
    }
    let palm = space.distance(wrist, middle_knuckle);
    let path = space.distance(base, mp) + space.distance(mp, ip) + space.distance(ip, tip);
    if palm < EPSILON || path < EPSILON {
        return None;
    }
    let straight = space.distance(base, tip) / path >= THUMB_STRAIGHTNESS;
    let reaches = space.distance(tip, index_knuckle) / palm >= THUMB_REACH;
    if !(straight && reaches) {
        return Some(Thumb::Folded);
    }
    let (dx, dy) = space.vector(mp, tip);
    let others = [joints.index, joints.middle, joints.ring, joints.little];
    let others = || others.into_iter().flatten().flatten().chain([wrist]);
    Some(if -dy >= dx.abs() && others().all(|point| tip.y < point.y) {
        Thumb::Up
    } else if dy >= dx.abs() && others().all(|point| tip.y > point.y) {
        Thumb::Down
    } else {
        Thumb::Out
    })
}
