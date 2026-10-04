// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What the room does to this Sceneuron's overlay: it covers the bound deck while a round is open,
//! shows each person their own vote at the reveal, lowers the deck for the next round, tells the
//! engine whether the deck's round is open, so finger counts go to the vote, and keeps the
//! `{{room}}` summary current. Each set of changes is one change of the engine, so no frame of the
//! overlay ever shows a hidden vote.

use sceneuron_core::{CardAction, Engine, Outcome, PropsPatch};

use crate::backend::RoomView;

/// What a cover shows in place of a hidden vote.
pub const COVER_TEXT: &str = "✓ Voted";

/// A change to the bound deck.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Local {
    /// A round starts for this connection: the deck lowers and is covered, so the next card
    /// raised is a hidden vote.
    NewRound,
    /// The deck is covered again after something cleared the cover while the round is open.
    Cover,
    /// The votes are revealed: this card, the vote of this connection, shows uncovered; without
    /// a vote, a covered card lowers instead of showing.
    Reveal(Option<String>),
    /// This connection left the room: a covered card lowers, never shown, and the cover clears.
    Leave,
}

/// What happens to the bound deck when the room goes from `previous` to `next`.
pub(crate) fn changes(previous: Option<&RoomView>, next: &RoomView) -> Vec<Local> {
    let reveal = || Local::Reveal(next.my_result().map(str::to_owned));
    match previous {
        None if next.revealed => Vec::new(),
        None => vec![Local::NewRound],
        Some(old) if old.round != next.round && next.revealed => vec![Local::NewRound, reveal()],
        Some(old) if old.round != next.round => vec![Local::NewRound],
        Some(old) if !old.revealed && next.revealed => vec![reveal()],
        Some(_) => Vec::new(),
    }
}

/// "✓ Voted" in white on blue, at a size that fits the width of the deck's card slot.
pub fn cover_for(engine: &Engine, deck: &str) -> Option<PropsPatch> {
    let deck = engine.project().deck(deck)?;
    let slot = engine.project().element(&deck.element)?;
    let fitting = (slot.props.width / 5.0).floor();
    Some(PropsPatch {
        text: Some(COVER_TEXT.to_owned()),
        color: Some("#ffffffff".to_owned()),
        background: Some("#1d4ed8ff".to_owned()),
        font_size: Some(slot.props.font_size.min(fitting).max(1.0)),
        ..PropsPatch::default()
    })
}

/// Applies `changes` to `deck` and sets the summary when one is given, as one engine change.
pub(crate) fn apply(engine: &mut Engine, deck: &str, changes: &[Local], summary: Option<&str>, now_ms: u64) -> Outcome {
    let mut outcome = summary.map_or(Outcome::default(), |text| engine.set_room_summary(text));
    let Some(cover) = cover_for(engine, deck) else {
        // The deck left the project; there is nothing to cover, but the round still opens or ends.
        return merge(outcome, report_round(engine, deck, changes));
    };
    for change in changes {
        let next = match change {
            Local::NewRound => {
                let lowered = card(engine, deck, None, CardAction::Lower, now_ms);
                merge(lowered, cover_deck(engine, deck, cover.clone(), now_ms))
            }
            Local::Cover if engine.cover(deck).is_none() => cover_deck(engine, deck, cover.clone(), now_ms),
            Local::Cover => Outcome::default(),
            Local::Reveal(vote) => reveal(engine, deck, vote.as_deref(), now_ms),
            Local::Leave => {
                let lowered = if covered_card(engine, deck) {
                    card(engine, deck, None, CardAction::Lower, now_ms)
                } else {
                    Outcome::default()
                };
                merge(lowered, uncover(engine, deck, now_ms))
            }
        };
        outcome = merge(outcome, next);
    }
    // Reported after the changes, so the card a reveal raises for this connection's vote opens no
    // vote of its own.
    merge(outcome, report_round(engine, deck, changes))
}

/// Tells the engine whether `deck`'s round is open after `changes`: a new round opens it, and a
/// reveal or leaving ends it.
fn report_round(engine: &mut Engine, deck: &str, changes: &[Local]) -> Outcome {
    let open = changes.iter().rev().find_map(|change| match change {
        Local::NewRound => Some(true),
        Local::Reveal(_) | Local::Leave => Some(false),
        Local::Cover => None,
    });
    open.map_or(Outcome::default(), |open| engine.set_room_round(open.then_some(deck)))
}

fn reveal(engine: &mut Engine, deck: &str, vote: Option<&str>, now_ms: u64) -> Outcome {
    let in_deck = vote.filter(|vote| {
        engine
            .project()
            .deck(deck)
            .is_some_and(|found| found.card(vote).is_some())
    });
    let shown = match in_deck {
        Some(vote) => card(engine, deck, Some(vote), CardAction::Raise, now_ms),
        None if covered_card(engine, deck) => card(engine, deck, None, CardAction::Lower, now_ms),
        None => Outcome::default(),
    };
    merge(shown, uncover(engine, deck, now_ms))
}

fn covered_card(engine: &Engine, deck: &str) -> bool {
    engine.raised_card(deck).is_some() && engine.cover(deck).is_some()
}

/// Card actions and covers on a deck the project has cannot fail, so a refusal changes nothing.
fn card(engine: &mut Engine, deck: &str, card: Option<&str>, action: CardAction, now_ms: u64) -> Outcome {
    engine.card(deck, card, action, now_ms).unwrap_or_default()
}

/// Covers the deck until the round's reveal or the end of the session; lowering keeps the cover, so
/// a card raised right after a lower, from anywhere, never shows.
fn cover_deck(engine: &mut Engine, deck: &str, cover: PropsPatch, now_ms: u64) -> Outcome {
    engine.cover_deck(deck, cover, now_ms).unwrap_or_default()
}

fn uncover(engine: &mut Engine, deck: &str, now_ms: u64) -> Outcome {
    engine.cover_card(deck, None, now_ms).unwrap_or_default()
}

fn merge(first: Outcome, second: Outcome) -> Outcome {
    Outcome {
        changed: first.changed || second.changed,
        project_dirty: first.project_dirty || second.project_dirty,
        telemetry: first.telemetry || second.telemetry,
    }
}
