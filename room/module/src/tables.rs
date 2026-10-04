// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The room's tables. Everyone sees rooms, who is in them, and who voted this round; what each
//! person voted stays in the private `vote` table until someone reveals the round.

use spacetimedb::{Identity, Timestamp};

/// A planning poker room, named by its code.
#[spacetimedb::table(accessor = room, public)]
pub struct Room {
    #[primary_key]
    pub code: String,
    /// The deck the room was created for, as its creator's Sceneuron names it.
    pub deck_name: String,
    /// Counts rounds from 1; the next round clears every vote.
    pub round: u32,
    /// Whether this round's votes are revealed.
    pub revealed: bool,
    pub created_at: Timestamp,
}

/// Someone in a room; a connection is in at most one room.
#[spacetimedb::table(accessor = participant, public)]
pub struct Participant {
    #[primary_key]
    pub identity: Identity,
    #[index(btree)]
    pub room: String,
    pub name: String,
    pub joined_at: Timestamp,
}

/// That a participant voted this round; it never says what.
#[spacetimedb::table(accessor = ballot, public)]
pub struct Ballot {
    #[primary_key]
    pub identity: Identity,
    #[index(btree)]
    pub room: String,
    pub round: u32,
}

/// What a participant voted this round: private, so no client can read it before the reveal.
#[spacetimedb::table(accessor = vote)]
pub struct Vote {
    #[primary_key]
    pub identity: Identity,
    #[index(btree)]
    pub room: String,
    pub round: u32,
    /// The card id in the voter's deck.
    pub card: String,
}

/// A vote made public by the reveal.
#[spacetimedb::table(accessor = result, public)]
pub struct RoundResult {
    #[primary_key]
    pub identity: Identity,
    #[index(btree)]
    pub room: String,
    pub round: u32,
    pub card: String,
}
