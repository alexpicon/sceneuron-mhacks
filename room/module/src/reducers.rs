// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What clients can do in a room. Every reducer acts on the caller's own room, which it finds from
//! the caller's identity, so nobody can vote, reveal, or start a round in a room they are not in.

use spacetimedb::{ReducerContext, Table};

use crate::rules::{check_card, check_deck_name, check_name, make_code, normalize_code};
use crate::tables::{Ballot, Participant, Room, RoundResult, Vote, ballot, participant, result, room, vote};

/// Fresh codes tried before giving up; with 31^6 codes, a clash is already rare.
const CODE_TRIES: usize = 32;

/// Creates a room with a fresh code and puts the caller in it, leaving any room they were in.
#[spacetimedb::reducer]
pub fn create_room(ctx: &ReducerContext, deck_name: String, name: String) -> Result<(), String> {
    let deck_name = check_deck_name(&deck_name)?;
    let name = check_name(&name)?;
    let code = fresh_code(ctx)?;
    leave_current(ctx);
    ctx.db.room().insert(Room {
        code: code.clone(),
        deck_name,
        round: 1,
        revealed: false,
        created_at: ctx.timestamp,
    });
    enter(ctx, &code, name);
    log::info!("room {code} created");
    Ok(())
}

/// Puts the caller in the room with a code they chose, such as CHASKI, creating the room first
/// when nobody has it, so one call always lands in the same room.
#[spacetimedb::reducer]
pub fn open_room(ctx: &ReducerContext, code: String, deck_name: String, name: String) -> Result<(), String> {
    let code = normalize_code(&code)?;
    if ctx.db.room().code().find(&code).is_none() {
        let deck_name = check_deck_name(&deck_name)?;
        check_name(&name)?;
        ctx.db.room().insert(Room {
            code: code.clone(),
            deck_name,
            round: 1,
            revealed: false,
            created_at: ctx.timestamp,
        });
        log::info!("room {code} opened");
    }
    join_room(ctx, code, name)
}

/// Puts the caller in the room with this code, leaving any other room; in the same room it renames
/// the caller.
#[spacetimedb::reducer]
pub fn join_room(ctx: &ReducerContext, code: String, name: String) -> Result<(), String> {
    let code = normalize_code(&code)?;
    let name = check_name(&name)?;
    if ctx.db.room().code().find(&code).is_none() {
        return Err(format!(
            "no room has the code {code}; check it with whoever created the room"
        ));
    }
    if let Some(me) = ctx.db.participant().identity().find(ctx.sender())
        && me.room == code
    {
        ctx.db.participant().identity().update(Participant { name, ..me });
        return Ok(());
    }
    leave_current(ctx);
    enter(ctx, &code, name);
    Ok(())
}

#[spacetimedb::reducer]
pub fn leave_room(ctx: &ReducerContext) -> Result<(), String> {
    my_room(ctx)?;
    leave_current(ctx);
    Ok(())
}

/// Records the caller's vote for this round, replacing an earlier one; refused after the reveal.
#[spacetimedb::reducer]
pub fn cast_vote(ctx: &ReducerContext, card: String) -> Result<(), String> {
    let card = check_card(&card)?;
    let (me, room) = my_room(ctx)?;
    if room.revealed {
        return Err("the cards are already revealed; start the next round to vote again".to_owned());
    }
    ctx.db.vote().identity().delete(me.identity);
    ctx.db.vote().insert(Vote {
        identity: me.identity,
        room: room.code.clone(),
        round: room.round,
        card,
    });
    ctx.db.ballot().identity().delete(me.identity);
    ctx.db.ballot().insert(Ballot {
        identity: me.identity,
        room: room.code,
        round: room.round,
    });
    Ok(())
}

/// Makes this round's votes public for everyone in the caller's room; revealing twice changes
/// nothing, so two people who reveal at once both succeed.
#[spacetimedb::reducer]
pub fn reveal(ctx: &ReducerContext) -> Result<(), String> {
    let (_, room) = my_room(ctx)?;
    if room.revealed {
        return Ok(());
    }
    for vote in ctx.db.vote().room().filter(&room.code) {
        if vote.round == room.round {
            ctx.db.result().identity().delete(vote.identity);
            ctx.db.result().insert(RoundResult {
                identity: vote.identity,
                room: vote.room,
                round: vote.round,
                card: vote.card,
            });
        }
    }
    ctx.db.room().code().update(Room { revealed: true, ..room });
    Ok(())
}

/// Clears every vote, ballot, and result of the caller's room and starts the next round.
#[spacetimedb::reducer]
pub fn next_round(ctx: &ReducerContext) -> Result<(), String> {
    let (_, room) = my_room(ctx)?;
    clear_votes(ctx, &room.code);
    ctx.db.room().code().update(Room {
        round: room.round.saturating_add(1),
        revealed: false,
        ..room
    });
    Ok(())
}

/// A connection that ends leaves its room, so nobody waits for a vote that cannot come.
#[spacetimedb::reducer(client_disconnected)]
pub fn client_disconnected(ctx: &ReducerContext) {
    leave_current(ctx);
}

fn fresh_code(ctx: &ReducerContext) -> Result<String, String> {
    for _ in 0..CODE_TRIES {
        let code = make_code(|| ctx.random::<u32>());
        if ctx.db.room().code().find(&code).is_none() {
            return Ok(code);
        }
    }
    Err("no free room code was found; try again".to_owned())
}

fn enter(ctx: &ReducerContext, code: &str, name: String) {
    ctx.db.participant().insert(Participant {
        identity: ctx.sender(),
        room: code.to_owned(),
        name,
        joined_at: ctx.timestamp,
    });
}

/// The caller's participant row and room.
fn my_room(ctx: &ReducerContext) -> Result<(Participant, Room), String> {
    let me = ctx
        .db
        .participant()
        .identity()
        .find(ctx.sender())
        .ok_or_else(|| "join a room first".to_owned())?;
    let room = ctx
        .db
        .room()
        .code()
        .find(&me.room)
        .ok_or_else(|| format!("room {} no longer exists", me.room))?;
    Ok((me, room))
}

/// Takes the caller out of their room, with their ballot, vote, and result, and deletes the room
/// once nobody is left in it.
fn leave_current(ctx: &ReducerContext) {
    let identity = ctx.sender();
    let Some(me) = ctx.db.participant().identity().find(identity) else {
        return;
    };
    ctx.db.participant().identity().delete(identity);
    ctx.db.ballot().identity().delete(identity);
    ctx.db.vote().identity().delete(identity);
    ctx.db.result().identity().delete(identity);
    if ctx.db.participant().room().filter(&me.room).next().is_none() {
        clear_votes(ctx, &me.room);
        ctx.db.room().code().delete(&me.room);
        log::info!("room {} closed", me.room);
    }
}

fn clear_votes(ctx: &ReducerContext, code: &str) {
    ctx.db.ballot().room().delete(code);
    ctx.db.vote().room().delete(code);
    ctx.db.result().room().delete(code);
}
