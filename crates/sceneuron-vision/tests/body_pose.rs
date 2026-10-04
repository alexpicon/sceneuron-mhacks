// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises body pose geometry: raised arms judged against the nose, or against the
//! shoulders when the nose is not visible, and missing joints.

use sceneuron_vision::{Body, BodyJoints, Point, body_pose};

fn point(x: f32, y: f32) -> Option<Point> {
    Some(Point { x, y })
}

/// A person with the nose at 0.3 and shoulders at 0.6; wrists at the given heights.
fn person(left_wrist: f32, right_wrist: f32) -> BodyJoints {
    BodyJoints {
        nose: point(0.5, 0.3),
        left_shoulder: point(0.65, 0.6),
        right_shoulder: point(0.35, 0.6),
        left_wrist: point(0.8, left_wrist),
        right_wrist: point(0.2, right_wrist),
    }
}

#[test]
fn both_wrists_above_the_nose_are_arms_up() {
    let body = body_pose(&person(0.1, 0.2));
    assert_eq!(
        body,
        Body {
            left_arm_up: true,
            right_arm_up: true,
            arms_up: true
        }
    );
}

#[test]
fn one_wrist_up_is_one_arm_up() {
    let body = body_pose(&person(0.1, 0.8));
    assert!(body.left_arm_up && !body.right_arm_up && !body.arms_up, "{body:?}");
    let body = body_pose(&person(0.8, 0.1));
    assert!(!body.left_arm_up && body.right_arm_up && !body.arms_up, "{body:?}");
}

#[test]
fn wrists_between_shoulders_and_nose_are_not_up_when_the_nose_is_visible() {
    assert_eq!(body_pose(&person(0.45, 0.45)), Body::default());
}

#[test]
fn shoulders_are_the_reference_without_a_nose() {
    let joints = BodyJoints {
        nose: None,
        ..person(0.45, 0.7)
    };
    let body = body_pose(&joints);
    assert!(body.left_arm_up && !body.right_arm_up && !body.arms_up, "{body:?}");
}

#[test]
fn missing_joints_are_not_up() {
    let joints = BodyJoints {
        left_wrist: None,
        ..person(0.1, 0.1)
    };
    let body = body_pose(&joints);
    assert!(!body.left_arm_up && body.right_arm_up && !body.arms_up, "{body:?}");
    let headless = BodyJoints {
        nose: None,
        left_shoulder: None,
        ..person(0.1, 0.1)
    };
    assert!(!body_pose(&headless).left_arm_up);
    assert_eq!(body_pose(&BodyJoints::default()), Body::default());
}
