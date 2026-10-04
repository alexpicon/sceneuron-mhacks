// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises extended finger counts on synthetic joints: zero through five fingers, counts
//! for shapes that are no gesture, the thumb up, out, down, or folded, the frame's aspect
//! ratio, and the half-bent fingers and missing or degenerate joints that leave the count
//! unknown.

mod support {
    pub mod hands;
}

use sceneuron_vision::{Chirality, Gesture, HandJoints, Point, hand_pose};
use support::hands::{
    Chain, MODEL, Model, THUMB_DOWN, THUMB_FOLDED, THUMB_OUT, THUMB_TUCKED, THUMB_UP, THUMB_UP_UPRIGHT,
    UPRIGHT_KNUCKLES, gesture,
};

/// Upright hands showing each count from zero to five: the thumb and the fingers extended.
const SHOWN: [(Chain, [bool; 4], u8); 6] = [
    (THUMB_FOLDED, [false; 4], 0),
    (THUMB_FOLDED, [true, false, false, false], 1),
    (THUMB_FOLDED, [true, true, false, false], 2),
    (THUMB_FOLDED, [true, true, true, false], 3),
    (THUMB_FOLDED, [true; 4], 4),
    (THUMB_OUT, [true; 4], 5),
];

fn count(joints: &HandJoints, aspect: f32) -> Option<u8> {
    hand_pose(joints, Chirality::Right, aspect).extended
}

#[test]
fn zero_through_five_fingers() {
    for (thumb, fingers, shown) in SHOWN {
        assert_eq!(count(&MODEL.upright(thumb, fingers), 1.0), Some(shown), "{fingers:?}");
    }
}

#[test]
fn shapes_that_are_no_gesture_still_count() {
    let three = MODEL.upright(THUMB_FOLDED, [true, true, true, false]);
    assert_eq!(gesture(&three, 1.0), None);
    assert_eq!(count(&three, 1.0), Some(3));
    let rock = MODEL.upright(THUMB_FOLDED, [true, false, false, true]);
    assert_eq!(count(&rock, 1.0), Some(2));
    let little = MODEL.upright(THUMB_FOLDED, [false, false, false, true]);
    assert_eq!(count(&little, 1.0), Some(1));
}

#[test]
fn the_thumb_counts_when_up_or_out_but_not_when_folded() {
    assert_eq!(count(&MODEL.side_fist(THUMB_UP), 1.0), Some(1), "thumbs up");
    let raised_thumb = MODEL.upright(THUMB_UP_UPRIGHT, [false; 4]);
    assert_eq!(count(&raised_thumb, 1.0), Some(1), "thumbs up on a raised forearm");
    let sideways = MODEL.side_fist(THUMB_OUT);
    assert_eq!(gesture(&sideways, 1.0), None);
    assert_eq!(count(&sideways, 1.0), Some(1), "thumb out sideways");
    let peace = |thumb: Chain| MODEL.upright(thumb, [true, true, false, false]);
    assert_eq!(gesture(&peace(THUMB_OUT), 1.0), Some(Gesture::Peace));
    assert_eq!(count(&peace(THUMB_OUT), 1.0), Some(3));
    assert_eq!(gesture(&peace(THUMB_FOLDED), 1.0), Some(Gesture::Peace));
    assert_eq!(count(&peace(THUMB_FOLDED), 1.0), Some(2));
    let tucked = MODEL.side_fist(THUMB_TUCKED);
    assert_eq!(gesture(&tucked, 1.0), Some(Gesture::Fist));
    assert_eq!(count(&tucked, 1.0), Some(0));
}

#[test]
fn thumbs_down_counts_its_thumb() {
    let hand = MODEL.side_fist(THUMB_DOWN);
    assert_eq!(gesture(&hand, 1.0), Some(Gesture::ThumbsDown));
    assert_eq!(count(&hand, 1.0), Some(1));
}

#[test]
fn counts_account_for_the_aspect_ratio() {
    let wide = Model {
        aspect: 16.0 / 9.0,
        ..MODEL
    };
    for (thumb, fingers, shown) in SHOWN {
        assert_eq!(
            count(&wide.upright(thumb, fingers), wide.aspect),
            Some(shown),
            "{fingers:?}"
        );
    }
}

#[test]
fn a_half_bent_finger_makes_the_count_unknown() {
    let mut hand = MODEL.upright(THUMB_FOLDED, [true; 4]);
    let (x, y) = UPRIGHT_KNUCKLES[1];
    hand.middle = MODEL.chain([(x, y), (x, y - 0.5), (x - 0.15, y - 0.65), (x - 0.25, y - 0.55)]);
    assert_eq!(count(&hand, 1.0), None);
}

#[test]
fn a_missing_joint_makes_the_count_unknown() {
    let palm = MODEL.upright(THUMB_OUT, [true; 4]);
    let without = |remove: fn(&mut HandJoints)| {
        let mut hand = palm;
        remove(&mut hand);
        count(&hand, 1.0)
    };
    assert_eq!(without(|_| {}), Some(5));
    assert_eq!(without(|hand| hand.wrist = None), None, "wrist");
    assert_eq!(without(|hand| hand.ring[3] = None), None, "ring tip");
    assert_eq!(without(|hand| hand.little[1] = None), None, "little middle joint");
    assert_eq!(without(|hand| hand.thumb[0] = None), None, "thumb base");
    assert_eq!(
        without(|hand| hand.index[0] = None),
        None,
        "index knuckle the thumb reaches past"
    );
    assert_eq!(
        without(|hand| hand.middle[0] = None),
        None,
        "middle knuckle that sets the palm length"
    );
    assert_eq!(count(&HandJoints::default(), 1.0), None);
}

#[test]
fn an_occluded_thumb_leaves_a_fist_uncounted() {
    let mut fist = MODEL.side_fist(THUMB_UP);
    fist.thumb[3] = None;
    assert_eq!(gesture(&fist, 1.0), Some(Gesture::Fist));
    assert_eq!(count(&fist, 1.0), None);
}

#[test]
fn degenerate_joints_give_no_count() {
    let point = MODEL.point((0.0, 0.0));
    let collapsed = HandJoints {
        wrist: point,
        thumb: [point; 4],
        index: [point; 4],
        middle: [point; 4],
        ring: [point; 4],
        little: [point; 4],
    };
    assert_eq!(count(&collapsed, 1.0), None);
    let not_a_number = |set: fn(&mut HandJoints, Option<Point>)| {
        let mut hand = MODEL.upright(THUMB_OUT, [true; 4]);
        set(&mut hand, Some(Point { x: f32::NAN, y: 0.1 }));
        count(&hand, 1.0)
    };
    assert_eq!(not_a_number(|hand, point| hand.index[3] = point), None, "index tip");
    assert_eq!(not_a_number(|hand, point| hand.thumb[3] = point), None, "thumb tip");
    assert_eq!(not_a_number(|hand, point| hand.index[0] = point), None, "index knuckle");
    assert_eq!(
        not_a_number(|hand, point| hand.middle[0] = point),
        None,
        "middle knuckle"
    );
}

#[test]
fn a_hand_pinching_something_gives_no_count() {
    let two = MODEL.upright(THUMB_OUT, [true, false, false, false]);
    assert_eq!(count(&two, 1.0), Some(2), "thumb and index apart");
    let mut pinch = two;
    pinch.thumb[3] = pinch.index[3];
    assert_eq!(
        count(&pinch, 1.0),
        None,
        "thumb tip on the index tip, as when holding up a lime"
    );
    let mut fist = MODEL.side_fist(THUMB_TUCKED);
    fist.thumb[3] = fist.index[3];
    assert_eq!(count(&fist, 1.0), Some(0), "a thumb resting on a curled index");
}
