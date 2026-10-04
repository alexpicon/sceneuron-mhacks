// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises reading the text protocol and keeping the copy of the tables, with messages recorded
//! from SpacetimeDB 2.10.2 running `room/module`: the identity, the first look at the subscription
//! with rows by field name, transaction updates with rows by position, the caller's own answers,
//! refusals, and rows this client cannot read.

use serde_json::json;

use super::cache::Cache;
use super::protocol::{Message, TableRows, call_reducer, identity, parse, subscribe};
use crate::backend::{CallError, Member, Shown};

const ANA: &str = "c20002be6f41d29b2ff483a43fb19ab53d86eae569c224497d2707eaab4ffcbf";
const BEN: &str = "c2006005700b59274b6b83b451d9e1efecd2b01afb10256af1cbf35b85ae0782";

const IDENTITY: &str = r#"{"IdentityToken":{"identity":{"__identity__":"0xc20002be6f41d29b2ff483a43fb19ab53d86eae569c224497d2707eaab4ffcbf"},"token":"eyJ0","connection_id":{"__connection_id__":146823508348706379001023255544889765176}}}"#;
const SUBSCRIBED: &str = r#"{"InitialSubscription":{"database_update":{"tables":[{"table_id":4099,"table_name":"room","num_rows":1,"updates":[{"deletes":[],"inserts":["{\"code\":\"2HV39A\",\"deck_name\":\"Planning poker\",\"round\":1,\"revealed\":false,\"created_at\":{\"__timestamp_micros_since_unix_epoch__\":1790974995748166}}"]}]},{"table_id":4097,"table_name":"participant","num_rows":1,"updates":[{"deletes":[],"inserts":["{\"identity\":{\"__identity__\":\"0xc2006005700b59274b6b83b451d9e1efecd2b01afb10256af1cbf35b85ae0782\"},\"room\":\"2HV39A\",\"name\":\"Ben\",\"joined_at\":{\"__timestamp_micros_since_unix_epoch__\":1790974995748166}}"]}]}]},"request_id":1,"total_host_execution_duration":{"__time_duration_micros__":2572}}}"#;
const JOINED: &str = r#"{"TransactionUpdate":{"status":{"Committed":{"tables":[{"table_id":4097,"table_name":"participant","num_rows":1,"updates":[{"deletes":[],"inserts":["[[\"0xc20002be6f41d29b2ff483a43fb19ab53d86eae569c224497d2707eaab4ffcbf\"],\"2HV39A\",\"Ana\",[1790974996000000]]"]}]}]}},"timestamp":{"__timestamp_micros_since_unix_epoch__":1790974996000000},"caller_identity":{"__identity__":"0xc20002be6f41d29b2ff483a43fb19ab53d86eae569c224497d2707eaab4ffcbf"},"caller_connection_id":{"__connection_id__":146823508348706379001023255544889765176},"reducer_call":{"reducer_name":"join_room","reducer_id":3,"args":"[\"2HV39A\",\"Ana\"]","request_id":2},"energy_quanta_used":{"quanta":20091},"total_host_execution_duration":{"__time_duration_micros__":1281}}}"#;
const BEN_VOTED: &str = r#"{"TransactionUpdateLight":{"request_id":3,"update":{"tables":[{"table_id":4096,"table_name":"ballot","num_rows":1,"updates":[{"deletes":[],"inserts":["[[\"0xc2006005700b59274b6b83b451d9e1efecd2b01afb10256af1cbf35b85ae0782\"],\"2HV39A\",1]"]}]}]}}}"#;
const REVEALED: &str = r#"{"TransactionUpdateLight":{"request_id":5,"update":{"tables":[{"table_id":4098,"table_name":"result","num_rows":1,"updates":[{"deletes":[],"inserts":["[[\"0xc2006005700b59274b6b83b451d9e1efecd2b01afb10256af1cbf35b85ae0782\"],\"2HV39A\",1,\"p8\"]"]}]},{"table_id":4099,"table_name":"room","num_rows":2,"updates":[{"deletes":["[\"2HV39A\",\"Planning poker\",1,false,[1790974995748166]]"],"inserts":["[\"2HV39A\",\"Planning poker\",1,true,[1790974995748166]]"]}]}]}}}"#;
const REFUSED: &str = r#"{"TransactionUpdate":{"status":{"Failed":"the cards are already revealed; start the next round to vote again"},"timestamp":{"__timestamp_micros_since_unix_epoch__":1790974978722068},"caller_identity":{"__identity__":"0xc20002be6f41d29b2ff483a43fb19ab53d86eae569c224497d2707eaab4ffcbf"},"caller_connection_id":{"__connection_id__":146823508348706379001023255544889765176},"reducer_call":{"reducer_name":"cast_vote","reducer_id":0,"args":"[\"p5\"]","request_id":6},"energy_quanta_used":{"quanta":5914},"total_host_execution_duration":{"__time_duration_micros__":52}}}"#;

fn tables(message: Message) -> Vec<TableRows> {
    match message {
        Message::Subscribed(tables) | Message::Transaction { tables, .. } => tables,
        other => panic!("no tables in {other:?}"),
    }
}

#[test]
fn requests_are_sats_json_with_arguments_as_text() {
    let call: serde_json::Value =
        serde_json::from_str(&call_reducer("join_room", &json!(["2HV39A", "Ana"]), 7)).unwrap();
    assert_eq!(
        call,
        json!({"CallReducer": {"reducer": "join_room", "args": "[\"2HV39A\",\"Ana\"]", "request_id": 7, "flags": 0}})
    );
    let subscription: serde_json::Value = serde_json::from_str(&subscribe(&["SELECT * FROM room"], 1)).unwrap();
    assert_eq!(
        subscription,
        json!({"Subscribe": {"query_strings": ["SELECT * FROM room"], "request_id": 1}})
    );
}

#[test]
fn the_identity_comes_first_in_lowercase_hex() {
    assert_eq!(parse(IDENTITY, None), Ok(Message::Identity(ANA.to_owned())));
    assert_eq!(identity(&json!(["0xABC"])), Some(format!("{:0>64}", "abc")));
    assert_eq!(
        identity(&json!({"__identity__": "0x12"})),
        Some(format!("{:0>64}", "12"))
    );
    for wrong in [
        json!("0x12"),
        json!(["zz"]),
        json!([12]),
        json!(["0x"]),
        json!([format!("0x{}", "a".repeat(65))]),
    ] {
        assert_eq!(identity(&wrong), None, "{wrong}");
    }
}

#[test]
fn the_callers_own_update_completes_its_call_and_others_do_not() {
    let Message::Transaction { call, .. } = parse(JOINED, Some(ANA)).unwrap() else {
        panic!("not a transaction");
    };
    assert_eq!(call, Some((2, Ok(()))));
    let Message::Transaction { call, .. } = parse(JOINED, Some(BEN)).unwrap() else {
        panic!("not a transaction");
    };
    assert_eq!(
        call, None,
        "an update for another identity answers none of this connection's calls"
    );
    let refused = parse(REFUSED, Some(ANA)).unwrap();
    let reason = "the cards are already revealed; start the next round to vote again".to_owned();
    assert_eq!(
        refused,
        Message::Transaction {
            tables: Vec::new(),
            call: Some((6, Err(CallError::Refused(reason)))),
        }
    );
    let out_of_energy = r#"{"TransactionUpdate":{"status":{"OutOfEnergy":[]},"caller_identity":{"__identity__":"0x01"},"reducer_call":{"request_id":9}}}"#;
    let Message::Transaction { call, .. } = parse(out_of_energy, Some(&format!("{:0>64}", "1"))).unwrap() else {
        panic!("not a transaction");
    };
    assert_eq!(
        call,
        Some((9, Err(CallError::Failed("the call ended as OutOfEnergy".to_owned()))))
    );
}

#[test]
fn errors_and_messages_the_room_does_not_use() {
    let error = r#"{"SubscriptionError":{"request_id":{"some":1},"error":"no such table: `vote`"}}"#;
    assert_eq!(
        parse(error, None),
        Ok(Message::SubscriptionFailed("no such table: `vote`".to_owned()))
    );
    let answer = r#"{"OneOffQueryResponse":{"message_id":"03","error":{"none":[]},"tables":[]}}"#;
    assert_eq!(parse(answer, None), Ok(Message::Other));
    for broken in ["", "[]", r#"{"a":1,"b":2}"#, r#"{"InitialSubscription":{}}"#] {
        assert!(parse(broken, None).is_err(), "{broken}");
    }
}

#[test]
fn the_cache_follows_rows_by_name_and_by_position_to_the_room_view() {
    let mut cache = Cache::default();
    cache.apply(&tables(parse(SUBSCRIBED, None).unwrap())).unwrap();
    assert_eq!(cache.view_of(ANA), None, "Ana is in no room yet");
    cache.apply(&tables(parse(JOINED, Some(ANA)).unwrap())).unwrap();
    cache.apply(&tables(parse(BEN_VOTED, Some(ANA)).unwrap())).unwrap();
    let view = cache.view_of(ANA).unwrap();
    assert_eq!(
        (view.code.as_str(), view.deck_name.as_str(), view.round, view.revealed),
        ("2HV39A", "Planning poker", 1, false)
    );
    let member = |id: &str, name: &str, voted: bool, me: bool| Member {
        id: id.to_owned(),
        name: name.to_owned(),
        voted,
        me,
    };
    assert_eq!(
        view.members,
        [member(BEN, "Ben", true, false), member(ANA, "Ana", false, true)],
        "in the order they joined"
    );
    assert!(view.results.is_empty());
    cache.apply(&tables(parse(REVEALED, Some(ANA)).unwrap())).unwrap();
    let view = cache.view_of(ANA).unwrap();
    assert!(view.revealed, "the room's row was replaced");
    let shown = Shown {
        id: BEN.to_owned(),
        card: "p8".to_owned(),
    };
    assert_eq!(view.results, [shown]);
}

#[test]
fn rows_of_an_unexpected_shape_are_reported() {
    let mut cache = Cache::default();
    let rows = TableRows {
        table: "room".to_owned(),
        inserts: vec![json!(["2HV39A", "Planning poker", "one", false, [0]])],
        ..TableRows::default()
    };
    let error = cache.apply(&[rows]).unwrap_err();
    assert!(error.starts_with("a room row has an unexpected shape"), "{error}");
    let other = TableRows {
        table: "vote".to_owned(),
        inserts: vec![json!("anything")],
        ..TableRows::default()
    };
    assert_eq!(
        cache.apply(&[other]),
        Ok(()),
        "tables the room does not use are skipped"
    );
}
