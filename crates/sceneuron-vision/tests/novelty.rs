// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the novelty test between two perceptions: people appearing or leaving, new
//! animals and labels at the novel confidence, new gestures, more raised hands or arms, and
//! new text with the similarity and length thresholds. Finger counts alone are never new.

mod support {
    pub mod perceptions;
}

use sceneuron_vision::{
    Body, Chirality, Gesture, MIN_NOVEL_TEXT_CHARS, NOVEL_LABEL_CONFIDENCE, Perception, SAME_TEXT_SIMILARITY, novelty,
};
use support::perceptions::{counted, faces, hand, label, text};

#[test]
fn the_same_perception_is_not_new() {
    let perception = Perception {
        hands: vec![hand(Chirality::Left, true, Some(Gesture::Peace))],
        animals: vec![label("dog", 0.9)],
        ..text(&["OAT MILK"])
    };
    assert!(!novelty(&perception, &perception));
    assert!(!novelty(&Perception::default(), &Perception::default()));
}

#[test]
fn a_person_appearing_or_leaving_is_new() {
    assert!(novelty(&faces(&[]), &faces(&[0.9])));
    assert!(novelty(&faces(&[0.9]), &faces(&[])));
    assert!(novelty(&faces(&[0.9]), &faces(&[0.9, 0.8])));
    assert!(!novelty(&faces(&[0.9]), &faces(&[0.6])), "only the score changed");
}

#[test]
fn a_body_without_a_face_still_counts_as_a_person() {
    let turned_away = Perception {
        bodies: vec![Body::default()],
        ..Perception::default()
    };
    assert!(!novelty(&faces(&[0.9]), &turned_away));
    assert!(novelty(&Perception::default(), &turned_away));
}

#[test]
fn a_new_animal_or_label_needs_the_novel_confidence() {
    let with = |confidence: f32| Perception {
        animals: vec![label("cat", confidence)],
        labels: vec![label("guitar", confidence)],
        ..Perception::default()
    };
    let empty = Perception::default();
    assert!(novelty(&empty, &with(NOVEL_LABEL_CONFIDENCE)));
    assert!(!novelty(&empty, &with(NOVEL_LABEL_CONFIDENCE - 0.01)));
    let animal_only = Perception {
        animals: vec![label("dog", 0.95)],
        ..Perception::default()
    };
    assert!(novelty(&empty, &animal_only));
    let label_only = Perception {
        labels: vec![label("pizza", 0.95)],
        ..Perception::default()
    };
    assert!(novelty(&empty, &label_only));
}

#[test]
fn a_label_seen_before_at_any_confidence_is_not_new() {
    let faint = Perception {
        labels: vec![label("guitar", 0.3)],
        ..Perception::default()
    };
    let clear = Perception {
        labels: vec![label("guitar", 0.95)],
        ..Perception::default()
    };
    assert!(!novelty(&faint, &clear));
    assert!(!novelty(&clear, &Perception::default()), "a label leaving is not new");
}

#[test]
fn a_new_gesture_is_new() {
    let with = |gesture: Option<Gesture>| Perception {
        hands: vec![hand(Chirality::Right, false, gesture)],
        ..Perception::default()
    };
    assert!(novelty(&with(None), &with(Some(Gesture::ThumbsUp))));
    assert!(novelty(&with(Some(Gesture::Fist)), &with(Some(Gesture::ThumbsUp))));
    assert!(!novelty(&with(Some(Gesture::ThumbsUp)), &with(Some(Gesture::ThumbsUp))));
    assert!(
        !novelty(&with(Some(Gesture::ThumbsUp)), &with(None)),
        "a gesture ending is not new"
    );
}

#[test]
fn a_changed_finger_count_alone_is_not_new() {
    let showing = |counts: &[Option<u8>]| Perception {
        hands: counts
            .iter()
            .map(|&count| {
                let fist = hand(Chirality::Unknown, false, Some(Gesture::Fist));
                count.map_or(fist, |count| counted(fist, count))
            })
            .collect(),
        ..Perception::default()
    };
    assert!(
        !novelty(&showing(&[None]), &showing(&[Some(0)])),
        "a fist whose thumb comes into view"
    );
    assert!(!novelty(&showing(&[Some(0)]), &showing(&[None])));
    assert!(!novelty(&showing(&[Some(2)]), &showing(&[Some(3)])));
    assert!(!novelty(&showing(&[Some(3)]), &showing(&[Some(3), Some(3)])));
}

#[test]
fn more_raised_hands_or_arms_are_new() {
    let hands = |raised: &[bool]| Perception {
        hands: raised.iter().map(|&up| hand(Chirality::Unknown, up, None)).collect(),
        ..Perception::default()
    };
    assert!(novelty(&hands(&[false]), &hands(&[true])));
    assert!(novelty(&hands(&[true, false]), &hands(&[true, true])));
    assert!(!novelty(&hands(&[true, true]), &hands(&[true, false])));
    let body = |body: Body| Perception {
        bodies: vec![body],
        ..Perception::default()
    };
    let arms_up = Body {
        left_arm_up: true,
        right_arm_up: true,
        arms_up: true,
    };
    assert!(novelty(&body(Body::default()), &body(arms_up)));
    assert!(!novelty(&body(arms_up), &body(Body::default())));
}

#[test]
fn new_text_is_new() {
    assert!(novelty(&text(&[]), &text(&["OAT MILK"])));
    assert!(novelty(&text(&["OAT MILK"]), &text(&["OAT MILK", "BARISTA EDITION"])));
    assert!(novelty(&text(&["OAT MILK"]), &text(&["SOY MILK"])));
    assert!(!novelty(&text(&["OAT MILK", "BARISTA"]), &text(&["OAT MILK"])));
}

#[test]
fn recognition_noise_case_and_punctuation_are_not_new_text() {
    assert!(!novelty(&text(&["OAT MILK"]), &text(&["oat milk!"])));
    assert!(
        !novelty(&text(&["OAT MILK"]), &text(&["OAT MlLK"])),
        "one character misread"
    );
    assert!(!novelty(&text(&["BARISTA EDITION"]), &text(&["BARISTA EDlTI0N"])));
}

#[test]
fn the_text_similarity_threshold_is_exact() {
    // 8 letters and digits with 2 substitutions: similarity 0.75, the same line.
    assert_eq!(SAME_TEXT_SIMILARITY, 0.75);
    assert!(!novelty(&text(&["ABCD EFGH"]), &text(&["ABCD EFXY"])));
    // 3 substitutions: similarity 0.625, a new line.
    assert!(novelty(&text(&["ABCD EFGH"]), &text(&["ABCD EXYZ"])));
}

#[test]
fn very_short_text_is_never_new() {
    let short = "a".repeat(MIN_NOVEL_TEXT_CHARS - 1);
    assert!(!novelty(&text(&[]), &text(&[&short])));
    assert!(!novelty(&text(&[]), &text(&["!?", "  "])));
    let long_enough = "a".repeat(MIN_NOVEL_TEXT_CHARS);
    assert!(novelty(&text(&[]), &text(&[&long_enough])));
}
