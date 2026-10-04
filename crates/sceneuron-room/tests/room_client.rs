// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the room client against the fake rooms with two participants: raising a card votes
//! and covers it, others see who voted but not what, a reveal from anyone shows each person's own
//! card, the next round lowers them, the `{{room}}` summary, and leaving.

mod common;

use common::{FakeRooms, person, request};
use sceneuron_core::{CardAction, Outcome};
use sceneuron_room::COVER_TEXT;
use sceneuron_server::{RoomAction, RoomRequest};
use serde_json::{Value, json};

fn voted(state: &Value) -> Vec<(String, bool)> {
    state["participants"]
        .as_array()
        .unwrap()
        .iter()
        .map(|person| (person["name"].as_str().unwrap().to_owned(), person["voted"] == true))
        .collect()
}

#[test]
fn joining_starts_a_covered_round_without_connecting_before() {
    let rooms = FakeRooms::new();
    let ana = person(&rooms);
    ana.raise("p13");
    assert!(
        rooms.connection_ids().is_empty(),
        "nothing connects until someone joins"
    );
    let code = ana.create("Ana");
    let state = ana.state();
    assert_eq!(
        (state["joined"].clone(), state["round"].clone()),
        (json!(true), json!(1))
    );
    assert_eq!(state["deck_name"], "Planning poker");
    assert_eq!(ana.raised(), None, "joining starts the round with the deck lowered");
    assert!(ana.covered(), "the next card raised is covered");
    let ben = person(&rooms);
    let joined = ben.join(&code.to_lowercase(), " Ben ");
    assert_eq!(joined["code"], code.as_str());
    assert_eq!(voted(&joined), [("Ana".to_owned(), false), ("Ben".to_owned(), false)]);
    assert_eq!(ana.summary(), "Voted: 0 of 2");
}

#[test]
fn raising_a_card_casts_a_hidden_vote() {
    let rooms = FakeRooms::new();
    let (ana, ben) = (person(&rooms), person(&rooms));
    let code = ana.create("Ana");
    ben.join(&code, "Ben");
    ana.raise("p3");
    assert_eq!(
        ana.slot(),
        (COVER_TEXT.to_owned(), true),
        "the overlay never shows the 3"
    );
    let ids = rooms.connection_ids();
    assert_eq!(rooms.vote_of(&ids[0]).as_deref(), Some("p3"));
    let seen_by_ben = ben.state();
    assert_eq!(
        voted(&seen_by_ben),
        [("Ana".to_owned(), true), ("Ben".to_owned(), false)]
    );
    assert!(!seen_by_ben.to_string().contains("p3"), "{seen_by_ben}");
    assert_eq!(seen_by_ben["my_vote"], Value::Null);
    assert_eq!(ana.state()["my_vote"], json!({"card": "p3", "label": "3"}));
    assert_eq!(
        (ana.summary(), ben.summary()),
        ("Voted: 1 of 2".to_owned(), "Voted: 1 of 2".to_owned())
    );
}

#[test]
fn raising_another_card_replaces_the_vote_and_stays_covered() {
    let rooms = FakeRooms::new();
    let ana = person(&rooms);
    ana.create("Ana");
    ana.raise("p3");
    ana.raise("p8");
    assert_eq!(ana.slot().0, COVER_TEXT);
    assert_eq!(rooms.vote_of(&rooms.connection_ids()[0]).as_deref(), Some("p8"));
    ana.lower();
    assert!(ana.covered(), "a lowered deck stays covered while the round is open");
    ana.raise("p5");
    assert_eq!(ana.slot().0, COVER_TEXT);
    assert_eq!(rooms.vote_of(&rooms.connection_ids()[0]).as_deref(), Some("p5"));
}

#[test]
fn a_card_raised_in_the_same_change_as_a_lower_is_covered() {
    let rooms = FakeRooms::new();
    let ana = person(&rooms);
    ana.create("Ana");
    ana.raise("p3");
    let mut shown = String::new();
    ana.hub.update(|engine, now| {
        let lowered = engine.card("poker", None, CardAction::Lower, now).unwrap();
        let raised = engine.card("poker", Some("p8"), CardAction::Raise, now).unwrap();
        // Read before any listener, such as the room client, sees the change.
        shown = engine.effective_props("poker-card").unwrap().text;
        Outcome {
            changed: lowered.changed || raised.changed,
            ..Outcome::default()
        }
    });
    assert_eq!(shown, COVER_TEXT, "the change that raises the card already covers it");
}

#[test]
fn a_reveal_shows_each_participant_their_own_card() {
    let rooms = FakeRooms::new();
    let (ana, ben, cleo) = (person(&rooms), person(&rooms), person(&rooms));
    let code = ana.create("Ana");
    ben.join(&code, "Ben");
    cleo.join(&code, "Cleo");
    ana.raise("p3");
    ben.raise("p8");
    ana.lower();
    let revealed = ben.act(request(RoomAction::Reveal));
    assert_eq!(revealed["revealed"], true);
    assert_eq!(
        ana.slot(),
        ("3".to_owned(), true),
        "a vote shows even after its card was lowered"
    );
    assert_eq!(ben.slot(), ("8".to_owned(), true));
    assert_eq!(cleo.raised(), None, "nobody's card shows for someone who did not vote");
    assert_eq!(ana.summary(), "You 3 · Ben 8 · Average 5.5");
    assert_eq!(ben.summary(), "Ana 3 · You 8 · Average 5.5");
    assert_eq!(cleo.summary(), "Ana 3 · Ben 8 · Average 5.5");
    for person in [&ana, &ben, &cleo] {
        assert!(!person.covered());
    }
    let results = &cleo.state()["results"];
    assert_eq!(
        results,
        &json!([
            {"id": "client-1", "name": "Ana", "card": "p3", "label": "3", "me": false},
            {"id": "client-2", "name": "Ben", "card": "p8", "label": "8", "me": false}
        ])
    );
    let people = cleo.state()["participants"].clone();
    let keys: Vec<&Value> = people.as_array().unwrap().iter().map(|person| &person["id"]).collect();
    assert_eq!(keys, [&json!("client-1"), &json!("client-2"), &json!("client-3")]);
}

#[test]
fn after_the_reveal_cards_show_normally_and_cast_no_vote() {
    let rooms = FakeRooms::new();
    let ana = person(&rooms);
    ana.create("Ana");
    ana.raise("p2");
    ana.act(request(RoomAction::Reveal));
    ana.raise("p21");
    assert_eq!(ana.slot().0, "21");
    assert_eq!(rooms.vote_of(&rooms.connection_ids()[0]).as_deref(), Some("p2"));
    assert_eq!(ana.state()["error"], Value::Null);
}

#[test]
fn the_next_round_lowers_every_card_and_covers_the_deck_again() {
    let rooms = FakeRooms::new();
    let (ana, ben) = (person(&rooms), person(&rooms));
    let code = ana.create("Ana");
    ben.join(&code, "Ben");
    ana.raise("p3");
    ben.raise("p5");
    ana.act(request(RoomAction::Reveal));
    let next = ben.act(request(RoomAction::Next));
    assert_eq!(
        (next["round"].clone(), next["revealed"].clone()),
        (json!(2), json!(false))
    );
    for person in [&ana, &ben] {
        assert_eq!(person.raised(), None);
        assert!(person.covered());
        assert_eq!(person.summary(), "Voted: 0 of 2");
    }
    assert_eq!(rooms.vote_of(&rooms.connection_ids()[0]), None);
    ana.raise("p1");
    assert_eq!(ana.slot().0, COVER_TEXT, "the new round votes again");
    assert_eq!(ben.summary(), "Voted: 1 of 2");
}

#[test]
fn leaving_lowers_a_covered_card_and_clears_the_summary() {
    let rooms = FakeRooms::new();
    let (ana, ben) = (person(&rooms), person(&rooms));
    let code = ana.create("Ana");
    ben.join(&code, "Ben");
    ana.raise("p13");
    let left = ana.act(request(RoomAction::Leave));
    assert_eq!(left["joined"], false);
    assert_eq!(ana.raised(), None, "leaving never shows a hidden vote");
    assert!(!ana.covered());
    assert_eq!(ana.summary(), "");
    assert_eq!(voted(&ben.state()), [("Ben".to_owned(), false)]);
    ana.raise("p8");
    assert_eq!(ana.slot().0, "8", "outside a room, cards show as usual");
}

#[test]
fn a_lost_connection_leaves_the_room_and_says_why() {
    let rooms = FakeRooms::new();
    let ana = person(&rooms);
    ana.create("Ana");
    ana.raise("p5");
    rooms.drop_connection(&rooms.connection_ids()[0]);
    let state = ana.state();
    assert_eq!(state["joined"], false);
    assert_eq!(state["error"], "lost the connection to the room: the server went away");
    assert_eq!(ana.raised(), None);
    assert_eq!(ana.summary(), "");
}

#[test]
fn the_state_offers_the_last_settings_again() {
    let rooms = FakeRooms::new();
    let ana = person(&rooms);
    let fresh = ana.state();
    assert_eq!(
        (&fresh["server"], &fresh["database"], &fresh["deck"], &fresh["joined"]),
        (
            &json!("http://127.0.0.1:3000"),
            &json!("sceneuron-room"),
            &json!("poker"),
            &json!(false)
        )
    );
    ana.act(RoomRequest {
        name: Some("Ana".to_owned()),
        server: Some("http://127.0.0.1:3005/".to_owned()),
        database: Some("team-room".to_owned()),
        ..request(RoomAction::Create)
    });
    ana.act(request(RoomAction::Leave));
    let state = ana.state();
    assert_eq!(
        (&state["server"], &state["database"], &state["name"]),
        (&json!("http://127.0.0.1:3005"), &json!("team-room"), &json!("Ana"))
    );
}
