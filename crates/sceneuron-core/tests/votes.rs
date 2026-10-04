// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises when a vote is open for a deck whose cards use finger counts: the shared room's open
//! round until it ends, and a card raised by a press, by voice too, for three minutes or until the
//! deck lowers, during which a count shown changes the card; a camera raise opens none, and decks
//! without finger counts have none.

mod common;

use common::camera::{by_camera, count, listening, nothing, pose, show, shown, sign};
use common::engine;
use common::voice::{final_words, meeting_engine};
use sceneuron_core::{CardAction, CardSignal, Engine, HandSign, RaisedBy, VOTE_WINDOW_MS, VoiceMode, VoteBy, VoteView};

fn press(until_ms: u64) -> Option<VoteView> {
    Some(VoteView {
        by: VoteBy::Press,
        until_ms: Some(until_ms),
    })
}

fn raise(engine: &mut Engine, deck: &str, card: &str, now_ms: u64) {
    engine.card(deck, Some(card), CardAction::Raise, now_ms).unwrap();
}

#[test]
fn the_shared_rooms_open_round_holds_a_vote_until_it_ends() {
    let mut engine = engine(listening(&["meeting", "poker"]));
    assert_eq!(engine.vote("poker", 0), None);
    assert!(engine.set_room_round(Some("poker")).changed);
    let room = Some(VoteView {
        by: VoteBy::Room,
        until_ms: None,
    });
    assert_eq!(engine.vote("poker", 10_000_000), room, "the round has no window");
    assert!(!engine.set_room_round(Some("poker")).changed);
    assert!(engine.set_room_round(None).changed);
    assert_eq!(engine.vote("poker", 0), None);
    show(&mut engine, &pose(HandSign::Fist, 0), 0, 700);
    assert_eq!(shown(&engine, "meeting"), by_camera("wrap-up"));
}

#[test]
fn a_press_holds_a_vote_for_three_minutes_while_its_card_is_up() {
    assert_eq!(VOTE_WINDOW_MS, 180_000);
    let mut engine = engine(listening(&["meeting", "poker"]));
    raise(&mut engine, "poker", "p5", 1_000);
    assert_eq!(engine.vote("poker", 1_000), press(181_000));
    assert_eq!(engine.snapshot(1_000).decks[1].vote, press(181_000));
    assert_eq!(engine.vote("poker", 180_999), press(181_000));
    assert_eq!(engine.vote("poker", 181_000), None, "the window ran out");
    assert!(!engine.tick(180_999).changed);
    assert!(engine.tick(181_000).changed, "the snapshot stops showing the vote");
    assert_eq!(engine.snapshot(181_000).decks[1].vote, None);
    assert!(!engine.tick(181_001).changed);
    show(&mut engine, &pose(HandSign::Fist, 0), 181_100, 181_800);
    assert_eq!(shown(&engine, "meeting"), by_camera("wrap-up"));
    assert_eq!(engine.raised_card("poker"), Some("p5"), "the card stays up");
}

#[test]
fn a_count_shown_during_a_vote_a_press_opened_changes_its_card_while_the_vote_is_open() {
    let mut engine = engine(listening(&["meeting", "poker"]));
    raise(&mut engine, "poker", "p5", 1_000);
    show(&mut engine, &pose(HandSign::OpenPalm, 5), 1_100, 1_800);
    let pressed = Some(("p5".to_owned(), RaisedBy::Manual));
    assert_eq!(
        shown(&engine, "poker"),
        pressed,
        "the count of the card already up leaves it"
    );
    show(&mut engine, &pose(HandSign::Fist, 0), 2_000, 2_700);
    assert_eq!(shown(&engine, "poker"), by_camera("p0"), "the fist goes to the vote");
    assert_eq!(shown(&engine, "meeting"), None);
    show(&mut engine, &nothing(), 2_800, 120_000);
    assert_eq!(shown(&engine, "poker"), by_camera("p0"), "the vote keeps the count up");
    show(&mut engine, &count(3), 120_100, 120_800);
    assert_eq!(shown(&engine, "poker"), by_camera("p3"));
    assert_eq!(
        engine.vote("poker", 120_800),
        press(181_000),
        "a count keeps the window"
    );
    show(&mut engine, &nothing(), 120_900, 180_900);
    assert_eq!(shown(&engine, "poker"), by_camera("p3"));
    show(&mut engine, &nothing(), 181_000, 181_000);
    assert_eq!(
        shown(&engine, "poker"),
        None,
        "once the vote ends, the count lowers as its signal ended"
    );
    assert_eq!(engine.vote("poker", 181_000), None);
}

#[test]
fn only_a_count_changes_a_pressed_card_and_only_during_a_vote_a_press_opened() {
    let pressed = Some(("p5".to_owned(), RaisedBy::Manual));
    let mut project = listening(&["poker"]);
    let coffee = project.decks[0].cards.iter_mut().find(|card| card.id == "break");
    coffee.unwrap().signal = Some(CardSignal::Sign(HandSign::ThumbsUp));
    let mut engine = engine(project);
    raise(&mut engine, "poker", "p5", 0);
    show(&mut engine, &sign(HandSign::ThumbsUp), 100, 800);
    assert_eq!(shown(&engine, "poker"), pressed, "a thumbs up is no count");

    // In the shared room, the press cast the vote.
    let mut room = common::engine(listening(&["meeting", "poker"]));
    room.set_room_round(Some("poker"));
    raise(&mut room, "poker", "p5", 0);
    show(&mut room, &count(3), 100, 800);
    assert_eq!(shown(&room, "poker"), pressed, "the room's round");
}

#[test]
fn raising_the_card_already_up_keeps_its_window_and_another_card_starts_one() {
    for auto_hide_ms in [0, 600_000] {
        let mut project = listening(&["poker"]);
        project.decks[0].auto_hide_ms = auto_hide_ms;
        let mut engine = engine(project);
        raise(&mut engine, "poker", "p5", 1_000);
        raise(&mut engine, "poker", "p5", 100_000);
        assert_eq!(
            engine.vote("poker", 100_000),
            press(181_000),
            "auto-hide {auto_hide_ms}"
        );
        assert_eq!(engine.vote("poker", 181_000), None, "auto-hide {auto_hide_ms}");
        raise(&mut engine, "poker", "p8", 182_000);
        assert_eq!(
            engine.vote("poker", 182_000),
            press(362_000),
            "auto-hide {auto_hide_ms}"
        );
    }
}

#[test]
fn a_card_raised_by_voice_holds_a_vote_too() {
    let mut engine = meeting_engine(VoiceMode::AlwaysOn);
    engine.hear(&final_words("Sceneuron, vote eight"), 100);
    assert_eq!(engine.raised_card("poker"), Some("p8"));
    assert_eq!(engine.vote("poker", 100), press(180_100));
}

#[test]
fn any_lower_of_the_deck_ends_a_vote_a_press_opened() {
    let mut engine = engine(listening(&["poker"]));
    raise(&mut engine, "poker", "p3", 0);
    engine.card("poker", None, CardAction::Lower, 10).unwrap();
    assert_eq!(engine.vote("poker", 10), None, "a lower");
    raise(&mut engine, "poker", "p3", 20);
    engine.lower_all_cards(30);
    assert_eq!(engine.vote("poker", 30), None, "lowering every deck");

    let mut project = listening(&["poker"]);
    project.decks[0].auto_hide_ms = 5_000;
    let mut engine = common::engine(project);
    raise(&mut engine, "poker", "p3", 0);
    assert!(engine.tick(5_000).changed);
    assert_eq!(engine.vote("poker", 5_000), None, "auto-hide");
    raise(&mut engine, "poker", "p3", 6_000);
    let mut project = engine.project().clone();
    project.decks[0].cards.retain(|card| card.id != "p3");
    engine.replace_project(project, 7_000).unwrap();
    assert_eq!(engine.vote("poker", 7_000), None, "a save that removed the card");
}

#[test]
fn the_end_of_the_rooms_round_ends_a_vote_a_press_opened() {
    let mut engine = engine(listening(&["meeting", "poker"]));
    engine.set_room_round(Some("poker"));
    raise(&mut engine, "poker", "p3", 10);
    // The reveal raises the counted vote, then the room client reports the round's end.
    raise(&mut engine, "poker", "p5", 20);
    engine.set_room_round(None);
    assert_eq!(engine.vote("poker", 20), None);
    raise(&mut engine, "poker", "p8", 30);
    assert_eq!(
        engine.vote("poker", 30),
        press(180_030),
        "a press after the reveal opens one"
    );
}

#[test]
fn a_card_the_camera_raised_holds_no_vote() {
    let mut project = listening(&["meeting", "poker"]);
    project.decks[0].camera = false;
    let mut engine = common::engine(project.clone());
    show(&mut engine, &pose(HandSign::Peace, 3), 0, 700);
    assert_eq!(shown(&engine, "poker"), by_camera("p3"));
    assert_eq!(engine.vote("poker", 700), None);
    project.decks[0].camera = true;
    engine.replace_project(project, 750).unwrap();
    show(&mut engine, &pose(HandSign::Fist, 0), 800, 2_300);
    assert_eq!(shown(&engine, "meeting"), by_camera("wrap-up"));
    assert_eq!(
        shown(&engine, "poker"),
        None,
        "the count went unheard, so its card lowered"
    );
}

#[test]
fn only_decks_whose_cards_use_finger_counts_have_a_vote() {
    let mut project = listening(&["meeting", "poker"]);
    project.decks[0].auto_hide_ms = 0;
    let mut engine = engine(project);
    raise(&mut engine, "meeting", "agree", 0);
    assert_eq!(engine.vote("meeting", 0), None);
    engine.set_room_round(Some("meeting"));
    assert_eq!(engine.vote("meeting", 0), None);
    assert!(!engine.tick(VOTE_WINDOW_MS).changed, "nothing ran out");
}
