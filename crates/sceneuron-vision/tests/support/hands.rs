// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Synthetic hands for pose tests: an upright hand with chosen fingers extended, a fist
//! seen from the side, and thumbs in several positions, placed, scaled, and turned in a frame.

use sceneuron_vision::{Chirality, Gesture, HandJoints, Point, hand_pose};

/// Hand joints relative to the wrist, in palm lengths, with `y` growing downward.
pub type Chain = [(f32, f32); 4];

/// Knuckle positions of an upright hand, palm facing the camera.
pub const UPRIGHT_KNUCKLES: [(f32, f32); 4] = [(-0.3, -1.0), (-0.1, -1.0), (0.1, -1.0), (0.3, -0.9)];

pub fn extended((x, y): (f32, f32)) -> Chain {
    [(x, y), (x, y - 0.5), (x, y - 0.8), (x, y - 1.05)]
}

/// A finger folded toward the camera and back into the palm, seen from the front.
pub fn curled((x, y): (f32, f32)) -> Chain {
    [(x, y), (x, y - 0.2), (x, y + 0.05), (x, y + 0.25)]
}

pub const THUMB_FOLDED: Chain = [(-0.4, -0.25), (-0.6, -0.5), (-0.5, -0.75), (-0.3, -0.85)];
pub const THUMB_OUT: Chain = [(-0.4, -0.25), (-0.7, -0.5), (-0.95, -0.7), (-1.15, -0.85)];

/// A synthetic hand: the wrist at `center` in a frame of `aspect`, one palm `size` frame
/// heights long, turned clockwise by `tilt` degrees.
pub struct Model {
    pub center: (f32, f32),
    pub size: f32,
    pub aspect: f32,
    pub tilt: f32,
}

pub const MODEL: Model = Model {
    center: (0.5, 0.5),
    size: 0.1,
    aspect: 1.0,
    tilt: 0.0,
};

impl Model {
    pub fn point(&self, (x, y): (f32, f32)) -> Option<Point> {
        let (sin, cos) = self.tilt.to_radians().sin_cos();
        let (x, y) = (x * cos - y * sin, x * sin + y * cos);
        Some(Point {
            x: self.center.0 + x * self.size / self.aspect,
            y: self.center.1 + y * self.size,
        })
    }

    pub fn chain(&self, chain: Chain) -> [Option<Point>; 4] {
        chain.map(|joint| self.point(joint))
    }

    pub fn hand(&self, thumb: Chain, fingers: [Chain; 4]) -> HandJoints {
        HandJoints {
            wrist: self.point((0.0, 0.0)),
            thumb: self.chain(thumb),
            index: self.chain(fingers[0]),
            middle: self.chain(fingers[1]),
            ring: self.chain(fingers[2]),
            little: self.chain(fingers[3]),
        }
    }

    /// An upright hand whose fingers are extended where `pattern` is true.
    pub fn upright(&self, thumb: Chain, pattern: [bool; 4]) -> HandJoints {
        let fingers = std::array::from_fn(|i| {
            let knuckle = UPRIGHT_KNUCKLES[i];
            if pattern[i] { extended(knuckle) } else { curled(knuckle) }
        });
        self.hand(thumb, fingers)
    }

    /// A fist seen from the side, knuckles to the right, with the given thumb.
    pub fn side_fist(&self, thumb: Chain) -> HandJoints {
        let finger = |(x, y): (f32, f32)| [(x, y), (x + 0.4, y), (x + 0.35, y + 0.2), (x + 0.1, y + 0.2)];
        self.hand(thumb, [(0.9, -0.3), (0.95, -0.1), (0.95, 0.1), (0.9, 0.28)].map(finger))
    }
}

pub const THUMB_UP: Chain = [(0.35, -0.35), (0.55, -0.6), (0.6, -0.9), (0.62, -1.2)];
/// A thumb pointing up past the knuckles of an upright fist, as with a raised forearm.
pub const THUMB_UP_UPRIGHT: Chain = [(-0.4, -0.3), (-0.5, -0.75), (-0.5, -1.2), (-0.5, -1.6)];
pub const THUMB_DOWN: Chain = [(0.35, 0.35), (0.55, 0.6), (0.6, 0.9), (0.62, 1.2)];
pub const THUMB_TUCKED: Chain = [(0.35, -0.35), (0.6, -0.45), (0.85, -0.4), (1.05, -0.25)];

pub fn gesture(joints: &HandJoints, aspect: f32) -> Option<Gesture> {
    hand_pose(joints, Chirality::Right, aspect).gesture
}
