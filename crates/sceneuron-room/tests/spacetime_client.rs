// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises Sceneuron's own SpacetimeDB client over a real WebSocket against a stand-in server:
//! subscribing, calling reducers with each answer after the room it changed, refusals, the server
//! closing, a server that goes silent and a quiet one that answers pings, a missing database, an
//! unreachable server, and closing while connecting.

mod wire;

use std::net::TcpListener;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use sceneuron_room::{BackendEvent, Call, CallError, Connector, RoomBackend, SpacetimeConnector, Target};
use serde_json::json;
use tungstenite::protocol::CloseFrame;
use tungstenite::protocol::frame::coding::CloseCode;
use wire::{ANA, BEN, answer, inserts, light, refuse, serve};

const WAIT: Duration = Duration::from_secs(5);

/// Connections that count as lost after an idle limit short enough for a test.
const QUICK: SpacetimeConnector = SpacetimeConnector {
    idle_limit: Duration::from_millis(300),
};

/// What the connection reported, in order: its events and the answers to calls.
#[derive(Debug, PartialEq)]
enum Seen {
    Event(BackendEvent),
    Answer(Result<(), CallError>),
}

fn connect(server: &str) -> (Arc<dyn RoomBackend>, Receiver<Seen>) {
    connect_with(SpacetimeConnector::default(), server)
}

fn connect_with(connector: SpacetimeConnector, server: &str) -> (Arc<dyn RoomBackend>, Receiver<Seen>) {
    let (seen, reports) = mpsc::channel();
    let target = Target::new(Some(server), Some("rooms")).unwrap();
    let events = Box::new(move |event| {
        let _ = seen.send(Seen::Event(event));
    });
    (connector.connect(&target, events).unwrap(), reports)
}

/// Sends `call` and waits for its answer; events the connection reported first are queued by then.
fn send(backend: &dyn RoomBackend, call: Call) -> Seen {
    let (answer, answered) = mpsc::channel();
    backend.send(
        call,
        Box::new(move |result| {
            let _ = answer.send(result);
        }),
    );
    Seen::Answer(answered.recv_timeout(WAIT).unwrap())
}

fn room_of(seen: Seen) -> Option<sceneuron_room::RoomView> {
    match seen {
        Seen::Event(BackendEvent::Room(view)) => view,
        other => panic!("not a room: {other:?}"),
    }
}

#[test]
fn it_follows_the_room_and_answers_each_call_after_the_room_changed() {
    let wire = serve(|peer| {
        peer.greet(ANA, vec![inserts("room", &[json!({"code": "OTHER2", "deck_name": "Poker", "round": 3, "revealed": false, "created_at": {"__timestamp_micros_since_unix_epoch__": 1}})])]);
        let create = peer.next();
        let call = create["CallReducer"].clone();
        assert_eq!(
            (call["reducer"].clone(), call["args"].clone(), call["flags"].clone()),
            (json!("create_room"), json!("[\"Planning poker\",\"Ana\"]"), json!(0))
        );
        let room = json!(["K7M2QX", "Planning poker", 1, false, [10]]);
        let ana = json!([[format!("0x{ANA}")], "K7M2QX", "Ana", [10]]);
        peer.send(&answer(
            ANA,
            &create,
            json!({"Committed": {"tables": [inserts("room", &[room]), inserts("participant", &[ana])]}}),
        ));
        let ben = json!([[format!("0x{BEN}")], "K7M2QX", "Ben", [20]]);
        let ballot = json!([[format!("0x{BEN}")], "K7M2QX", 1]);
        peer.send(&light(vec![
            inserts("participant", &[ben]),
            inserts("ballot", &[ballot]),
        ]));
        let vote = peer.next();
        assert_eq!(vote["CallReducer"]["args"], "[\"p5\"]");
        peer.send(&answer(ANA, &vote, json!({"Failed": "the cards are already revealed"})));
        peer.closing()
    });
    let (backend, reports) = connect(&wire.server);
    backend.ready(WAIT).unwrap();
    assert_eq!(
        room_of(reports.recv_timeout(WAIT).unwrap()),
        None,
        "a room Ana is not in is not hers"
    );
    let created = Call::Create {
        deck_name: "Planning poker".to_owned(),
        name: "Ana".to_owned(),
    };
    assert_eq!(send(backend.as_ref(), created), Seen::Answer(Ok(())));
    let view = room_of(reports.try_recv().expect("the room is reported before the answer")).unwrap();
    assert_eq!((view.code.as_str(), view.round, view.members.len()), ("K7M2QX", 1, 1));
    assert!(view.members[0].me && view.members[0].id == ANA);
    let view = room_of(reports.recv_timeout(WAIT).unwrap()).unwrap();
    let people: Vec<(&str, bool, bool)> = view.members.iter().map(|m| (m.name.as_str(), m.voted, m.me)).collect();
    assert_eq!(people, [("Ana", false, true), ("Ben", true, false)]);
    let vote = Call::Vote { card: "p5".to_owned() };
    let refused = CallError::Refused("the cards are already revealed".to_owned());
    assert_eq!(send(backend.as_ref(), vote), Seen::Answer(Err(refused)));
    backend.close();
    assert!(backend.join(WAIT), "the connection's thread ends");
    let goodbye = wire.finish().expect("a close frame");
    assert_eq!(goodbye.code, CloseCode::Normal);
}

#[test]
fn a_server_that_closes_is_reported_with_its_reason_and_calls_then_fail() {
    let wire = serve(|peer| {
        peer.greet(ANA, Vec::new());
        peer.close(CloseFrame {
            code: CloseCode::Error,
            reason: "the module panicked".into(),
        });
    });
    let (backend, reports) = connect(&wire.server);
    backend.ready(WAIT).unwrap();
    assert_eq!(room_of(reports.recv_timeout(WAIT).unwrap()), None);
    let lost = reports.recv_timeout(WAIT).unwrap();
    assert_eq!(
        lost,
        Seen::Event(BackendEvent::Disconnected(Some("the module panicked".to_owned())))
    );
    wire.finish();
    let failed = send(backend.as_ref(), Call::Reveal);
    assert!(matches!(failed, Seen::Answer(Err(CallError::Failed(_)))), "{failed:?}");
    assert!(backend.join(WAIT));
}

#[test]
fn a_server_that_goes_silent_is_lost_after_the_idle_limit_with_its_calls() {
    let (release, released) = mpsc::channel::<()>();
    let wire = serve(move |peer| {
        peer.greet(ANA, Vec::new());
        // Silent with its socket still open, as a server is once the network dropped this client.
        let _ = released.recv_timeout(WAIT);
    });
    let (backend, reports) = connect_with(QUICK, &wire.server);
    backend.ready(WAIT).unwrap();
    assert_eq!(room_of(reports.recv_timeout(WAIT).unwrap()), None);
    let vote = send(backend.as_ref(), Call::Vote { card: "p5".to_owned() });
    assert!(
        matches!(vote, Seen::Answer(Err(CallError::Failed(_)))),
        "a vote sent into the silence fails: {vote:?}"
    );
    let silence = Some("the server stopped answering".to_owned());
    assert_eq!(
        reports.recv_timeout(WAIT),
        Ok(Seen::Event(BackendEvent::Disconnected(silence)))
    );
    assert!(backend.join(WAIT));
    release.send(()).unwrap();
    wire.finish();
}

#[test]
fn a_quiet_server_that_answers_pings_keeps_the_connection() {
    let wire = serve(|peer| {
        peer.greet(ANA, Vec::new());
        peer.answer_pings()
    });
    let (backend, reports) = connect_with(QUICK, &wire.server);
    backend.ready(WAIT).unwrap();
    assert_eq!(room_of(reports.recv_timeout(WAIT).unwrap()), None);
    assert_eq!(
        reports.recv_timeout(QUICK.idle_limit * 3),
        Err(RecvTimeoutError::Timeout),
        "nothing is reported while the server only answers pings"
    );
    backend.close();
    assert!(backend.join(WAIT));
    let (pings, goodbye) = wire.finish();
    assert!(pings >= 3, "the client pinged the quiet server {pings} times");
    assert_eq!(goodbye.map(|frame| frame.code), Some(CloseCode::Normal));
}

#[test]
fn a_refused_subscription_or_a_missing_database_fails_connecting() {
    let wire = serve(|peer| {
        peer.send(&json!({"IdentityToken": {"identity": {"__identity__": format!("0x{ANA}")}, "token": "t"}}));
        let subscribe = peer.next();
        let error = "no such table: `room`. If the table exists, it may be marked private.";
        peer.send(&json!({"SubscriptionError": {"request_id": {"some": subscribe["Subscribe"]["request_id"]}, "error": error}}));
    });
    let (backend, _) = connect(&wire.server);
    let error = backend.ready(WAIT).unwrap_err();
    assert!(
        error.starts_with("cannot follow the rooms: no such table: `room`"),
        "{error}"
    );
    wire.finish();
    let missing = refuse("404 Not Found", "`rooms` not found");
    let (backend, _) = connect(&missing);
    assert_eq!(
        backend.ready(WAIT).unwrap_err(),
        "the server answered 404 Not Found: `rooms` not found"
    );
}

#[test]
fn an_unreachable_server_fails_and_closing_stops_a_server_that_never_answers() {
    let free = TcpListener::bind("127.0.0.1:0").unwrap();
    let nobody = format!("http://{}", free.local_addr().unwrap());
    drop(free);
    let (backend, _) = connect(&nobody);
    assert!(backend.ready(WAIT).is_err());
    let silent = TcpListener::bind("127.0.0.1:0").unwrap();
    let (backend, _) = connect(&format!("http://{}", silent.local_addr().unwrap()));
    assert!(
        backend.ready(Duration::from_millis(200)).is_err(),
        "no handshake, no rooms"
    );
    let started = Instant::now();
    backend.close();
    assert!(backend.join(Duration::from_secs(1)), "closing ends the attempt");
    assert!(started.elapsed() < Duration::from_secs(1));
    drop(silent);
}
