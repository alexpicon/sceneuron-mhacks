// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises how the room client tells the engine that the bound deck's round is open, so finger
//! counts go to the vote: joining an open round opens it, the reveal ends it even though it raises
//! the counted vote, the next round opens it again, and leaving or losing the connection ends it.

mod common;

use common::{FakeRooms, Person, person, request};
use sceneuron_core::VoteBy;
use sceneuron_server::RoomAction;

/// What holds the poker deck's vote open, if anything.
fn vote(person: &Person) -> Option<VoteBy> {
    person.hub.read(|engine| engine.vote("poker", 0).map(|vote| vote.by))
}

#[test]
fn the_rooms_open_round_holds_the_vote_until_the_reveal_and_the_next_round_opens_it_again() {
    let rooms = FakeRooms::new();
    let (ana, ben) = (person(&rooms), person(&rooms));
    assert_eq!(vote(&ana), None);
    let code = ana.create("Ana");
    assert_eq!(vote(&ana), Some(VoteBy::Room));
    ana.raise("p3");
    assert_eq!(vote(&ana), Some(VoteBy::Room), "the round, not the press, holds it");
    ana.act(request(RoomAction::Reveal));
    assert_eq!(ana.raised().as_deref(), Some("p3"), "the reveal shows the vote");
    assert_eq!(
        vote(&ana),
        None,
        "the reveal ends the vote, and showing the card opens none"
    );
    ben.join(&code, "Ben");
    assert_eq!(vote(&ben), None, "a room joined after its reveal has no open round");
    ana.act(request(RoomAction::Next));
    assert_eq!((vote(&ana), vote(&ben)), (Some(VoteBy::Room), Some(VoteBy::Room)));
}

#[test]
fn leaving_or_losing_the_room_ends_its_vote() {
    let rooms = FakeRooms::new();
    let ana = person(&rooms);
    ana.create("Ana");
    ana.act(request(RoomAction::Leave));
    assert_eq!(vote(&ana), None);
    ana.create("Ana");
    assert_eq!(vote(&ana), Some(VoteBy::Room));
    rooms.drop_connection(&rooms.connection_ids()[1]);
    assert_eq!(ana.state()["joined"], false);
    assert_eq!(vote(&ana), None);
}
