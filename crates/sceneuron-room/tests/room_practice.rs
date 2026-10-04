// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises a room with a chosen code and practice teammates: opening CHASKI creates it once and
//! joins it after, practice brings four teammates in as their own connections, starting it twice
//! adds no more, and stopping it or leaving takes them out again.

mod common;

use common::{FakeRooms, person, request};
use sceneuron_server::{RoomAction, RoomRequest};
use serde_json::Value;

fn open(name: &str) -> RoomRequest {
    RoomRequest {
        code: Some("chaski".to_owned()),
        name: Some(name.to_owned()),
        ..request(RoomAction::Open)
    }
}

fn names(state: &Value) -> Vec<String> {
    state["participants"]
        .as_array()
        .unwrap()
        .iter()
        .map(|person| person["name"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn opening_a_chosen_code_creates_the_room_once_and_joins_it_after() {
    let rooms = FakeRooms::new();
    let (ana, ben) = (person(&rooms), person(&rooms));
    let first = ana.act(open("Ana"));
    assert_eq!(
        (first["code"].as_str(), first["joined"].as_bool()),
        (Some("CHASKI"), Some(true))
    );
    let second = ben.act(open("Ben"));
    assert_eq!(second["code"], "CHASKI");
    assert_eq!(names(&second), ["Ana", "Ben"]);
}

#[test]
fn practice_brings_four_teammates_and_stopping_takes_them_out() {
    let rooms = FakeRooms::new();
    let ana = person(&rooms);
    let refused = ana.try_act(request(RoomAction::Practice)).unwrap_err();
    assert_eq!((refused.status, refused.message.as_str()), (409, "join a room first"));
    ana.act(open("Ana"));
    let state = ana.act(request(RoomAction::Practice));
    assert_eq!(state["practice"], true);
    let joined = || names(&ana.state());
    common::wait_until(|| joined().len() == 5);
    assert_eq!(joined(), ["Ana", "Maya", "Sam", "Priya", "Leo"]);
    ana.act(request(RoomAction::Practice));
    assert_eq!(rooms.open_connections(), 5, "starting twice adds nobody");
    let stopped = ana.act(request(RoomAction::PracticeStop));
    assert_eq!(stopped["practice"], false);
    common::wait_until(|| joined() == ["Ana"]);
}

#[test]
fn leaving_the_room_takes_the_teammates_out_too() {
    let rooms = FakeRooms::new();
    let ana = person(&rooms);
    ana.act(open("Ana"));
    ana.act(request(RoomAction::Practice));
    ana.act(request(RoomAction::Leave));
    assert_eq!(ana.state()["practice"], false);
    common::wait_until(|| rooms.open_connections() == 0);
}

#[test]
fn a_room_that_already_has_the_teammates_gets_no_second_set() {
    let rooms = FakeRooms::new();
    let (ana, ben) = (person(&rooms), person(&rooms));
    ana.act(open("Ana"));
    ana.act(request(RoomAction::Practice));
    common::wait_until(|| names(&ana.state()).len() == 5);
    ben.act(open("Ben"));
    ben.act(request(RoomAction::Practice));
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert_eq!(names(&ben.state()), ["Ana", "Maya", "Sam", "Priya", "Leo", "Ben"]);
}
