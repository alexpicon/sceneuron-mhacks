// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Camera signals: raised hands, finger counts, and hand signs that raise cards on decks whose
//! camera is on.
//!
//! The host summarizes each analyzed frame as a [`SignalSummary`]. For each camera deck, the
//! first card whose signal matches is the candidate, among the cards the context lets the camera
//! match (see [`crate::signal_context`]). A candidate seen without a break for
//! [`RAISE_AFTER_MS`] raises its card, and a card the camera raised lowers once
//! [`LOWER_AFTER_MS`] pass without any candidate. The camera never replaces or lowers a card
//! raised by hand or by voice, except that a finger count shown during a vote a press opened
//! changes that vote (see [`crate::votes`]), and stops while automation is paused.
//!
//! A signal that reached the hold time is shown anew only after [`LOWER_AFTER_MS`] without it.
//! Detectors miss frames while a sign is held, so a missed frame neither raises the card again
//! nor brings back a card that auto-hide or a hand lowered.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::decks::{Deck, RaisedBy};
use crate::engine::Engine;
use crate::signal_context::Listening;
use crate::types::Outcome;

/// How long a candidate must be seen without a break before the camera raises its card.
const RAISE_AFTER_MS: u64 = 700;
/// How long a card the camera raised stays up once no card of its deck matches.
const LOWER_AFTER_MS: u64 = 1_500;
/// The most fingers a card can ask for: two hands.
pub(crate) const MAX_FINGERS: u8 = 10;

/// A static hand shape the camera recognizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandSign {
    ThumbsUp,
    ThumbsDown,
    OpenPalm,
    Peace,
    Pointing,
    Fist,
}

/// What the camera must see to raise a card: `"hand_raised"`, `{"fingers": 3}`, or
/// `{"sign": "thumbs_up"}` in JSON.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardSignal {
    /// A raised hand, or an arm up.
    HandRaised,
    /// Exactly this many fingers, 0 to 10, added up over the hands in view.
    Fingers(u8),
    /// This sign on any hand.
    Sign(HandSign),
}

/// What the camera sees in one analyzed frame, as far as card signals go.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SignalSummary {
    /// Hands held up.
    pub hands_raised: u8,
    /// Someone has an arm up.
    pub arms_up: bool,
    /// Extended fingers added up over the hands; `None` when no hand gives a count or any
    /// hand's count is unknown.
    pub fingers: Option<u8>,
    /// The signs hands show.
    pub signs: Vec<HandSign>,
}

impl CardSignal {
    /// Whether a frame summarized as `summary` shows this signal.
    pub fn matches(self, summary: &SignalSummary) -> bool {
        match self {
            CardSignal::HandRaised => summary.hands_raised > 0 || summary.arms_up,
            CardSignal::Fingers(count) => summary.fingers == Some(count),
            CardSignal::Sign(sign) => summary.signs.contains(&sign),
        }
    }
}

/// What the camera has been showing each deck; kept in memory only, like raised cards.
#[derive(Clone, Debug, Default)]
pub(crate) struct CameraWatch {
    /// When the last summary arrived.
    last_ms: Option<u64>,
    decks: BTreeMap<String, DeckWatch>,
}

#[derive(Clone, Debug, Default)]
struct DeckWatch {
    run: Option<Run>,
    /// When the deck last had a candidate.
    matched_ms: Option<u64>,
    /// The card of the last run that reached the hold time, and when its signal was last seen.
    held: Option<(String, u64)>,
}

/// One candidate seen without a break.
#[derive(Clone, Debug)]
struct Run {
    card: String,
    since_ms: u64,
    /// The run reached the hold time. A run acts once, so lowering a card by hand while its
    /// signal is still shown does not raise it again; the signal has to be shown anew.
    held: bool,
}

impl Engine {
    /// Feeds one frame's signals, taken at `now_ms`, and raises or lowers the cards of camera
    /// decks.
    ///
    /// Summaries older than the previous one are ignored. A gap longer than the detection
    /// stale time breaks every run and lowers what the camera raised, as [`Engine::tick`]
    /// does when summaries stop. While automation is paused no deck has a candidate, so cards
    /// the camera raised lower as when their signal ends, and cards raised by hand stay.
    pub fn observe_signals(&mut self, summary: &SignalSummary, now_ms: u64) -> Outcome {
        let stale_ms = self.project.detection.effective_stale_ms();
        let mut changed = match self.camera.last_ms {
            Some(last) if now_ms < last => return Outcome::NONE,
            Some(last) => now_ms - last > stale_ms && self.camera_stopped(now_ms),
            None => false,
        };
        self.camera.last_ms = Some(now_ms);
        let paused = !self.project.automation_enabled;
        let candidates: Vec<(String, Option<String>)> = self
            .project
            .decks
            .iter()
            .zip(self.listening(now_ms))
            .map(|(deck, listening)| (deck.id.clone(), candidate(deck, listening, summary).filter(|_| !paused)))
            .collect();
        let mut previous = std::mem::take(&mut self.camera.decks);
        for (deck, card) in candidates {
            let mut watch = previous.remove(&deck).unwrap_or_default();
            changed |= self.watch_deck(&deck, &mut watch, card, now_ms);
            self.camera.decks.insert(deck, watch);
        }
        if changed { Outcome::CHANGED } else { Outcome::NONE }
    }

    /// Lowers what the camera raised once summaries stop for the detection stale time.
    pub(crate) fn expire_signals(&mut self, now_ms: u64) -> Outcome {
        let stale_ms = self.project.detection.effective_stale_ms();
        let Some(last) = self.camera.last_ms else {
            return Outcome::NONE;
        };
        if now_ms.saturating_sub(last) <= stale_ms {
            return Outcome::NONE;
        }
        self.camera.last_ms = None;
        if self.camera_stopped(now_ms) {
            Outcome::CHANGED
        } else {
            Outcome::NONE
        }
    }

    /// Forgets every run and lowers every card the camera raised; true when a card lowered.
    fn camera_stopped(&mut self, now_ms: u64) -> bool {
        self.camera.decks.clear();
        let decks: Vec<String> = self
            .raised
            .iter()
            .filter(|(_, raised)| raised.by == RaisedBy::Camera)
            .map(|(deck, _)| deck.clone())
            .collect();
        let mut changed = false;
        for deck in decks {
            changed |= self.lower_deck(&deck, "camera", now_ms).changed;
        }
        changed
    }

    /// Advances one deck's run with this frame's candidate; true when its card changed.
    fn watch_deck(&mut self, deck: &str, watch: &mut DeckWatch, candidate: Option<String>, now_ms: u64) -> bool {
        match (&watch.run, candidate) {
            (Some(run), Some(card)) if run.card == card => {}
            (_, Some(card)) => {
                let held = watch
                    .held
                    .as_ref()
                    .is_some_and(|(held, seen_ms)| *held == card && now_ms - seen_ms < LOWER_AFTER_MS);
                watch.run = Some(Run {
                    card,
                    since_ms: now_ms,
                    held,
                });
            }
            (_, None) => watch.run = None,
        }
        if let Some(run) = &mut watch.run {
            watch.matched_ms = Some(now_ms);
            let reached = !run.held && now_ms - run.since_ms >= RAISE_AFTER_MS;
            run.held |= reached;
            if run.held {
                watch.held = Some((run.card.clone(), now_ms));
            }
            if reached {
                let kept = self.raised.get(deck).is_some_and(|raised| {
                    raised.card == run.card
                        || (raised.by != RaisedBy::Camera && !self.count_changes_vote(deck, &run.card, now_ms))
                });
                return !kept && self.raise_card(deck, &run.card, RaisedBy::Camera, now_ms).changed;
            }
        }
        let raised_by = self.raised.get(deck).map(|raised| raised.by);
        let quiet = watch.matched_ms.is_none_or(|at| now_ms - at >= LOWER_AFTER_MS);
        if quiet && raised_by == Some(RaisedBy::Camera) && !self.press_vote(deck, now_ms) {
            return self.lower_deck(deck, "camera", now_ms).changed;
        }
        false
    }
}

/// The first card of a camera deck whose signal the frame shows, among those the camera listens for.
fn candidate(deck: &Deck, listening: Listening, summary: &SignalSummary) -> Option<String> {
    if !deck.camera {
        return None;
    }
    deck.cards
        .iter()
        .find(|card| {
            card.signal
                .is_some_and(|signal| listening.allows(signal) && signal.matches(summary))
        })
        .map(|card| card.id.clone())
}
