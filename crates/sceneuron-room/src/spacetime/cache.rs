// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! This connection's copy of the public tables of `room/module`, kept current from the rows the
//! server sends, and the room this connection is in, read from it.
//!
//! Rows are SATS-JSON product values, written by position in transaction updates and by field name
//! in the first look at the subscription, so each field is read by its position in the module's
//! table or by its name.

use std::collections::BTreeMap;

use serde_json::Value;

use super::protocol::{TableRows, identity, timestamp};
use crate::backend::{Member, RoomView, Shown};

/// The queries this client subscribes to: every public table. Rooms are few and small, and the
/// module keeps votes out of all of them until the reveal.
pub(super) const QUERIES: [&str; 4] = [
    "SELECT * FROM room",
    "SELECT * FROM participant",
    "SELECT * FROM ballot",
    "SELECT * FROM result",
];

#[derive(Clone, Debug, PartialEq, Eq)]
struct Room {
    deck_name: String,
    round: u32,
    revealed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Participant {
    room: String,
    name: String,
    joined_at: i64,
}

/// A ballot or a result: someone's vote in a room's round, with the card once it is revealed.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Vote {
    room: String,
    round: u32,
    card: String,
}

/// The rows of the subscribed tables, each keyed by its primary key.
#[derive(Debug, Default)]
pub(super) struct Cache {
    rooms: BTreeMap<String, Room>,
    participants: BTreeMap<String, Participant>,
    ballots: BTreeMap<String, Vote>,
    results: BTreeMap<String, Vote>,
}

impl Cache {
    /// Applies one message's rows: in each table, deletions before insertions, so a row that
    /// changed is replaced. Tables the room does not use are skipped.
    pub(super) fn apply(&mut self, tables: &[TableRows]) -> Result<(), String> {
        for rows in tables {
            match rows.table.as_str() {
                "room" => change(&mut self.rooms, rows, room)?,
                "participant" => change(&mut self.participants, rows, participant)?,
                "ballot" => change(&mut self.ballots, rows, |row| vote(row, false))?,
                "result" => change(&mut self.results, rows, |row| vote(row, true))?,
                _ => {}
            }
        }
        Ok(())
    }

    /// The room the identity `me` is in, with its members in the order they joined and this
    /// round's votes once revealed.
    pub(super) fn view_of(&self, me: &str) -> Option<RoomView> {
        let code = &self.participants.get(me)?.room;
        let room = self.rooms.get(code)?;
        let mut people: Vec<(&String, &Participant)> = self
            .participants
            .iter()
            .filter(|(_, person)| person.room == *code)
            .collect();
        people.sort_by_key(|(id, person)| (person.joined_at, *id));
        let this_round = |vote: &Vote| vote.room == *code && vote.round == room.round;
        let members: Vec<Member> = people
            .iter()
            .map(|(id, person)| Member {
                id: (*id).clone(),
                name: person.name.clone(),
                voted: self.ballots.get(*id).is_some_and(this_round),
                me: *id == me,
            })
            .collect();
        let mut results: Vec<Shown> = self
            .results
            .iter()
            .filter(|(_, result)| this_round(result))
            .map(|(id, result)| Shown {
                id: id.clone(),
                card: result.card.clone(),
            })
            .collect();
        results.sort_by_key(|shown| members.iter().position(|member| member.id == shown.id));
        Some(RoomView {
            code: code.clone(),
            deck_name: room.deck_name.clone(),
            round: room.round,
            revealed: room.revealed,
            members,
            results,
        })
    }
}

/// Removes the deleted rows of one table, then adds the inserted ones, keyed as `decode` says.
fn change<T>(
    table: &mut BTreeMap<String, T>,
    rows: &TableRows,
    decode: impl Fn(&Value) -> Option<(String, T)>,
) -> Result<(), String> {
    let decoded =
        |row: &Value| decode(row).ok_or_else(|| format!("a {} row has an unexpected shape: {row}", rows.table));
    for row in &rows.deletes {
        table.remove(&decoded(row)?.0);
    }
    for row in &rows.inserts {
        let (key, value) = decoded(row)?;
        table.insert(key, value);
    }
    Ok(())
}

/// Field `index` of a product value, or the field called `name` when it is written by name.
fn field<'a>(row: &'a Value, index: usize, name: &str) -> Option<&'a Value> {
    match row {
        Value::Array(fields) => fields.get(index),
        Value::Object(fields) => fields.get(name),
        _ => None,
    }
}

fn text(row: &Value, index: usize, name: &str) -> Option<String> {
    field(row, index, name)?.as_str().map(str::to_owned)
}

fn number(row: &Value, index: usize, name: &str) -> Option<u32> {
    u32::try_from(field(row, index, name)?.as_u64()?).ok()
}

/// `room(code, deck_name, round, revealed, created_at)`, keyed by code.
fn room(row: &Value) -> Option<(String, Room)> {
    let found = Room {
        deck_name: text(row, 1, "deck_name")?,
        round: number(row, 2, "round")?,
        revealed: field(row, 3, "revealed")?.as_bool()?,
    };
    Some((text(row, 0, "code")?, found))
}

/// `participant(identity, room, name, joined_at)`, keyed by identity.
fn participant(row: &Value) -> Option<(String, Participant)> {
    let found = Participant {
        room: text(row, 1, "room")?,
        name: text(row, 2, "name")?,
        joined_at: timestamp(field(row, 3, "joined_at")?)?,
    };
    Some((identity(field(row, 0, "identity")?)?, found))
}

/// `ballot(identity, room, round)` or `result(identity, room, round, card)`, keyed by identity.
fn vote(row: &Value, with_card: bool) -> Option<(String, Vote)> {
    let card = if with_card {
        text(row, 3, "card")?
    } else {
        String::new()
    };
    let found = Vote {
        room: text(row, 1, "room")?,
        round: number(row, 2, "round")?,
        card,
    };
    Some((identity(field(row, 0, "identity")?)?, found))
}
