// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Votes, as hand signals see them: while a vote is open for a deck whose cards use finger counts,
//! finger counts go to that deck (see [`crate::signal_context`]).
//!
//! A vote is open for such a deck while either:
//!
//! - the shared room is joined with this deck and its round is not revealed yet, as the room
//!   client reports with [`Engine::set_room_round`]; or
//! - a card of the deck went up by a press, from the editor, a Stream Deck, a hotkey,
//!   obs-websocket, the HTTP API, or voice, less than [`VOTE_WINDOW_MS`] ago, and the deck has not
//!   lowered since. Raising the card already up changes nothing, so it does not restart the window.
//!   A card the camera raises opens no vote, so the camera never keeps itself listening.
//!
//! During a vote a press opened, a finger count shown changes the vote: the camera puts the count's
//! card up in place of the pressed one, and the card it put up stays while the vote is open and the
//! camera runs, since lowering it would end the vote. Neither restarts the window; once it runs
//! out, the card the camera put up lowers as when its signal ends.
//!
//! The end of the room's round, by the reveal or by leaving, ends the deck's vote, one a press
//! opened included: the card a reveal raises shows the vote already cast.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::decks::{Deck, RaisedBy};
use crate::engine::Engine;
use crate::signals::CardSignal;
use crate::types::Outcome;

/// How long a vote a press opened stays open while its card stays up: three minutes, enough to
/// show a count and change it while an estimate is discussed, yet short enough that a card left up
/// does not keep a deck's hand signs from it for the rest of a meeting. Lowering the card ends the
/// vote at once.
pub const VOTE_WINDOW_MS: u64 = 180_000;

/// What opened a deck's vote.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VoteBy {
    /// The shared room's round, which is joined with this deck and not revealed.
    Room,
    /// A card of the deck that went up by a press.
    Press,
}

/// A vote open for a deck.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct VoteView {
    pub by: VoteBy,
    /// When a vote a press opened ends, unless the deck lowers first, on the same clock as
    /// `now_ms`; `None` for the room's round, which ends at the reveal.
    pub until_ms: Option<u64>,
}

/// What may hold a vote open; kept in memory only, like raised cards.
#[derive(Clone, Debug, Default)]
pub(crate) struct Votes {
    /// The deck the shared room is joined with while its round is open.
    room: Option<String>,
    /// The deck the room's summary belongs to, kept after a reveal so the revealed votes still show
    /// on it; leaving the room empties the summary instead.
    summary_deck: Option<String>,
    /// When a press put up the card each deck shows, for decks whose cards use finger counts.
    pressed: BTreeMap<String, u64>,
}

impl Engine {
    /// The vote open for `deck` at `now_ms`, if any; only decks whose cards use finger counts have
    /// one.
    pub fn vote(&self, deck: &str, now_ms: u64) -> Option<VoteView> {
        if !self.project.deck(deck).is_some_and(uses_fingers) {
            return None;
        }
        if self.votes.room.as_deref() == Some(deck) {
            return Some(VoteView {
                by: VoteBy::Room,
                until_ms: None,
            });
        }
        let until_ms = self.votes.pressed.get(deck)?.saturating_add(VOTE_WINDOW_MS);
        (now_ms < until_ms).then_some(VoteView {
            by: VoteBy::Press,
            until_ms: Some(until_ms),
        })
    }

    /// Records the deck whose shared room round is open: `Some` while the room is joined with that
    /// deck and not revealed, and `None` once the round is revealed or the room is left.
    ///
    /// The room client reports this after the overlay follows the room, so the card a reveal raises
    /// for this Sceneuron's own vote opens no vote of its own.
    pub fn set_room_round(&mut self, deck: Option<&str>) -> Outcome {
        if self.votes.room.as_deref() == deck {
            return Outcome::NONE;
        }
        if let Some(ended) = self.votes.room.take() {
            self.votes.pressed.remove(&ended);
        }
        self.votes.room = deck.map(str::to_owned);
        if let Some(deck) = deck {
            self.votes.summary_deck = Some(deck.to_owned());
        }
        self.bump(Outcome::CHANGED)
    }

    /// The shared room's summary while its open round is joined with `deck`.
    pub(crate) fn room_summary_for(&self, deck: &str) -> Option<String> {
        let shown = self.votes.summary_deck.as_deref() == Some(deck) && !self.room_summary.is_empty();
        shown.then(|| self.room_summary.clone())
    }

    /// Whether a vote a press opened is open for `deck` at `now_ms`.
    pub(crate) fn press_vote(&self, deck: &str, now_ms: u64) -> bool {
        self.vote(deck, now_ms).is_some_and(|vote| vote.by == VoteBy::Press)
    }

    /// Whether the camera may put `card` of `deck` up in place of a card raised by hand or by
    /// voice: a card for a finger count, while a vote a press opened is open for the deck.
    pub(crate) fn count_changes_vote(&self, deck: &str, card: &str, now_ms: u64) -> bool {
        let count = self
            .project
            .deck(deck)
            .and_then(|found| found.card(card))
            .is_some_and(|found| matches!(found.signal, Some(CardSignal::Fingers(_))));
        count && self.press_vote(deck, now_ms)
    }

    /// Notes that a different card of `deck` went up, or the same card by someone else, raised
    /// `by` someone at `now_ms`.
    pub(crate) fn note_raised(&mut self, deck: &str, by: RaisedBy, now_ms: u64) {
        let counts = self.project.deck(deck).is_some_and(uses_fingers);
        match by {
            // The count the camera puts up during a vote a press opened changes that vote.
            RaisedBy::Camera if self.press_vote(deck, now_ms) => {}
            RaisedBy::Manual | RaisedBy::Voice if counts => {
                self.votes.pressed.insert(deck.to_owned(), now_ms);
            }
            _ => {
                self.votes.pressed.remove(deck);
            }
        }
    }

    /// Ends the vote a press opened for `deck`, whose card lowered.
    pub(crate) fn end_press(&mut self, deck: &str) {
        self.votes.pressed.remove(deck);
    }

    /// Forgets the presses of decks whose raised card the current project no longer has, or whose
    /// cards no longer use finger counts.
    pub(crate) fn keep_presses(&mut self) {
        let (raised, project) = (&self.raised, &self.project);
        self.votes
            .pressed
            .retain(|deck, _| raised.contains_key(deck) && project.deck(deck).is_some_and(uses_fingers));
    }

    /// Ends the votes whose window ran out, so the snapshot stops showing them.
    pub(crate) fn expire_votes(&mut self, now_ms: u64) -> Outcome {
        let before = self.votes.pressed.len();
        self.votes
            .pressed
            .retain(|_, at_ms| now_ms < at_ms.saturating_add(VOTE_WINDOW_MS));
        if self.votes.pressed.len() == before {
            return Outcome::NONE;
        }
        self.bump(Outcome::CHANGED)
    }
}

/// Whether one of the deck's cards asks for a finger count, whatever its camera switch says.
pub(crate) fn uses_fingers(deck: &Deck) -> bool {
    deck.cards
        .iter()
        .any(|card| matches!(card.signal, Some(CardSignal::Fingers(_))))
}
