// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! SpacetimeDB's WebSocket text protocol, `v1.json.spacetimedb`, as far as the room uses it: the
//! messages this client sends, subscribing to tables and calling reducers, and the ones it reads,
//! its identity, the subscribed rows, and each transaction's changes with the outcome of this
//! connection's own calls. Messages are SATS-JSON, where a sum is an object whose one key names
//! the variant.

use serde_json::{Value, json};

use crate::backend::CallError;

/// The subprotocol a client asks for to speak SATS-JSON text.
pub(super) const PROTOCOL: &str = "v1.json.spacetimedb";

/// The reducer-call flags that ask for the caller's full update, which carries the call's outcome.
const FULL_UPDATE: u8 = 0;

/// A request to follow every row the `queries` select, answered by [`Message::Subscribed`].
pub(super) fn subscribe(queries: &[&str], request_id: u32) -> String {
    json!({ "Subscribe": { "query_strings": queries, "request_id": request_id } }).to_string()
}

/// A request to run `reducer` with `args`, a JSON array, answered by [`Message::Transaction`].
pub(super) fn call_reducer(reducer: &str, args: &Value, request_id: u32) -> String {
    let call = json!({
        "reducer": reducer,
        // The arguments travel as JSON text inside the message.
        "args": args.to_string(),
        "request_id": request_id,
        "flags": FULL_UPDATE,
    });
    json!({ "CallReducer": call }).to_string()
}

/// What the server sent, as far as the room uses it.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Message {
    /// The identity of this connection, in 64 lowercase hex digits.
    Identity(String),
    /// Every row the subscription selects, once.
    Subscribed(Vec<TableRows>),
    /// The subscription was refused.
    SubscriptionFailed(String),
    /// A transaction changed subscribed rows. `call` holds the request id and outcome when the
    /// transaction is a call this connection made.
    Transaction {
        tables: Vec<TableRows>,
        call: Option<(u32, Result<(), CallError>)>,
    },
    /// Something the room does not use, such as the answer to a one-off query.
    Other,
}

/// The rows of one table that a message deletes and inserts, as SATS-JSON product values.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct TableRows {
    pub(super) table: String,
    pub(super) deletes: Vec<Value>,
    pub(super) inserts: Vec<Value>,
}

/// Reads one text message. `me` is this connection's identity once known, which tells this
/// connection's own calls from others'.
pub(super) fn parse(text: &str, me: Option<&str>) -> Result<Message, String> {
    let value: Value = serde_json::from_str(text).map_err(|error| format!("a message is not JSON: {error}"))?;
    let (kind, body) = value
        .as_object()
        .filter(|object| object.len() == 1)
        .and_then(|object| object.iter().next())
        .ok_or_else(|| "a message is not one tagged variant".to_owned())?;
    Ok(match kind.as_str() {
        "IdentityToken" => Message::Identity(identity(&body["identity"]).ok_or("an identity message has no identity")?),
        "InitialSubscription" => Message::Subscribed(tables(&body["database_update"])?),
        "SubscriptionError" => Message::SubscriptionFailed(body["error"].as_str().unwrap_or("no reason").to_owned()),
        "TransactionUpdateLight" => Message::Transaction {
            tables: tables(&body["update"])?,
            call: None,
        },
        "TransactionUpdate" => transaction(body, me)?,
        _ => Message::Other,
    })
}

/// A full transaction update, which the server sends to the connection that called the reducer.
fn transaction(body: &Value, me: Option<&str>) -> Result<Message, String> {
    let status = &body["status"];
    let (tables, outcome) = if let Some(committed) = status.get("Committed") {
        (tables(committed)?, Ok(()))
    } else if let Some(reason) = status.get("Failed") {
        let reason = reason.as_str().unwrap_or("the reducer failed").to_owned();
        (Vec::new(), Err(CallError::Refused(reason)))
    } else {
        let what = status.as_object().and_then(|object| object.keys().next().cloned());
        let reason = format!("the call ended as {}", what.as_deref().unwrap_or("unknown"));
        (Vec::new(), Err(CallError::Failed(reason)))
    };
    let mine = me.is_some() && identity(&body["caller_identity"]).as_deref() == me;
    let request = body["reducer_call"]["request_id"]
        .as_u64()
        .and_then(|id| u32::try_from(id).ok());
    let call = request.filter(|_| mine).map(|id| (id, outcome));
    Ok(Message::Transaction { tables, call })
}

/// The tables of a database update: `{"tables": [{"table_name", "updates": [{"deletes",
/// "inserts"}]}]}`, where each row is the JSON text of a product value.
fn tables(update: &Value) -> Result<Vec<TableRows>, String> {
    let Some(tables) = update["tables"].as_array() else {
        return Err("an update has no tables".to_owned());
    };
    let mut found = Vec::with_capacity(tables.len());
    for table in tables {
        let mut rows = TableRows {
            table: table["table_name"].as_str().unwrap_or_default().to_owned(),
            ..TableRows::default()
        };
        for update in table["updates"].as_array().into_iter().flatten() {
            rows.deletes.extend(row_values(&update["deletes"])?);
            rows.inserts.extend(row_values(&update["inserts"])?);
        }
        found.push(rows);
    }
    Ok(found)
}

fn row_values(rows: &Value) -> Result<Vec<Value>, String> {
    let rows = rows.as_array().map(Vec::as_slice).unwrap_or_default();
    rows.iter()
        .map(|row| match row {
            Value::String(text) => serde_json::from_str(text).map_err(|error| format!("a row is not JSON: {error}")),
            other => Ok(other.clone()),
        })
        .collect()
}

/// An identity from its SATS-JSON form, a product with one 256-bit field, `["0x…"]` or
/// `{"__identity__": "0x…"}`, as 64 lowercase hex digits.
pub(super) fn identity(value: &Value) -> Option<String> {
    let hex = special(value, "__identity__")?.as_str()?;
    let digits = hex.strip_prefix("0x").unwrap_or(hex);
    let valid = !digits.is_empty() && digits.len() <= 64 && digits.bytes().all(|byte| byte.is_ascii_hexdigit());
    valid.then(|| format!("{:0>64}", digits.to_ascii_lowercase()))
}

/// A timestamp from its SATS-JSON form, `[micros]` or `{"__timestamp_micros_since_unix_epoch__":
/// micros}`, in microseconds since 1970.
pub(super) fn timestamp(value: &Value) -> Option<i64> {
    special(value, "__timestamp_micros_since_unix_epoch__")?.as_i64()
}

/// The one field of a product that wraps a special value, written by position or by name.
fn special<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    match value {
        Value::Array(fields) if fields.len() == 1 => fields.first(),
        Value::Object(fields) if fields.len() == 1 => fields.get(name),
        _ => None,
    }
}
