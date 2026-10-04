// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises how the camera shares decks with people and settings: cards raised by hand win,
//! a card lowered by hand waits for its signal anew, the per-deck camera switch, pausing
//! automation, auto-hide, and packs once their camera is on.

mod common;

use common::camera::{by_camera, camera_project, nothing, raised, raised_by_camera, show, sign};
use common::{auto_hide_project, engine};
use sceneuron_core::{
    CardAction, CardSignal, Engine, HandSign, RaisedBy, SignalSummary, default_project, pack_for_canvas, packs,
};

#[test]
fn the_camera_never_replaces_or_lowers_a_card_raised_by_hand() {
    let mut engine = engine(camera_project());
    engine.card("votes", Some("no"), CardAction::Raise, 0).unwrap();
    assert!(!show(&mut engine, &sign(HandSign::ThumbsUp), 0, 2_000));
    assert!(!show(&mut engine, &nothing(), 2_100, 6_000));
    assert_eq!(raised(&engine), Some(("no".to_owned(), RaisedBy::Manual)));

    let mut engine = raised_by_camera();
    engine.card("votes", Some("maybe"), CardAction::Raise, 750).unwrap();
    show(&mut engine, &nothing(), 800, 5_000);
    assert_eq!(raised(&engine), Some(("maybe".to_owned(), RaisedBy::Manual)));
}

#[test]
fn a_card_lowered_by_hand_waits_for_its_signal_to_be_shown_again() {
    let mut engine = raised_by_camera();
    engine.card("votes", None, CardAction::Lower, 750).unwrap();
    assert!(!show(&mut engine, &sign(HandSign::ThumbsUp), 800, 3_000));
    assert_eq!(raised(&engine), None);
    engine.observe_signals(&nothing(), 3_100);
    assert!(
        !show(&mut engine, &sign(HandSign::ThumbsUp), 3_200, 4_000),
        "a missed frame does not show the signal anew"
    );
    show(&mut engine, &nothing(), 4_100, 5_500);
    show(&mut engine, &sign(HandSign::ThumbsUp), 5_600, 6_200);
    assert_eq!(raised(&engine), None);
    engine.observe_signals(&sign(HandSign::ThumbsUp), 6_300);
    assert_eq!(raised(&engine), by_camera("yes"));

    let mut engine = common::engine(camera_project());
    engine.card("votes", Some("no"), CardAction::Raise, 0).unwrap();
    show(&mut engine, &sign(HandSign::ThumbsUp), 0, 1_000);
    engine.card("votes", None, CardAction::Lower, 1_050).unwrap();
    show(&mut engine, &sign(HandSign::ThumbsUp), 1_100, 3_000);
    assert_eq!(
        raised(&engine),
        None,
        "a signal held while a hand-raised card was up is spent"
    );
}

#[test]
fn the_camera_stays_off_until_a_deck_turns_it_on() {
    let mut project = camera_project();
    project.decks[0].camera = false;
    let mut engine = engine(project.clone());
    assert!(!show(&mut engine, &sign(HandSign::ThumbsUp), 0, 3_000));
    assert_eq!(raised(&engine), None);
    project.decks[0].camera = true;
    engine.replace_project(project, 3_050).unwrap();
    show(&mut engine, &sign(HandSign::ThumbsUp), 3_100, 3_700);
    assert_eq!(raised(&engine), None, "the hold starts when the camera is on");
    engine.observe_signals(&sign(HandSign::ThumbsUp), 3_800);
    assert_eq!(raised(&engine), by_camera("yes"));
}

#[test]
fn turning_the_camera_off_lowers_its_card_like_a_signal_ending() {
    let mut engine = raised_by_camera();
    let mut project = camera_project();
    project.decks[0].camera = false;
    engine.replace_project(project, 750).unwrap();
    show(&mut engine, &sign(HandSign::ThumbsUp), 800, 2_100);
    assert_eq!(raised(&engine), by_camera("yes"));
    engine.observe_signals(&sign(HandSign::ThumbsUp), 2_200);
    assert_eq!(raised(&engine), None);
}

#[test]
fn auto_hide_applies_to_camera_cards_and_a_held_signal_does_not_raise_them_again() {
    let mut project = auto_hide_project(1_000);
    project.decks[0].camera = true;
    project.decks[0].cards[0].signal = Some(CardSignal::Sign(HandSign::ThumbsUp));
    let mut engine = engine(project);
    show(&mut engine, &sign(HandSign::ThumbsUp), 0, 700);
    assert_eq!(engine.snapshot(700).decks[0].hide_at_ms, Some(1_700));
    engine.observe_signals(&nothing(), 800);
    assert!(!show(&mut engine, &sign(HandSign::ThumbsUp), 900, 1_600));
    assert_eq!(
        engine.snapshot(1_600).decks[0].hide_at_ms,
        Some(1_700),
        "a missed frame neither raises the card again nor delays its auto-hide"
    );
    assert!(engine.tick(1_700).changed);
    engine.observe_signals(&nothing(), 1_800);
    assert!(!show(&mut engine, &sign(HandSign::ThumbsUp), 1_900, 5_000));
    assert_eq!(raised(&engine), None);
}

#[test]
fn pausing_automation_stops_the_camera_until_it_resumes() {
    let mut engine = engine(camera_project());
    engine.set_automation(false, 0);
    assert!(!show(&mut engine, &sign(HandSign::ThumbsUp), 0, 3_000));
    assert_eq!(raised(&engine), None);
    engine.set_automation(true, 3_050);
    show(&mut engine, &sign(HandSign::ThumbsUp), 3_100, 3_700);
    assert_eq!(raised(&engine), None, "the hold starts when automation resumes");
    engine.observe_signals(&sign(HandSign::ThumbsUp), 3_800);
    assert_eq!(raised(&engine), by_camera("yes"));

    let mut engine = raised_by_camera();
    engine.set_automation(false, 750);
    show(&mut engine, &sign(HandSign::ThumbsUp), 800, 2_100);
    assert_eq!(raised(&engine), by_camera("yes"));
    assert!(engine.observe_signals(&sign(HandSign::ThumbsUp), 2_200).changed);
    assert_eq!(
        raised(&engine),
        None,
        "a card the camera raised lowers as when its signal ends"
    );

    let mut engine = common::engine(camera_project());
    engine.card("votes", Some("no"), CardAction::Raise, 0).unwrap();
    engine.set_automation(false, 50);
    show(&mut engine, &sign(HandSign::ThumbsUp), 100, 3_000);
    assert_eq!(raised(&engine), Some(("no".to_owned(), RaisedBy::Manual)));
}

#[test]
fn packs_raise_their_signal_cards_once_their_camera_is_on() {
    let mut project = default_project();
    for pack in packs() {
        project = project
            .add_pack(&pack_for_canvas(&pack.id, &project.canvas).unwrap())
            .unwrap();
    }
    let summary = |hands_raised: u8, fingers: Option<u8>, signs: &[HandSign]| SignalSummary {
        hands_raised,
        fingers,
        signs: signs.to_vec(),
        ..SignalSummary::default()
    };
    // While no vote is open, Planning poker takes no finger counts, since Meeting signals reads
    // signs that count too: a fist is 0 fingers, pointing 1, and a peace sign 2.
    let cases = [
        (summary(1, Some(8), &[HandSign::ThumbsUp]), ["agree", "-", "hand", "-"]),
        (
            summary(0, Some(3), &[HandSign::ThumbsDown]),
            ["disagree", "-", "-", "-"],
        ),
        (summary(0, None, &[HandSign::ThumbsUp]), ["agree", "-", "got-it", "-"]),
        (summary(0, Some(0), &[HandSign::Fist]), ["wrap-up", "-", "-", "-"]),
        (summary(0, Some(1), &[HandSign::Pointing]), ["question", "-", "-", "-"]),
        (
            summary(1, Some(2), &[HandSign::Peace]),
            ["next-topic", "-", "hand", "-"],
        ),
        (summary(0, Some(5), &[HandSign::OpenPalm]), ["-", "-", "-", "-"]),
        (summary(0, Some(4), &[]), ["-", "-", "-", "-"]),
    ];
    let shown = |engine: &Engine| -> Vec<String> {
        let decks = engine.snapshot(0).decks.into_iter();
        decks
            .map(|deck| deck.raised.unwrap_or_else(|| "-".to_owned()))
            .collect()
    };
    let mut off = engine(project.clone());
    show(&mut off, &cases[0].0, 0, 1_000);
    assert_eq!(shown(&off), ["-"; 4], "every pack's camera starts off");
    for deck in &mut project.decks {
        deck.camera = true;
    }
    for (summary, expected) in cases {
        let mut engine = engine(project.clone());
        show(&mut engine, &summary, 0, 700);
        assert_eq!(shown(&engine), expected, "{summary:?}");
    }
    let mut voting = engine(project);
    voting.set_room_round(Some("poker"));
    show(&mut voting, &summary(1, Some(2), &[HandSign::Peace]), 0, 700);
    assert_eq!(shown(&voting), ["-", "p2", "hand", "-"], "while a vote is open");
}
