// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the room actions' refusals and failures: missing or invalid fields, a deck the
//! project lacks, wrong codes, acting outside a room or twice, an unreachable server, and a
//! server that never answers.

mod common;

use std::sync::atomic::Ordering;

use common::{FakeRooms, person, request};
use sceneuron_room::COVER_TEXT;
use sceneuron_server::{RoomAction, RoomRequest};

fn create(name: &str) -> RoomRequest {
    RoomRequest {
        name: Some(name.to_owned()),
        ..request(RoomAction::Create)
    }
}

#[test]
fn fields_are_checked_before_connecting() {
    let rooms = FakeRooms::new();
    let ana = person(&rooms);
    let refusals = [
        (create("  "), "enter a display name of 1 to 40 characters"),
        (create(&"n".repeat(41)), "enter a display name of 1 to 40 characters"),
        (
            RoomRequest {
                deck: Some("status".to_owned()),
                ..create("Ana")
            },
            "this project has no saved deck \"status\"; add the Planning poker pack and save",
        ),
        (
            RoomRequest {
                code: Some("K7-2".to_owned()),
                name: Some("Ana".to_owned()),
                ..request(RoomAction::Join)
            },
            "enter the room's code: 4 to 12 letters and digits",
        ),
    ];
    for (request, message) in refusals {
        let error = ana.try_act(request).unwrap_err();
        assert_eq!((error.status, error.message.as_str()), (400, message));
    }
    let server = RoomRequest {
        server: Some("ftp://example.com".to_owned()),
        ..create("Ana")
    };
    assert_eq!(ana.try_act(server).unwrap_err().status, 400);
    assert!(rooms.connection_ids().is_empty());
}

#[test]
fn the_rooms_refusal_reaches_the_editor_and_leaves_no_session() {
    let rooms = FakeRooms::new();
    let ana = person(&rooms);
    let error = ana
        .try_act(RoomRequest {
            code: Some("ZZZZ99".to_owned()),
            name: Some("Ana".to_owned()),
            ..request(RoomAction::Join)
        })
        .unwrap_err();
    assert_eq!(
        (error.status, error.message.as_str()),
        (400, "no room has the code ZZZZ99")
    );
    assert_eq!(ana.state()["joined"], false);
    assert!(!ana.covered());
    ana.create("Ana");
}

#[test]
fn actions_need_a_room_and_one_room_at_a_time() {
    let rooms = FakeRooms::new();
    let ana = person(&rooms);
    for action in [RoomAction::Reveal, RoomAction::Next, RoomAction::Leave] {
        assert_eq!(ana.try_act(request(action)).unwrap_err().status, 409, "{action:?}");
    }
    let code = ana.create("Ana");
    let again = ana.try_act(create("Ana")).unwrap_err();
    assert_eq!(again.status, 409);
    assert_eq!(
        again.message,
        format!("this Sceneuron is in room {code}; leave it first")
    );
}

#[test]
fn an_unreachable_server_is_a_gateway_error() {
    let rooms = FakeRooms::new();
    let ana = person(&rooms);
    *rooms.refuse_connect.lock().unwrap() = Some("connection refused".to_owned());
    let error = ana.try_act(create("Ana")).unwrap_err();
    assert_eq!(error.status, 502);
    assert_eq!(
        error.message,
        "cannot reach the rooms at http://127.0.0.1:3000: connection refused"
    );
    assert_eq!(ana.state()["joined"], false);
}

#[test]
fn a_server_that_never_answers_times_out_and_leaves_no_session() {
    let rooms = FakeRooms::new();
    let ana = person(&rooms);
    rooms.silent.store(true, Ordering::SeqCst);
    let error = ana.try_act(create("Ana")).unwrap_err();
    assert_eq!(error.status, 504);
    assert_eq!(ana.state()["joined"], false);
    rooms.silent.store(false, Ordering::SeqCst);
    ana.create("Ana");
    rooms.silent.store(true, Ordering::SeqCst);
    assert_eq!(ana.try_act(request(RoomAction::Reveal)).unwrap_err().status, 504);
    assert_eq!(ana.state()["joined"], true, "a slow reveal keeps the room");
}

#[test]
fn a_vote_without_an_answer_never_shows_at_the_reveal() {
    let rooms = FakeRooms::new();
    let (ana, ben) = (person(&rooms), person(&rooms));
    let code = ana.create("Ana");
    ben.join(&code, "Ben");
    ana.raise("p3");
    rooms.silent.store(true, Ordering::SeqCst);
    ana.raise("p5");
    rooms.silent.store(false, Ordering::SeqCst);
    assert_eq!(ana.slot().0, COVER_TEXT);
    ben.act(request(RoomAction::Reveal));
    assert_eq!(
        ana.slot().0,
        "3",
        "the counted vote shows, not the card whose vote got lost"
    );
}

#[test]
fn a_refused_vote_is_reported_once_and_not_sent_again() {
    let rooms = FakeRooms::new();
    let ana = person(&rooms);
    ana.create("Ana");
    *rooms.refuse_call.lock().unwrap() = Some("the room is full".to_owned());
    ana.raise("p5");
    let message = "the vote for p5 was not counted: the room is full";
    assert_eq!(ana.state()["error"], message);
    assert_eq!(rooms.vote_of(&rooms.connection_ids()[0]), None);
    let events = ana.hub.snapshot().events;
    let notes: Vec<_> = events.iter().filter(|event| event.source == "room").collect();
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert_eq!(notes[0].detail, message);
    ana.raise("p8");
    assert_eq!(
        rooms.vote_of(&rooms.connection_ids()[0]).as_deref(),
        Some("p8"),
        "another card votes again"
    );
}
