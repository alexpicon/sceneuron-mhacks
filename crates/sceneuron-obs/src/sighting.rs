// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Turns each perception into what the live description says about it. It needs no AI key, and
//! nothing it sees leaves the computer.

use sceneuron_core::{HandSide, SeenHand, Sighting};
use sceneuron_vision::{Chirality, MOUTH_OPEN, Perception};

use crate::signals::sign;

/// Summarizes one perception for [`Engine::observe_sighting`](sceneuron_core::Engine::observe_sighting).
///
/// People are the faces or the bodies found, whichever are more. A hand over the face is the
/// facepalm, not a hand of its own. The mouth is open from [`MOUTH_OPEN`] on, as for the
/// vision model's hints. Labels that name an animal already listed are left out.
pub fn sighting(perception: &Perception) -> Sighting {
    let hands = perception
        .hands
        .iter()
        .filter(|hand| !hand.over_face)
        .map(|hand| SeenHand {
            side: match hand.chirality {
                Chirality::Left => HandSide::Left,
                Chirality::Right => HandSide::Right,
                Chirality::Unknown => HandSide::Unknown,
            },
            raised: hand.raised,
            sign: hand.gesture.map(sign),
            fingers: hand.extended,
        })
        .collect();
    let arms_up = perception
        .bodies
        .iter()
        .map(|body| u8::from(body.left_arm_up) + u8::from(body.right_arm_up))
        .max()
        .unwrap_or(0);
    let animals: Vec<String> = perception.animals.iter().map(|animal| animal.name.clone()).collect();
    let things = perception
        .labels
        .iter()
        .filter(|label| !animals.contains(&label.name))
        .map(|label| label.name.clone())
        .collect();
    Sighting {
        people: perception.faces.len().max(perception.bodies.len()),
        hands,
        facepalm: perception.facepalm(),
        arms_up,
        mouth_open: perception.mouth_opening.is_some_and(|opening| opening >= MOUTH_OPEN),
        animals,
        things,
        text: perception.text.iter().map(|line| line.text.clone()).collect(),
    }
}
