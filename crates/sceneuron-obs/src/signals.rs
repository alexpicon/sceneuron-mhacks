// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Turns each perception into the signals that raise cards. It needs no AI key, and nothing it
//! sees leaves the computer.

use sceneuron_core::{HandSign, SignalSummary};
use sceneuron_vision::{Chirality, Gesture, Perception};

/// Summarizes one perception for [`Engine::observe_signals`](sceneuron_core::Engine::observe_signals).
///
/// A hand showing thumbs up or thumbs down gives only its sign: no finger count, so a thumbs up
/// never reads as one finger, and no raised hand, since a thumbs up held toward the camera is
/// usually raised. The other hands' counts are added up; the sum is `None` when no other hand
/// is in view or any of their counts is unknown. Any arm up counts as `arms_up`.
///
/// A hand over the face, a facepalm, gives nothing: it is neither a raised hand, nor a sign, nor
/// a count. Nor does the arm on its side count as up, since its wrist is by the face, above the
/// nose or the shoulder; when the hand's side is unknown, neither arm counts.
pub fn signal_summary(perception: &Perception) -> SignalSummary {
    let mut signs = Vec::new();
    let mut counted = false;
    let mut raised = 0_usize;
    let mut fingers = Some(0_u8);
    let mut covering = Vec::new();
    for hand in &perception.hands {
        if hand.over_face {
            covering.push(hand.chirality);
            continue;
        }
        if let Some(sign) = hand.gesture.map(sign)
            && !signs.contains(&sign)
        {
            signs.push(sign);
        }
        if matches!(hand.gesture, Some(Gesture::ThumbsUp | Gesture::ThumbsDown)) {
            continue;
        }
        counted = true;
        raised += usize::from(hand.raised);
        fingers = fingers.zip(hand.extended).map(|(sum, count)| sum.saturating_add(count));
    }
    let counts = |side: Chirality| !covering.contains(&side) && !covering.contains(&Chirality::Unknown);
    SignalSummary {
        hands_raised: u8::try_from(raised).unwrap_or(u8::MAX),
        arms_up: perception.bodies.iter().any(|body| {
            (body.left_arm_up && counts(Chirality::Left)) || (body.right_arm_up && counts(Chirality::Right))
        }),
        fingers: fingers.filter(|_| counted),
        signs,
    }
}

/// The card signal's name for a hand shape.
pub(crate) fn sign(gesture: Gesture) -> HandSign {
    match gesture {
        Gesture::ThumbsUp => HandSign::ThumbsUp,
        Gesture::ThumbsDown => HandSign::ThumbsDown,
        Gesture::OpenPalm => HandSign::OpenPalm,
        Gesture::Peace => HandSign::Peace,
        Gesture::Pointing => HandSign::Pointing,
        Gesture::Fist => HandSign::Fist,
    }
}
