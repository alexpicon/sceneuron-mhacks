// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the room client under concurrency: a reveal while other changes keep reaching the
//! overlay always uncovers the card, shutting down while a connection is still being made leaves
//! no connection behind and nothing connects afterwards, and shutting down waits for connections
//! closed by leaving, by losing them, or by a create that failed.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use common::{FakeRooms, Person, person, request};
use sceneuron_core::PropsPatch;
use sceneuron_server::{Room, RoomAction, RoomRequest};

/// Enough reveals that a race hit about once in a thousand shows up nearly every run.
const ROUNDS: usize = 3000;

/// Threads that keep changing the overlay during the reveals.
const HAMMERS: usize = 3;

#[test]
fn a_reveal_while_the_overlay_keeps_changing_always_uncovers_the_card() {
    let rooms = FakeRooms::new();
    let (ana, ben) = (person(&rooms), person(&rooms));
    let code = ana.create("Ana");
    ben.join(&code, "Ben");
    let stop = Arc::new(AtomicBool::new(false));
    // Every change of Ana's overlay makes her room client look at the deck again, and with
    // several threads changing it, that look often waits for the overlay while a reveal runs.
    let busy: Vec<_> = (0..HAMMERS)
        .map(|thread| {
            let hub = Arc::clone(&ana.hub);
            let stop = Arc::clone(&stop);
            thread::spawn(move || {
                let mut count = 0u64;
                while !stop.load(Ordering::SeqCst) {
                    count += 1;
                    let patch = PropsPatch {
                        text: Some(format!("change {thread}-{count}")),
                        ..PropsPatch::default()
                    };
                    hub.try_update(|engine, now| engine.set_override("brb", &patch, now))
                        .unwrap();
                }
            })
        })
        .collect();
    let mut covered = 0;
    for _ in 0..ROUNDS {
        ana.raise("p3");
        ben.act(request(RoomAction::Reveal));
        // A late cover from a look at the deck that started before the reveal lands right away.
        thread::sleep(Duration::from_micros(200));
        if ana.covered() || ana.slot().0 != "3" {
            covered += 1;
        }
        ben.act(request(RoomAction::Next));
    }
    stop.store(true, Ordering::SeqCst);
    for thread in busy {
        thread.join().unwrap();
    }
    assert_eq!(covered, 0, "reveals out of {ROUNDS} that left Ana's card covered");
}

#[test]
fn shutting_down_while_a_connection_is_being_made_leaves_none_open() {
    let rooms = FakeRooms::new();
    let ana = person(&rooms);
    let create = || RoomRequest {
        name: Some("Ana".to_owned()),
        ..request(RoomAction::Create)
    };
    rooms.hold_ready.store(true, Ordering::SeqCst);
    let joining = {
        let client = Arc::clone(&ana.client);
        let create = create();
        thread::spawn(move || client.act(create))
    };
    let deadline = Instant::now() + Duration::from_secs(2);
    while rooms.waiting() == 0 && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        (rooms.connection_ids().len(), rooms.waiting()),
        (1, 1),
        "the connection is being made"
    );
    ana.client.shutdown();
    assert_eq!(
        rooms.open_connections(),
        0,
        "shutting down closed the connection being made before it returned"
    );
    rooms.hold_ready.store(false, Ordering::SeqCst);
    let refused = joining.join().unwrap().unwrap_err();
    assert_eq!(
        (refused.status, refused.message.as_str()),
        (503, "Sceneuron is shutting down")
    );
    assert_eq!(ana.state()["joined"], false);
    assert_eq!(
        rooms.open_connections(),
        0,
        "shutting down closed the connection being made"
    );
    let again = ana.try_act(create()).unwrap_err();
    assert_eq!(again.status, 503);
    assert_eq!(rooms.connection_ids().len(), 1, "nothing connects after shutting down");
}

/// Shuts `person`'s client down at once while the threads of a connection it closed have not
/// ended, and checks that shutting down waits for them.
fn shutting_down_waits(rooms: &FakeRooms, person: &Person, closed_by: &str) {
    let stopping = {
        let client = Arc::clone(&person.client);
        thread::spawn(move || client.shutdown())
    };
    thread::sleep(Duration::from_millis(100));
    let waited = !stopping.is_finished();
    // Released before asserting, so a regression fails instead of leaving threads held.
    rooms.hold_join.store(false, Ordering::SeqCst);
    stopping.join().unwrap();
    assert!(
        waited,
        "shutting down returned while the connection closed by {closed_by} was still ending"
    );
}

#[test]
fn shutting_down_waits_for_connections_closed_by_leaving_losing_them_or_failing_to_join() {
    let rooms = FakeRooms::new();
    let create = |name: &str| RoomRequest {
        name: Some(name.to_owned()),
        ..request(RoomAction::Create)
    };
    let ana = person(&rooms);
    ana.create("Ana");
    rooms.hold_join.store(true, Ordering::SeqCst);
    ana.act(request(RoomAction::Leave));
    shutting_down_waits(&rooms, &ana, "leaving");

    let ben = person(&rooms);
    ben.create("Ben");
    rooms.hold_join.store(true, Ordering::SeqCst);
    rooms.drop_connection(rooms.connection_ids().last().unwrap());
    assert_eq!(ben.state()["joined"], false, "the lost connection ended Ben's session");
    shutting_down_waits(&rooms, &ben, "losing it");

    let cleo = person(&rooms);
    rooms.hold_join.store(true, Ordering::SeqCst);
    rooms.hold_ready.store(true, Ordering::SeqCst);
    assert_eq!(
        cleo.try_act(create("Cleo")).unwrap_err().status,
        502,
        "the rooms never answered"
    );
    rooms.hold_ready.store(false, Ordering::SeqCst);
    shutting_down_waits(&rooms, &cleo, "a failed connect");

    let dan = person(&rooms);
    rooms.hold_join.store(true, Ordering::SeqCst);
    rooms.silent.store(true, Ordering::SeqCst);
    assert_eq!(
        dan.try_act(create("Dan")).unwrap_err().status,
        504,
        "the create was never answered"
    );
    rooms.silent.store(false, Ordering::SeqCst);
    shutting_down_waits(&rooms, &dan, "a failed create");

    let eve = person(&rooms);
    eve.create("Eve");
    rooms.hold_join.store(true, Ordering::SeqCst);
    rooms.remove_from_room(rooms.connection_ids().last().unwrap());
    assert_eq!(
        eve.state()["joined"],
        false,
        "leaving the room on the server ended Eve's session"
    );
    shutting_down_waits(&rooms, &eve, "no longer being in the room");
}
