// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the live description's summary of a perception: people from faces or bodies, hands
//! with their side, sign, and count, a hand over the face as the facepalm, arms up, the mouth's
//! threshold, labels that repeat an animal, and the line a full perception reads as.

use sceneuron::sighting::sighting;
use sceneuron_core::{HandSide, HandSign, SeenHand, Sighting};
use sceneuron_vision::{Body, Chirality, FaceBox, Gesture, Hand, Label, MOUTH_OPEN, Perception, TextLine};

fn face() -> FaceBox {
    FaceBox {
        x1: 0.4,
        y1: 0.2,
        x2: 0.6,
        y2: 0.5,
        score: 0.97,
    }
}

fn label(name: &str) -> Label {
    Label {
        name: name.to_owned(),
        confidence: 0.8,
    }
}

#[test]
fn nothing_in_view_gives_an_empty_sighting() {
    assert_eq!(sighting(&Perception::default()), Sighting::default());
    assert_eq!(sighting(&Perception::default()).describe(), "Nobody in view");
}

#[test]
fn people_are_the_faces_or_the_bodies_whichever_are_more() {
    let faces = Perception {
        faces: vec![face(), face()],
        bodies: vec![Body::default()],
        ..Perception::default()
    };
    assert_eq!(sighting(&faces).people, 2);
    let bodies = Perception {
        faces: vec![face()],
        bodies: vec![Body::default(); 3],
        ..Perception::default()
    };
    assert_eq!(sighting(&bodies).people, 3);
}

#[test]
fn hands_keep_their_side_sign_and_count_and_a_hand_over_the_face_is_the_facepalm() {
    let perception = Perception {
        faces: vec![face()],
        hands: vec![
            Hand {
                chirality: Chirality::Left,
                raised: true,
                gesture: Some(Gesture::Peace),
                extended: Some(2),
                over_face: false,
            },
            Hand {
                chirality: Chirality::Right,
                raised: true,
                gesture: None,
                extended: Some(4),
                over_face: true,
            },
            Hand {
                chirality: Chirality::Unknown,
                ..Hand::default()
            },
        ],
        ..Perception::default()
    };
    let seen = sighting(&perception);
    assert!(seen.facepalm);
    assert_eq!(
        seen.hands,
        [
            SeenHand {
                side: HandSide::Left,
                raised: true,
                sign: Some(HandSign::Peace),
                fingers: Some(2),
            },
            SeenHand::default(),
        ]
    );
    assert_eq!(
        seen.describe(),
        "1 person; facepalm; left hand raised, peace sign; hand in view"
    );
}

#[test]
fn arms_up_are_the_most_one_person_holds_up() {
    let arms = |bodies: Vec<Body>| {
        sighting(&Perception {
            bodies,
            ..Perception::default()
        })
        .arms_up
    };
    let left = Body {
        left_arm_up: true,
        ..Body::default()
    };
    let both = Body {
        left_arm_up: true,
        right_arm_up: true,
        arms_up: true,
    };
    assert_eq!(arms(Vec::new()), 0);
    assert_eq!(arms(vec![left, left]), 1, "two people with one arm up each");
    assert_eq!(arms(vec![left, both]), 2);
}

#[test]
fn the_mouth_is_open_from_the_threshold_the_hints_use() {
    let mouth = |opening| {
        sighting(&Perception {
            mouth_opening: opening,
            ..Perception::default()
        })
        .mouth_open
    };
    assert!(!mouth(None));
    assert!(!mouth(Some(MOUTH_OPEN - 0.01)));
    assert!(mouth(Some(MOUTH_OPEN)));
}

#[test]
fn a_full_perception_reads_as_one_line() {
    let perception = Perception {
        faces: vec![face(), face()],
        best_face: 0.97,
        mouth_opening: Some(0.3),
        hands: vec![Hand {
            chirality: Chirality::Right,
            raised: true,
            gesture: None,
            extended: Some(2),
            over_face: false,
        }],
        bodies: Vec::new(),
        animals: vec![label("cat")],
        labels: vec![label("cat"), label("guitar"), label("coffee mug")],
        text: vec![TextLine {
            text: "OPEN".to_owned(),
            confidence: 0.9,
        }],
    };
    let seen = sighting(&perception);
    assert_eq!(seen.animals, ["cat"]);
    assert_eq!(
        seen.things,
        ["guitar", "coffee mug"],
        "the cat label repeats the animal"
    );
    assert_eq!(
        seen.describe(),
        "2 people; right hand raised, 2 fingers up; mouth open; seen: cat, guitar, coffee mug; text: “OPEN”"
    );
}
