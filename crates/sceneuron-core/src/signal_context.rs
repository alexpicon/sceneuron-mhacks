// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Hand signals by context: where a pose that is both a hand sign and a finger count goes.
//!
//! A hand showing a fist, pointing, a peace sign, or an open palm also gives a finger count, so a
//! deck that reads one of these signs and a deck whose cards use finger counts would each raise a
//! card for one pose. Two such decks compete while both listen (their hand signals are on). Then:
//!
//! - a deck whose cards use finger counts matches those cards only while a vote is open for it
//!   (see [`crate::votes`]);
//! - a deck that reads such signs matches its cards for them only while no deck it competes with
//!   has a vote open.
//!
//! So while no vote is open, the pose goes to the sign's deck, and while one is, its count goes to
//! the vote. A deck that competes with no other deck matches every card as before: while no
//! listening deck reads such a sign, finger counts go to the decks that use them. Two decks that
//! read the same sign both raise their card, and so do two decks that use the same count, as the
//! editor warns. A thumbs up or down gives no finger count, and a raised hand is no sign, so their
//! cards never wait.

use serde::Serialize;

use crate::decks::Deck;
use crate::engine::Engine;
use crate::signals::{CardSignal, HandSign};
use crate::votes::uses_fingers;

/// Where hand signals go while decks compete for the poses that are both a sign and a count.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HandSignalsView {
    /// Listening decks that read a sign the camera also counts, competing with a deck in
    /// `count_decks`.
    pub sign_decks: Vec<String>,
    /// Listening decks whose cards use finger counts, competing with a deck in `sign_decks`.
    pub count_decks: Vec<String>,
    /// The competing decks those poses go to right now: count decks with an open vote, and sign
    /// decks whose rivals have none; in the project's order.
    pub to: Vec<String>,
}

/// Which of a deck's cards the camera matches right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Listening {
    /// Its cards for finger counts.
    counts: bool,
    /// Its cards for signs that also give a finger count.
    counted_signs: bool,
}

impl Listening {
    /// Whether the camera matches a card asking for `signal` right now.
    pub(crate) fn allows(self, signal: CardSignal) -> bool {
        match signal {
            CardSignal::Fingers(_) => self.counts,
            CardSignal::Sign(sign) if gives_count(sign) => self.counted_signs,
            CardSignal::Sign(_) | CardSignal::HandRaised => true,
        }
    }
}

impl Engine {
    /// What the camera matches on each deck right now, in the project's order.
    pub(crate) fn listening(&self, now_ms: u64) -> Vec<Listening> {
        let decks = &self.project.decks;
        let voting: Vec<bool> = decks
            .iter()
            .map(|deck| takes_counts(deck) && self.vote(&deck.id, now_ms).is_some())
            .collect();
        decks
            .iter()
            .enumerate()
            .map(|(at, deck)| {
                let rivals = || decks.iter().enumerate().filter(move |(other, _)| *other != at);
                let rival_signs = rivals().any(|(_, other)| reads_counted_signs(other));
                let rival_votes = rivals().any(|(other, _)| voting[other]);
                Listening {
                    counts: !(takes_counts(deck) && rival_signs) || voting[at],
                    counted_signs: !(reads_counted_signs(deck) && rival_votes),
                }
            })
            .collect()
    }

    /// Where hand signals go right now, while decks compete for them; `None` while none do.
    pub(crate) fn hand_signals_view(&self, now_ms: u64) -> Option<HandSignalsView> {
        let decks = &self.project.decks;
        let rival = |deck: &Deck, kind: fn(&Deck) -> bool| decks.iter().any(|other| other.id != deck.id && kind(other));
        let ids = |kind: fn(&Deck) -> bool, rivals: fn(&Deck) -> bool| -> Vec<String> {
            decks
                .iter()
                .filter(|deck| kind(deck) && rival(deck, rivals))
                .map(|deck| deck.id.clone())
                .collect()
        };
        let sign_decks = ids(reads_counted_signs, takes_counts);
        if sign_decks.is_empty() {
            return None;
        }
        let count_decks = ids(takes_counts, reads_counted_signs);
        let to = decks
            .iter()
            .zip(self.listening(now_ms))
            .filter(|(deck, listening)| {
                (count_decks.contains(&deck.id) && listening.counts)
                    || (sign_decks.contains(&deck.id) && listening.counted_signs)
            })
            .map(|(deck, _)| deck.id.clone())
            .collect();
        Some(HandSignalsView {
            sign_decks,
            count_decks,
            to,
        })
    }
}

/// Whether a hand showing `sign` also gives a finger count: every sign but a thumbs up or down.
fn gives_count(sign: HandSign) -> bool {
    !matches!(sign, HandSign::ThumbsUp | HandSign::ThumbsDown)
}

/// A listening deck whose cards use finger counts.
fn takes_counts(deck: &Deck) -> bool {
    deck.camera && uses_fingers(deck)
}

/// A listening deck that reads a sign that also gives a finger count.
fn reads_counted_signs(deck: &Deck) -> bool {
    deck.camera
        && deck
            .cards
            .iter()
            .any(|card| matches!(card.signal, Some(CardSignal::Sign(sign)) if gives_count(sign)))
}
