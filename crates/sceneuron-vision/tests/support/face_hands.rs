// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A synthetic hand for facepalm tests, in a 1920x1080 frame, and the face it is measured against,
//! whose box spans 384 by 378 pixels.

use sceneuron_vision::{FaceBox, HandJoints, Point};

const WIDTH: f32 = 1920.0;
const HEIGHT: f32 = 1080.0;

/// The face: pixels 768 to 1152 across and 270 to 648 down.
pub const FACE: FaceBox = FaceBox {
    x1: 0.4,
    y1: 0.25,
    x2: 0.6,
    y2: 0.6,
    score: 0.97,
};

/// Knuckles relative to the wrist in palm lengths, along the hand and across it toward the little
/// finger, with each finger's length and how far it fans out, in degrees.
const FINGERS: [((f32, f32), f32, f32); 4] = [
    ((0.97, -0.42), 0.88, -8.0),
    ((1.0, -0.14), 0.98, -2.0),
    ((0.95, 0.14), 0.92, 4.0),
    ((0.85, 0.40), 0.72, 10.0),
];

/// A hand with its wrist at `wrist` in pixels, pointing `angle` degrees clockwise from straight
/// up, its palm `palm` pixels from the wrist to the middle knuckle; its four fingers extended
/// where `pattern` is true and curled into the palm elsewhere, and its thumb reaching out.
pub fn hand(wrist: (f32, f32), angle: f32, palm: f32, pattern: [bool; 4]) -> HandJoints {
    let turn = |degrees: f32| {
        let (sin, cos) = (angle + degrees).to_radians().sin_cos();
        (sin, -cos)
    };
    let across = turn(90.0);
    let at = |(along, side): (f32, f32)| {
        let up = turn(0.0);
        let x = wrist.0 + (up.0 * along + across.0 * side) * palm;
        let y = wrist.1 + (up.1 * along + across.1 * side) * palm;
        Point {
            x: x / WIDTH,
            y: y / HEIGHT,
        }
    };
    let chain = |knuckle: (f32, f32), length: f32, fan: f32, extended: bool| {
        let (dx, dy) = turn(fan);
        // Rotated back into the hand's own axes: along the hand, and across it.
        let up = turn(0.0);
        let along = dx * up.0 + dy * up.1;
        let side = dx * across.0 + dy * across.1;
        let reach = if extended {
            [0.45, 0.75, 1.0]
        } else {
            [0.35, 0.15, -0.05]
        };
        let joint = |share: f32| at((knuckle.0 + along * share * length, knuckle.1 + side * share * length));
        [
            Some(at(knuckle)),
            Some(joint(reach[0])),
            Some(joint(reach[1])),
            Some(joint(reach[2])),
        ]
    };
    let thumb_base = (0.25, -0.35);
    HandJoints {
        wrist: Some(at((0.0, 0.0))),
        thumb: chain(thumb_base, 0.9, -55.0, true),
        index: chain(FINGERS[0].0, FINGERS[0].1, FINGERS[0].2, pattern[0]),
        middle: chain(FINGERS[1].0, FINGERS[1].1, FINGERS[1].2, pattern[1]),
        ring: chain(FINGERS[2].0, FINGERS[2].1, FINGERS[2].2, pattern[2]),
        little: chain(FINGERS[3].0, FINGERS[3].1, FINGERS[3].2, pattern[3]),
    }
}
