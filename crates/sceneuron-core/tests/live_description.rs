// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the live description's sentence: the words for every kind of fact, an empty
//! sighting, hands that read the same, many labels, the order of the parts, and the length cap.

use sceneuron_core::{HandSide, HandSign, MAX_SEEN_CHARS, SeenHand, Sighting};

fn hand(side: HandSide, raised: bool, sign: Option<HandSign>, fingers: Option<u8>) -> SeenHand {
    SeenHand {
        side,
        raised,
        sign,
        fingers,
    }
}

fn person_with(hands: Vec<SeenHand>) -> Sighting {
    Sighting {
        people: 1,
        hands,
        ..Sighting::default()
    }
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn an_empty_sighting_says_nobody_is_in_view() {
    assert_eq!(Sighting::default().describe(), "Nobody in view");
}

#[test]
fn people_are_counted() {
    let people = |people| {
        Sighting {
            people,
            ..Sighting::default()
        }
        .describe()
    };
    assert_eq!(people(1), "1 person");
    assert_eq!(people(2), "2 people");
    assert_eq!(people(12), "12 people");
}

#[test]
fn every_kind_of_fact_has_its_words_in_order() {
    let sighting = Sighting {
        people: 2,
        hands: vec![hand(HandSide::Right, true, None, Some(2))],
        facepalm: true,
        arms_up: 2,
        mouth_open: true,
        animals: names(&["cat"]),
        things: names(&["guitar", "coffee mug"]),
        text: names(&["OPEN"]),
    };
    assert_eq!(
        sighting.describe(),
        "2 people; facepalm; right hand raised, 2 fingers up; mouth open; seen: cat, guitar, coffee mug; \
         text: “OPEN”"
    );
    let arms = Sighting {
        facepalm: false,
        ..sighting
    };
    assert_eq!(
        arms.describe(),
        "2 people; right hand raised, 2 fingers up; arms up; mouth open; seen: cat, guitar, coffee mug; \
         text: “OPEN”"
    );
}

#[test]
fn a_hand_says_its_side_then_its_sign_or_else_its_fingers() {
    let cases = [
        (
            hand(HandSide::Left, false, Some(HandSign::ThumbsUp), None),
            "left hand, thumbs up",
        ),
        (
            hand(HandSide::Right, true, Some(HandSign::ThumbsDown), None),
            "right hand raised, thumbs down",
        ),
        (
            hand(HandSide::Unknown, false, Some(HandSign::OpenPalm), Some(5)),
            "hand, open palm",
        ),
        (
            hand(HandSide::Left, true, Some(HandSign::Peace), Some(2)),
            "left hand raised, peace sign",
        ),
        (
            hand(HandSide::Right, false, Some(HandSign::Pointing), Some(1)),
            "right hand, pointing",
        ),
        (
            hand(HandSide::Right, false, Some(HandSign::Fist), Some(0)),
            "right hand, fist",
        ),
        (hand(HandSide::Right, false, None, Some(1)), "right hand, 1 finger up"),
        (hand(HandSide::Left, false, None, Some(0)), "left hand, 0 fingers up"),
        (hand(HandSide::Unknown, true, None, None), "hand raised"),
        (hand(HandSide::Unknown, false, None, None), "hand in view"),
    ];
    for (seen, words) in cases {
        assert_eq!(person_with(vec![seen]).describe(), format!("1 person; {words}"));
    }
}

#[test]
fn hands_that_read_the_same_are_counted_and_others_listed_in_order() {
    let raised = hand(HandSide::Unknown, true, None, None);
    assert_eq!(person_with(vec![raised, raised]).describe(), "1 person; 2 hands raised");
    let thumbs = hand(HandSide::Left, false, Some(HandSign::ThumbsUp), None);
    assert_eq!(
        person_with(vec![thumbs, raised, thumbs]).describe(),
        "1 person; 2 left hands, thumbs up; hand raised"
    );
    let left = hand(HandSide::Left, true, None, Some(3));
    let right = hand(HandSide::Right, true, None, Some(3));
    assert_eq!(
        person_with(vec![right, left]).describe(),
        "1 person; right hand raised, 3 fingers up; left hand raised, 3 fingers up"
    );
}

#[test]
fn one_arm_up_is_left_out_while_a_hand_is_raised_and_arms_during_a_facepalm() {
    let arms = |arms_up, hands, facepalm| {
        Sighting {
            people: 1,
            hands,
            facepalm,
            arms_up,
            ..Sighting::default()
        }
        .describe()
    };
    assert_eq!(arms(1, Vec::new(), false), "1 person; an arm up");
    assert_eq!(arms(2, Vec::new(), false), "1 person; arms up");
    let raised = vec![hand(HandSide::Right, true, None, None)];
    assert_eq!(arms(1, raised.clone(), false), "1 person; right hand raised");
    assert_eq!(arms(2, raised, false), "1 person; right hand raised; arms up");
    assert_eq!(arms(2, Vec::new(), true), "1 person; facepalm");
}

#[test]
fn a_hand_or_a_mouth_without_a_person_starts_the_line_with_a_capital() {
    let hand_only = Sighting {
        hands: vec![hand(HandSide::Right, true, Some(HandSign::Peace), None)],
        ..Sighting::default()
    };
    assert_eq!(hand_only.describe(), "Right hand raised, peace sign");
    let mouth_only = Sighting {
        mouth_open: true,
        ..Sighting::default()
    };
    assert_eq!(mouth_only.describe(), "Mouth open");
}

#[test]
fn things_in_an_empty_room_follow_nobody_in_view() {
    let room = Sighting {
        animals: names(&["dog"]),
        things: names(&["sofa"]),
        text: names(&["EXIT"]),
        ..Sighting::default()
    };
    assert_eq!(room.describe(), "Nobody in view; seen: dog, sofa; text: “EXIT”");
}

#[test]
fn many_labels_name_four_animals_first_without_repeats_or_blanks() {
    let sighting = Sighting {
        people: 1,
        animals: names(&["cat", "dog"]),
        things: names(&["cat", " ", "guitar", "coffee mug", "lamp", "plant"]),
        ..Sighting::default()
    };
    assert_eq!(sighting.describe(), "1 person; seen: cat, dog, guitar, coffee mug");
}

#[test]
fn text_is_quoted_on_one_line_two_lines_at_most_each_cut_short() {
    let sighting = Sighting {
        people: 1,
        text: names(&[
            "  OPEN\nNOW ",
            "",
            "a very long line of text that goes on and on",
            "third",
        ]),
        ..Sighting::default()
    };
    assert_eq!(
        sighting.describe(),
        "1 person; text: “OPEN NOW”, “a very long line of tex…”"
    );
}

#[test]
fn a_long_line_is_cut_at_a_word_and_ends_with_an_ellipsis() {
    let sighting = Sighting {
        people: 3,
        hands: vec![
            hand(HandSide::Left, true, Some(HandSign::ThumbsUp), None),
            hand(HandSide::Right, true, None, Some(4)),
        ],
        arms_up: 2,
        mouth_open: true,
        animals: names(&["golden retriever", "siamese cat"]),
        things: names(&["electric guitar", "espresso machine"]),
        text: names(&["GRAND OPENING TODAY", "EVERYTHING HALF PRICE"]),
        ..Sighting::default()
    };
    let line = sighting.describe();
    assert!(
        line.chars().count() <= MAX_SEEN_CHARS,
        "{} characters: {line}",
        line.chars().count()
    );
    assert_eq!(
        line,
        "3 people; left hand raised, thumbs up; right hand raised, 4 fingers up; arms up; mouth open; seen: golden \
         retriever…",
        "cut before \"siamese\", which did not fit, with the comma before it"
    );
}

#[test]
fn a_long_word_with_no_space_to_cut_at_is_cut_inside() {
    let sighting = Sighting {
        hands: vec![hand(HandSide::Unknown, true, None, None)],
        things: names(&["y".repeat(2 * MAX_SEEN_CHARS).as_str()]),
        ..Sighting::default()
    };
    let line = sighting.describe();
    assert_eq!(line.chars().count(), MAX_SEEN_CHARS);
    assert!(
        line.starts_with("Hand raised; seen: yyy") && line.ends_with("y…"),
        "{line}"
    );
}

#[test]
fn a_line_that_fits_is_never_cut() {
    let sighting = Sighting {
        people: 1,
        things: names(&["x".repeat(MAX_SEEN_CHARS - "1 person; seen: ".len()).as_str()]),
        ..Sighting::default()
    };
    let line = sighting.describe();
    assert_eq!(line.chars().count(), MAX_SEEN_CHARS);
    assert!(!line.ends_with('…'));
}
