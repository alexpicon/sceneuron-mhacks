// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the hints rendered for a vision-language model: wording for faces, hands and
//! their finger counts, bodies, animals, labels, and text, their order, and removal of
//! repeats.

mod support {
    pub mod perceptions;
}

use sceneuron_vision::{Body, Chirality, Gesture, Perception, facts};
use support::perceptions::{counted, face, faces, hand, label, text};

#[test]
fn nothing_seen_gives_no_hints() {
    assert!(facts(&Perception::default()).is_empty());
    let below_threshold = Perception {
        best_face: 0.3,
        ..Perception::default()
    };
    assert!(facts(&below_threshold).is_empty());
}

#[test]
fn faces_are_counted_with_the_best_score() {
    assert_eq!(facts(&faces(&[0.981])), ["face in view (0.98)"]);
    assert_eq!(facts(&faces(&[0.9, 0.75])), ["2 faces in view (best 0.90)"]);
}

#[test]
fn hands_name_side_raise_and_gesture() {
    let perception = Perception {
        hands: vec![
            hand(Chirality::Right, true, Some(Gesture::ThumbsUp)),
            hand(Chirality::Left, false, Some(Gesture::OpenPalm)),
            hand(Chirality::Unknown, true, None),
            hand(Chirality::Right, false, None),
            hand(Chirality::Left, false, Some(Gesture::Peace)),
            hand(Chirality::Unknown, false, Some(Gesture::Pointing)),
            hand(Chirality::Right, true, Some(Gesture::Fist)),
            hand(Chirality::Left, false, Some(Gesture::ThumbsDown)),
        ],
        ..Perception::default()
    };
    assert_eq!(
        facts(&perception),
        [
            "right hand raised, thumbs up",
            "left hand, open palm",
            "hand raised",
            "right hand in view",
            "left hand, peace sign",
            "hand, pointing",
            "right hand raised, fist",
            "left hand, thumbs down",
        ]
    );
}

#[test]
fn a_finger_count_follows_its_hand() {
    let perception = Perception {
        hands: vec![
            counted(hand(Chirality::Right, true, Some(Gesture::ThumbsUp)), 1),
            hand(Chirality::Right, false, None),
            counted(hand(Chirality::Left, false, Some(Gesture::OpenPalm)), 5),
        ],
        ..Perception::default()
    };
    assert_eq!(
        facts(&perception),
        [
            "right hand raised, thumbs up",
            "1 finger up",
            "right hand in view",
            "left hand, open palm",
            "5 fingers up",
        ]
    );
}

#[test]
fn finger_counts_read_naturally_from_zero_to_five() {
    let hints = [
        "0 fingers up",
        "1 finger up",
        "2 fingers up",
        "3 fingers up",
        "4 fingers up",
        "5 fingers up",
    ];
    for (count, hint) in (0..).zip(hints) {
        let perception = Perception {
            hands: vec![counted(hand(Chirality::Unknown, true, None), count)],
            ..Perception::default()
        };
        assert_eq!(facts(&perception), ["hand raised", hint]);
    }
}

#[test]
fn two_hands_keep_a_count_each() {
    let raised = |chirality: Chirality, count: u8| counted(hand(chirality, true, None), count);
    let perception = Perception {
        hands: vec![
            raised(Chirality::Left, 3),
            raised(Chirality::Right, 3),
            raised(Chirality::Unknown, 2),
            raised(Chirality::Unknown, 2),
            raised(Chirality::Unknown, 4),
            hand(Chirality::Unknown, true, None),
            hand(Chirality::Unknown, true, None),
        ],
        ..Perception::default()
    };
    assert_eq!(
        facts(&perception),
        [
            "left hand raised",
            "3 fingers up",
            "right hand raised",
            "3 fingers up",
            "hand raised",
            "2 fingers up",
            "hand raised",
            "2 fingers up",
            "hand raised",
            "4 fingers up",
            "hand raised",
        ],
        "hands with a count are always listed; hands without one appear once"
    );
}

#[test]
fn bodies_report_raised_arms_once() {
    let arms_up = Body {
        left_arm_up: true,
        right_arm_up: true,
        arms_up: true,
    };
    let left = Body {
        left_arm_up: true,
        ..Body::default()
    };
    let right = Body {
        right_arm_up: true,
        ..Body::default()
    };
    let perception = Perception {
        bodies: vec![arms_up, Body::default(), left, right, arms_up],
        ..Perception::default()
    };
    assert_eq!(facts(&perception), ["arms up", "left arm up", "right arm up"]);
}

#[test]
fn animals_come_before_labels_without_repeating_them() {
    let perception = Perception {
        animals: vec![label("cat", 0.934)],
        labels: vec![label("cat", 0.8), label("coffee mug", 0.81)],
        ..Perception::default()
    };
    assert_eq!(facts(&perception), ["cat (0.93)", "coffee mug (0.81)"]);
}

#[test]
fn text_is_quoted_and_escaped() {
    assert_eq!(
        facts(&text(&["OAT MILK", "say \"hi\""])),
        ["text: \"OAT MILK\"", "text: \"say \\\"hi\\\"\""]
    );
}

#[test]
fn hints_follow_a_fixed_order() {
    let perception = Perception {
        hands: vec![hand(Chirality::Right, true, None)],
        labels: vec![label("guitar", 0.7)],
        ..text(&["LIVE"])
    };
    let perception = Perception {
        faces: vec![face(0.99)],
        best_face: 0.99,
        ..perception
    };
    assert_eq!(
        facts(&perception),
        [
            "face in view (0.99)",
            "right hand raised",
            "guitar (0.70)",
            "text: \"LIVE\""
        ]
    );
}
