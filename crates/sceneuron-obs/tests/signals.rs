// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the card-signal summary of a perception: raised hands and arms, finger counts added
//! up over hands, thumbs signs kept out of the count and the raised hands, unknown counts, signs,
//! a hand over the face and its arm kept out of everything, and the pack cards a raised thumbs up
//! raises.

use sceneuron::signals::signal_summary;
use sceneuron_core::{Engine, HandSign, SignalSummary, default_project, pack_for_canvas};
use sceneuron_vision::{Body, Chirality, Gesture, Hand, Perception};

fn hand(gesture: Option<Gesture>, extended: Option<u8>) -> Hand {
    Hand {
        chirality: Chirality::Unknown,
        raised: false,
        gesture,
        extended,
        over_face: false,
    }
}

fn hands(hands: Vec<Hand>) -> SignalSummary {
    signal_summary(&Perception {
        hands,
        ..Perception::default()
    })
}

#[test]
fn nothing_in_view_gives_an_empty_summary() {
    assert_eq!(signal_summary(&Perception::default()), SignalSummary::default());
    assert_eq!(SignalSummary::default().fingers, None, "no hand gives no count");
}

#[test]
fn counts_add_up_over_two_hands() {
    assert_eq!(hands(vec![hand(None, Some(3))]).fingers, Some(3));
    assert_eq!(hands(vec![hand(None, Some(5)), hand(None, Some(3))]).fingers, Some(8));
    assert_eq!(
        hands(vec![
            hand(Some(Gesture::OpenPalm), Some(5)),
            hand(Some(Gesture::OpenPalm), Some(5))
        ])
        .fingers,
        Some(10)
    );
    assert_eq!(hands(vec![hand(Some(Gesture::Fist), Some(0))]).fingers, Some(0));
}

#[test]
fn an_unknown_count_on_any_counted_hand_makes_the_sum_unknown() {
    assert_eq!(hands(vec![hand(None, None)]).fingers, None);
    assert_eq!(hands(vec![hand(None, Some(3)), hand(None, None)]).fingers, None);
    assert_eq!(hands(vec![hand(None, None), hand(None, Some(3))]).fingers, None);
}

#[test]
fn thumbs_up_and_down_give_their_sign_and_no_count() {
    let up = hands(vec![hand(Some(Gesture::ThumbsUp), Some(1))]);
    assert_eq!((up.fingers, up.signs), (None, vec![HandSign::ThumbsUp]));
    let down = hands(vec![hand(Some(Gesture::ThumbsDown), Some(1))]);
    assert_eq!((down.fingers, down.signs), (None, vec![HandSign::ThumbsDown]));
    let beside_three = hands(vec![hand(Some(Gesture::ThumbsUp), Some(1)), hand(None, Some(3))]);
    assert_eq!(beside_three.fingers, Some(3), "only the other hand counts");
    let beside_unknown = hands(vec![hand(None, None), hand(Some(Gesture::ThumbsDown), Some(1))]);
    assert_eq!(beside_unknown.fingers, None);
    let thumbs_without_count = hands(vec![hand(Some(Gesture::ThumbsUp), None), hand(None, Some(2))]);
    assert_eq!(
        thumbs_without_count.fingers,
        Some(2),
        "a thumbs hand never makes the count unknown"
    );
}

#[test]
fn every_gesture_becomes_its_sign_once() {
    let gestures = [
        (Gesture::ThumbsUp, HandSign::ThumbsUp),
        (Gesture::ThumbsDown, HandSign::ThumbsDown),
        (Gesture::OpenPalm, HandSign::OpenPalm),
        (Gesture::Peace, HandSign::Peace),
        (Gesture::Pointing, HandSign::Pointing),
        (Gesture::Fist, HandSign::Fist),
    ];
    for (gesture, sign) in gestures {
        assert_eq!(hands(vec![hand(Some(gesture), None)]).signs, [sign], "{gesture:?}");
    }
    let twice = hands(vec![
        hand(Some(Gesture::Peace), Some(2)),
        hand(Some(Gesture::Peace), Some(2)),
    ]);
    assert_eq!((twice.signs, twice.fingers), (vec![HandSign::Peace], Some(4)));
    let mixed = hands(vec![
        hand(Some(Gesture::Fist), Some(0)),
        hand(Some(Gesture::ThumbsUp), Some(1)),
    ]);
    assert_eq!(mixed.signs, [HandSign::Fist, HandSign::ThumbsUp]);
    assert!(hands(vec![hand(None, Some(3))]).signs.is_empty());
}

#[test]
fn raised_hands_are_counted_and_any_arm_up_counts() {
    let raised = |up: bool| Hand {
        raised: up,
        ..hand(None, Some(5))
    };
    assert_eq!(hands(vec![raised(true), raised(false)]).hands_raised, 1);
    assert_eq!(hands(vec![raised(true), raised(true)]).hands_raised, 2);
    let body = |body: Body| {
        signal_summary(&Perception {
            bodies: vec![Body::default(), body],
            ..Perception::default()
        })
        .arms_up
    };
    assert!(!body(Body::default()));
    let left = Body {
        left_arm_up: true,
        ..Body::default()
    };
    let right = Body {
        right_arm_up: true,
        ..Body::default()
    };
    let both = Body {
        left_arm_up: true,
        right_arm_up: true,
        arms_up: true,
    };
    assert!(body(left) && body(right) && body(both));
}

#[test]
fn a_raised_thumbs_hand_gives_only_its_sign() {
    let raised = |gesture: Gesture| Hand {
        raised: true,
        ..hand(Some(gesture), Some(1))
    };
    let up = hands(vec![raised(Gesture::ThumbsUp)]);
    assert_eq!(
        (up.hands_raised, up.fingers, up.signs),
        (0, None, vec![HandSign::ThumbsUp])
    );
    let down = hands(vec![raised(Gesture::ThumbsDown)]);
    assert_eq!((down.hands_raised, down.signs), (0, vec![HandSign::ThumbsDown]));
    let beside_palm = hands(vec![raised(Gesture::ThumbsUp), raised(Gesture::OpenPalm)]);
    assert_eq!(beside_palm.hands_raised, 1, "only the open palm is a raised hand");
}

#[test]
fn a_hand_over_the_face_and_its_arm_give_no_signal() {
    let over_face = |chirality: Chirality| Hand {
        chirality,
        raised: true,
        over_face: true,
        ..hand(Some(Gesture::OpenPalm), Some(5))
    };
    let arms = |left_arm_up: bool, right_arm_up: bool| Body {
        left_arm_up,
        right_arm_up,
        arms_up: left_arm_up && right_arm_up,
    };
    let summary = |hands: Vec<Hand>, body: Body| {
        signal_summary(&Perception {
            hands,
            bodies: vec![body],
            ..Perception::default()
        })
    };
    let picard = summary(vec![over_face(Chirality::Right)], arms(false, true));
    assert_eq!(picard, SignalSummary::default(), "an open, raised palm with its arm up");
    assert!(summary(vec![over_face(Chirality::Right)], arms(true, false)).arms_up);
    assert!(!summary(vec![over_face(Chirality::Left)], arms(true, false)).arms_up);
    assert!(summary(vec![over_face(Chirality::Left)], arms(false, true)).arms_up);
    let unknown = summary(vec![over_face(Chirality::Unknown)], arms(true, true));
    assert!(!unknown.arms_up, "neither arm counts beside a hand of unknown side");

    let raised_peace = Hand {
        raised: true,
        ..hand(Some(Gesture::Peace), Some(2))
    };
    let beside = summary(vec![over_face(Chirality::Left), raised_peace], arms(false, false));
    let expected = SignalSummary {
        hands_raised: 1,
        arms_up: false,
        fingers: Some(2),
        signs: vec![HandSign::Peace],
    };
    assert_eq!(beside, expected, "the other hand still counts");
}

#[test]
fn a_raised_thumbs_up_raises_the_thumbs_up_cards_of_packs() {
    let mut project = default_project();
    for id in ["meeting", "classroom"] {
        project = project
            .add_pack(&pack_for_canvas(id, &project.canvas).unwrap())
            .unwrap();
    }
    for deck in &mut project.decks {
        deck.camera = true;
    }
    let mut engine = Engine::new(project, 0).unwrap();
    let thumbs_up = signal_summary(&Perception {
        hands: vec![Hand {
            raised: true,
            ..hand(Some(Gesture::ThumbsUp), Some(1))
        }],
        ..Perception::default()
    });
    for at_ms in (0..=700).step_by(100) {
        engine.observe_signals(&thumbs_up, at_ms);
    }
    let raised: Vec<Option<String>> = engine.snapshot(700).decks.into_iter().map(|deck| deck.raised).collect();
    assert_eq!(raised, [Some("agree".to_owned()), Some("got-it".to_owned())]);
}
