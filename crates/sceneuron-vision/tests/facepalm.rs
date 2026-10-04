// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the facepalm geometry on synthetic hands in a 1920x1080 frame, against a face whose
//! box spans 384 by 378 pixels: hands over the eyes and the forehead are facepalms; a hand on the
//! chin, under it with a finger up the cheek, over the mouth, scratching an ear, holding a cheek,
//! or propping up a temple is not, and neither is a hand near the camera or one with few joints
//! located. Also how a perception reports it, and its hint.

mod support {
    pub mod face_hands;
}

use sceneuron_vision::{
    FACEPALM_COVER, FACEPALM_FACT, FACEPALM_MIDDLE_COVER, FaceBox, FaceCover, Hand, HandJoints, Perception, Point,
    covers_face, face_cover, facts,
};
use support::face_hands::{FACE, hand};

const OPEN: [bool; 4] = [true; 4];
const FIST: [bool; 4] = [false; 4];

fn cover(joints: &HandJoints) -> FaceCover {
    face_cover(joints, &FACE).expect("a measurable hand")
}

mod facepalms {
    use super::*;

    #[test]
    fn a_hand_over_the_eyes_with_the_fingers_up_the_forehead() {
        let picard = hand((1040.0, 590.0), -25.0, 180.0, OPEN);
        let measured = cover(&picard);
        assert!(measured.is_facepalm(), "{measured:?}");
        assert!(measured.upper >= FACEPALM_COVER + 0.1, "a clear margin: {measured:?}");
        assert!(measured.middle >= FACEPALM_MIDDLE_COVER + 0.3, "{measured:?}");
        assert!(covers_face(&picard, &FACE));
    }

    #[test]
    fn a_hand_across_both_eyes_and_one_on_the_brow() {
        let across = hand((1210.0, 420.0), -90.0, 180.0, OPEN);
        assert!(cover(&across).is_facepalm(), "{:?}", cover(&across));
        let brow = hand((1200.0, 330.0), -95.0, 180.0, OPEN);
        assert!(cover(&brow).is_facepalm(), "{:?}", cover(&brow));
    }

    #[test]
    fn the_other_hand_and_another_frame_shape_measure_alike() {
        let left = hand((880.0, 590.0), 25.0, 180.0, OPEN);
        assert!(covers_face(&left, &FACE), "{:?}", cover(&left));
        let squeezed = |joints: HandJoints| {
            let squeeze = |point: Option<Point>| {
                point.map(|point| Point {
                    x: 0.5 + (point.x - 0.5) * 0.75,
                    ..point
                })
            };
            HandJoints {
                wrist: squeeze(joints.wrist),
                thumb: joints.thumb.map(squeeze),
                index: joints.index.map(squeeze),
                middle: joints.middle.map(squeeze),
                ring: joints.ring.map(squeeze),
                little: joints.little.map(squeeze),
            }
        };
        let face = FaceBox {
            x1: 0.5 + (FACE.x1 - 0.5) * 0.75,
            x2: 0.5 + (FACE.x2 - 0.5) * 0.75,
            ..FACE
        };
        let picard = hand((1040.0, 590.0), -25.0, 180.0, OPEN);
        let (wide, narrow) = (cover(&picard), face_cover(&squeezed(picard), &face).unwrap());
        assert!((wide.upper - narrow.upper).abs() < 0.02, "{wide:?} {narrow:?}");
        assert!((wide.area - narrow.area).abs() < 0.01, "{wide:?} {narrow:?}");
    }
}

mod not_facepalms {
    use super::*;

    #[test]
    fn a_chin_on_a_fist_or_a_finger_up_the_cheek() {
        let chin = hand((960.0, 860.0), 0.0, 170.0, FIST);
        assert_eq!(cover(&chin).upper, 0.0);
        let thinker = hand((1040.0, 760.0), -5.0, 170.0, [true, false, false, false]);
        assert!(cover(&thinker).upper < 0.05, "{:?}", cover(&thinker));
        assert!(!covers_face(&chin, &FACE) && !covers_face(&thinker, &FACE));
    }

    #[test]
    fn a_hand_over_the_mouth() {
        let yawn = hand((1150.0, 640.0), -85.0, 170.0, OPEN);
        assert!(!covers_face(&yawn, &FACE), "{:?}", cover(&yawn));
    }

    #[test]
    fn scratching_an_ear() {
        let scratch = hand((1290.0, 600.0), -35.0, 170.0, FIST);
        let measured = cover(&scratch);
        assert!(measured.upper < 0.05 && measured.middle == 0.0, "{measured:?}");
    }

    #[test]
    fn resting_a_cheek_on_a_hand() {
        let cheek = hand((1170.0, 720.0), -8.0, 180.0, OPEN);
        let measured = cover(&cheek);
        assert!(measured.upper < FACEPALM_COVER && measured.middle < 0.1, "{measured:?}");
    }

    #[test]
    fn propping_up_a_temple_stays_off_the_middle_of_the_face() {
        let temple = hand((1215.0, 560.0), -12.0, 180.0, OPEN);
        let measured = cover(&temple);
        assert!(
            measured.upper >= FACEPALM_COVER,
            "it covers enough of the upper half: {measured:?}"
        );
        assert!(
            measured.middle < FACEPALM_MIDDLE_COVER / 2.0,
            "but not between the eyes: {measured:?}"
        );
        assert!(!measured.is_facepalm());
    }

    #[test]
    fn a_hand_near_the_camera_is_too_large_for_the_face() {
        let near = hand((960.0, 1300.0), 0.0, 600.0, OPEN);
        let measured = cover(&near);
        assert!(measured.upper > 0.9 && measured.area > 3.0, "{measured:?}");
        assert!(!measured.is_facepalm());
    }

    #[test]
    fn a_hand_with_few_joints_located() {
        let mut sparse = hand((1040.0, 590.0), -25.0, 180.0, OPEN);
        sparse.thumb = [None; 4];
        for finger in [
            &mut sparse.index,
            &mut sparse.middle,
            &mut sparse.ring,
            &mut sparse.little,
        ] {
            finger[1] = None;
            finger[2] = None;
        }
        let measured = cover(&sparse);
        assert_eq!(measured.joints, 9);
        assert!(
            measured.is_facepalm(),
            "nine joints still outline the hand: {measured:?}"
        );
        sparse.index = [None; 4];
        sparse.wrist = None;
        assert_eq!(cover(&sparse).joints, 6);
        assert!(!covers_face(&sparse, &FACE));
    }

    #[test]
    fn joints_in_a_line_or_a_face_without_area_measure_nothing() {
        let point = |x: f32, y: f32| Some(Point { x, y });
        let line = HandJoints {
            wrist: point(0.45, 0.3),
            index: [
                point(0.5, 0.3),
                point(0.55, 0.3),
                point(0.58, 0.3),
                point(f32::NAN, 0.3),
            ],
            ..HandJoints::default()
        };
        assert_eq!(face_cover(&line, &FACE), None);
        let picard = hand((1040.0, 590.0), -25.0, 180.0, OPEN);
        let flat = FaceBox { y2: FACE.y1, ..FACE };
        assert_eq!(face_cover(&picard, &flat), None);
        assert_eq!(face_cover(&HandJoints::default(), &FACE), None);
    }
}

#[test]
fn a_hand_over_the_face_is_a_facepalm_and_a_hint_after_the_mouth() {
    let over_face = Hand {
        over_face: true,
        ..Hand::default()
    };
    let perception = Perception {
        faces: vec![FACE],
        best_face: 0.97,
        mouth_opening: Some(0.02),
        hands: vec![Hand::default(), over_face],
        ..Perception::default()
    };
    assert!(perception.facepalm());
    assert_eq!(
        facts(&perception),
        [
            "face in view (0.97)",
            "mouth closed (0.02)",
            FACEPALM_FACT,
            "hand in view"
        ]
    );
    assert_eq!(FACEPALM_FACT, "hand over face");
    let without = Perception {
        hands: vec![Hand::default()],
        ..perception
    };
    assert!(!without.facepalm());
    assert!(!facts(&without).iter().any(|fact| fact == FACEPALM_FACT));
}
