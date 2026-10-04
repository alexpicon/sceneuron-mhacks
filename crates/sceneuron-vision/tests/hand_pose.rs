// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises hand pose geometry on synthetic joints: each gesture, shapes that are no
//! gesture, missing and degenerate joints, the frame's aspect ratio, and raised hands
//! against the raised-hand line and the tilt limit.

mod support {
    pub mod hands;
}

use sceneuron_vision::{
    Chirality, Gesture, HandJoints, Point, RAISED_HAND_LINE, RAISED_HAND_MAX_TILT_DEGREES, hand_pose,
};
use support::hands::{
    MODEL, Model, THUMB_DOWN, THUMB_FOLDED, THUMB_OUT, THUMB_TUCKED, THUMB_UP, THUMB_UP_UPRIGHT, UPRIGHT_KNUCKLES,
    gesture,
};

mod gestures {
    use super::*;

    #[test]
    fn open_palm_with_any_thumb() {
        assert_eq!(
            gesture(&MODEL.upright(THUMB_OUT, [true; 4]), 1.0),
            Some(Gesture::OpenPalm)
        );
        assert_eq!(
            gesture(&MODEL.upright(THUMB_FOLDED, [true; 4]), 1.0),
            Some(Gesture::OpenPalm)
        );
    }

    #[test]
    fn peace_sign() {
        let hand = MODEL.upright(THUMB_FOLDED, [true, true, false, false]);
        assert_eq!(gesture(&hand, 1.0), Some(Gesture::Peace));
    }

    #[test]
    fn pointing() {
        let hand = MODEL.upright(THUMB_FOLDED, [true, false, false, false]);
        assert_eq!(gesture(&hand, 1.0), Some(Gesture::Pointing));
    }

    #[test]
    fn thumbs_up_and_fist() {
        assert_eq!(gesture(&MODEL.side_fist(THUMB_UP), 1.0), Some(Gesture::ThumbsUp));
        assert_eq!(gesture(&MODEL.side_fist(THUMB_TUCKED), 1.0), Some(Gesture::Fist));
        assert_eq!(
            gesture(&MODEL.upright(THUMB_FOLDED, [false; 4]), 1.0),
            Some(Gesture::Fist)
        );
    }

    #[test]
    fn thumbs_up_on_a_raised_forearm() {
        let hand = MODEL.upright(THUMB_UP_UPRIGHT, [false; 4]);
        assert_eq!(gesture(&hand, 1.0), Some(Gesture::ThumbsUp));
    }

    #[test]
    fn thumbs_down() {
        assert_eq!(gesture(&MODEL.side_fist(THUMB_DOWN), 1.0), Some(Gesture::ThumbsDown));
        let lowered_forearm = Model { tilt: 180.0, ..MODEL };
        let hand = lowered_forearm.upright(THUMB_UP_UPRIGHT, [false; 4]);
        assert_eq!(gesture(&hand, 1.0), Some(Gesture::ThumbsDown));
    }

    #[test]
    fn a_fist_with_an_occluded_thumb_is_a_fist() {
        let mut hand = MODEL.side_fist(THUMB_UP);
        hand.thumb[3] = None;
        assert_eq!(gesture(&hand, 1.0), Some(Gesture::Fist));
    }

    #[test]
    fn other_shapes_are_no_gesture() {
        assert_eq!(gesture(&MODEL.side_fist(THUMB_OUT), 1.0), None, "thumb out sideways");
        let short_down = MODEL.side_fist([(0.35, -0.35), (0.4, -0.15), (0.45, 0.05), (0.5, 0.25)]);
        assert_eq!(
            gesture(&short_down, 1.0),
            None,
            "thumb down but above the little finger"
        );
        let rock = MODEL.upright(THUMB_FOLDED, [true, false, false, true]);
        assert_eq!(gesture(&rock, 1.0), None);
        let three = MODEL.upright(THUMB_FOLDED, [true, true, true, false]);
        assert_eq!(gesture(&three, 1.0), None);
    }

    #[test]
    fn a_half_bent_finger_is_no_gesture() {
        let mut hand = MODEL.upright(THUMB_FOLDED, [true; 4]);
        let (x, y) = UPRIGHT_KNUCKLES[1];
        hand.middle = MODEL.chain([(x, y), (x, y - 0.5), (x - 0.15, y - 0.65), (x - 0.25, y - 0.55)]);
        assert_eq!(gesture(&hand, 1.0), None);
    }

    #[test]
    fn missing_joints_give_no_gesture() {
        let mut no_wrist = MODEL.upright(THUMB_OUT, [true; 4]);
        no_wrist.wrist = None;
        assert_eq!(gesture(&no_wrist, 1.0), None);
        let mut no_tip = MODEL.upright(THUMB_OUT, [true; 4]);
        no_tip.ring[3] = None;
        assert_eq!(gesture(&no_tip, 1.0), None);
        assert_eq!(gesture(&HandJoints::default(), 1.0), None);
    }

    #[test]
    fn degenerate_joints_give_no_gesture() {
        let point = MODEL.point((0.0, 0.0));
        let collapsed = HandJoints {
            wrist: point,
            thumb: [point; 4],
            index: [point; 4],
            middle: [point; 4],
            ring: [point; 4],
            little: [point; 4],
        };
        assert_eq!(gesture(&collapsed, 1.0), None);
        let mut nan = MODEL.upright(THUMB_OUT, [true; 4]);
        nan.index[3] = Some(Point { x: f32::NAN, y: 0.1 });
        assert_eq!(gesture(&nan, 1.0), None);
    }

    #[test]
    fn distances_account_for_the_aspect_ratio() {
        let wide = Model {
            aspect: 16.0 / 9.0,
            ..MODEL
        };
        let peace = wide.upright(THUMB_FOLDED, [true, true, false, false]);
        assert_eq!(gesture(&peace, wide.aspect), Some(Gesture::Peace));
        assert_eq!(gesture(&wide.side_fist(THUMB_UP), wide.aspect), Some(Gesture::ThumbsUp));
        assert_eq!(
            gesture(&wide.side_fist(THUMB_DOWN), wide.aspect),
            Some(Gesture::ThumbsDown)
        );
    }

    #[test]
    fn an_invalid_aspect_ratio_counts_as_square() {
        let palm = MODEL.upright(THUMB_OUT, [true; 4]);
        for aspect in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(gesture(&palm, aspect), Some(Gesture::OpenPalm), "aspect {aspect}");
        }
    }
}

mod raised {
    use super::*;

    fn at(y: f32) -> HandJoints {
        Model {
            center: (0.5, y),
            ..MODEL
        }
        .upright(THUMB_OUT, [true; 4])
    }

    #[test]
    fn a_hand_wholly_above_the_line_is_raised() {
        let hand = hand_pose(&at(0.5), Chirality::Left, 1.0);
        assert!(hand.raised);
        assert_eq!(hand.chirality, Chirality::Left);
        assert_eq!(hand.gesture, Some(Gesture::OpenPalm));
    }

    #[test]
    fn a_hand_reaching_below_the_line_is_not_raised() {
        let wrist_on_line = at(RAISED_HAND_LINE);
        assert!(!hand_pose(&wrist_on_line, Chirality::Right, 1.0).raised);
        assert!(!hand_pose(&at(0.9), Chirality::Right, 1.0).raised);
        let wrist_just_above = at(RAISED_HAND_LINE - 0.01);
        assert!(hand_pose(&wrist_just_above, Chirality::Right, 1.0).raised);
    }

    #[test]
    fn a_hand_must_point_up_within_the_tilt_limit() {
        let tilted = |tilt: f32| {
            let model = Model {
                center: (0.5, 0.5),
                tilt,
                ..MODEL
            };
            hand_pose(&model.upright(THUMB_OUT, [true; 4]), Chirality::Right, 1.0).raised
        };
        let limit = RAISED_HAND_MAX_TILT_DEGREES;
        assert!(tilted(limit - 2.0) && tilted(2.0 - limit));
        assert!(!tilted(limit + 2.0) && !tilted(-limit - 2.0));
        assert!(!tilted(180.0), "pointing down");
    }

    #[test]
    fn a_hand_resting_sideways_is_not_raised() {
        let high = Model {
            center: (0.5, 0.3),
            ..MODEL
        };
        let hand = hand_pose(&high.side_fist(THUMB_UP), Chirality::Left, 1.0);
        assert!(!hand.raised);
        assert_eq!(hand.gesture, Some(Gesture::ThumbsUp));
        let raised_thumb = hand_pose(&high.upright(THUMB_UP_UPRIGHT, [false; 4]), Chirality::Left, 1.0);
        assert!(raised_thumb.raised);
        assert_eq!(raised_thumb.gesture, Some(Gesture::ThumbsUp));
    }

    #[test]
    fn a_hand_without_a_wrist_or_knuckles_is_not_raised() {
        let mut hand = at(0.3);
        hand.wrist = None;
        assert!(!hand_pose(&hand, Chirality::Unknown, 1.0).raised);
        let mut hand = at(0.3);
        for finger in [&mut hand.index, &mut hand.middle, &mut hand.ring, &mut hand.little] {
            finger[0] = None;
        }
        assert!(!hand_pose(&hand, Chirality::Unknown, 1.0).raised);
        assert!(!hand_pose(&HandJoints::default(), Chirality::Unknown, 1.0).raised);
    }
}
