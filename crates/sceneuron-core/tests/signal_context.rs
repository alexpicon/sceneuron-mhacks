// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises hand signals by context: while no vote is open, a pose that is both a sign and a
//! finger count goes to the sign's deck; while one is, its count goes to the vote and thumbs still
//! reach their deck; with no deck reading such a sign, finger counts go as before; two decks that
//! read the same sign both raise it; and the snapshot says where hand signals go.

mod common;

use common::camera::{by_camera, count, listening, nothing, pose, show, shown, sign};
use common::engine;
use sceneuron_core::{CardAction, HandSign, HandSignalsView, SignalSummary};
use serde_json::json;

#[test]
fn while_no_vote_is_open_a_sign_that_counts_goes_to_its_deck_alone() {
    let project = listening(&["meeting", "poker"]);
    let cases = [
        (pose(HandSign::Fist, 0), Some("wrap-up")),
        (pose(HandSign::Pointing, 1), Some("question")),
        (pose(HandSign::Peace, 2), Some("next-topic")),
        (pose(HandSign::Peace, 3), Some("next-topic")),
        (pose(HandSign::OpenPalm, 5), None),
        (count(4), None),
    ];
    for (summary, meeting) in cases {
        let mut engine = engine(project.clone());
        show(&mut engine, &summary, 0, 700);
        assert_eq!(shown(&engine, "meeting"), meeting.and_then(by_camera), "{summary:?}");
        assert_eq!(
            shown(&engine, "poker"),
            None,
            "{summary:?}: finger counts wait for a vote"
        );
    }
}

#[test]
fn while_a_vote_is_open_finger_counts_go_to_it_and_thumbs_still_reach_their_deck() {
    let project = listening(&["meeting", "poker"]);
    let both_hands = SignalSummary {
        fingers: Some(3),
        signs: vec![HandSign::ThumbsDown],
        ..SignalSummary::default()
    };
    let cases = [
        (pose(HandSign::Fist, 0), None, Some("p0")),
        (pose(HandSign::Pointing, 1), None, Some("p1")),
        (pose(HandSign::Peace, 2), None, Some("p2")),
        (pose(HandSign::OpenPalm, 5), None, Some("p5")),
        (sign(HandSign::ThumbsUp), Some("agree"), None),
        (both_hands, Some("disagree"), Some("p3")),
    ];
    for (summary, meeting, poker) in cases {
        let mut engine = engine(project.clone());
        engine.set_room_round(Some("poker"));
        show(&mut engine, &summary, 0, 700);
        assert_eq!(shown(&engine, "meeting"), meeting.and_then(by_camera), "{summary:?}");
        assert_eq!(shown(&engine, "poker"), poker.and_then(by_camera), "{summary:?}");
    }
}

#[test]
fn when_a_vote_opens_the_sign_card_lowers_and_the_count_raises_its_card() {
    let mut engine = engine(listening(&["meeting", "poker"]));
    show(&mut engine, &pose(HandSign::Fist, 0), 0, 700);
    assert_eq!(shown(&engine, "meeting"), by_camera("wrap-up"));
    assert!(engine.set_room_round(Some("poker")).changed);
    show(&mut engine, &pose(HandSign::Fist, 0), 800, 1_400);
    assert_eq!(
        shown(&engine, "poker"),
        None,
        "the count is held for 700 ms like any signal"
    );
    show(&mut engine, &pose(HandSign::Fist, 0), 1_500, 2_200);
    assert_eq!(shown(&engine, "poker"), by_camera("p0"));
    assert_eq!(
        shown(&engine, "meeting"),
        None,
        "a fist the meeting deck no longer hears ends like its signal"
    );
}

#[test]
fn with_no_deck_reading_a_sign_that_counts_finger_counts_go_as_before() {
    let mut alone = engine(listening(&["poker"]));
    show(&mut alone, &pose(HandSign::Fist, 0), 0, 700);
    assert_eq!(shown(&alone, "poker"), by_camera("p0"), "no deck reads signs");

    // The classroom deck reads a raised hand and a thumbs up, which give no finger count.
    let mut classroom = engine(listening(&["poker", "classroom"]));
    show(&mut classroom, &count(3), 0, 700);
    assert_eq!(shown(&classroom, "poker"), by_camera("p3"));

    let mut project = listening(&["meeting", "poker"]);
    project.decks[0].camera = false;
    let mut off = engine(project);
    show(&mut off, &pose(HandSign::Fist, 0), 0, 700);
    assert_eq!(
        shown(&off, "poker"),
        by_camera("p0"),
        "the meeting deck's hand signals are off"
    );
}

#[test]
fn two_decks_reading_the_same_sign_both_raise_it() {
    let mut engine = engine(listening(&["meeting", "poker", "classroom"]));
    show(&mut engine, &sign(HandSign::ThumbsUp), 0, 700);
    assert_eq!(
        (shown(&engine, "meeting"), shown(&engine, "classroom")),
        (by_camera("agree"), by_camera("got-it"))
    );
    engine.card("meeting", None, CardAction::Lower, 750).unwrap();
    engine.card("classroom", None, CardAction::Lower, 750).unwrap();
    engine.set_room_round(Some("poker"));
    show(&mut engine, &nothing(), 800, 2_400);
    show(&mut engine, &sign(HandSign::ThumbsUp), 2_500, 3_200);
    assert_eq!(
        (shown(&engine, "meeting"), shown(&engine, "classroom")),
        (by_camera("agree"), by_camera("got-it")),
        "a vote leaves thumbs alone"
    );
}

#[test]
fn the_snapshot_says_where_hand_signals_go() {
    let view = |sign_decks: &[&str], count_decks: &[&str], to: &[&str]| {
        let owned = |ids: &[&str]| ids.iter().map(|id| (*id).to_owned()).collect();
        Some(HandSignalsView {
            sign_decks: owned(sign_decks),
            count_decks: owned(count_decks),
            to: owned(to),
        })
    };
    let mut engine = engine(listening(&["meeting", "poker", "classroom"]));
    let competing = view(&["meeting"], &["poker"], &["meeting"]);
    assert_eq!(engine.snapshot(0).hand_signals, competing);
    engine.set_room_round(Some("poker"));
    assert_eq!(
        engine.snapshot(0).hand_signals,
        view(&["meeting"], &["poker"], &["poker"])
    );
    let json = serde_json::to_value(engine.snapshot(0)).unwrap();
    assert_eq!(
        json["hand_signals"],
        json!({"sign_decks": ["meeting"], "count_decks": ["poker"], "to": ["poker"]})
    );
    let poker = json["decks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|deck| deck["id"] == "poker");
    assert_eq!(poker.unwrap()["vote"], json!({"by": "room", "until_ms": null}));

    let mut project = engine.project().clone();
    project.decks[0].camera = false;
    engine.replace_project(project, 10).unwrap();
    assert_eq!(engine.snapshot(10).hand_signals, None, "nothing competes");
    assert_eq!(common::engine(listening(&["meeting"])).snapshot(0).hand_signals, None);
}
