// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Camera-signal fixtures: a deck whose cards answer to hand signs and a raised hand, packs whose
//! hand signals are on, frame summaries, and readers for what the camera raised.

use sceneuron_core::{
    CardSignal, Engine, HandSign, Project, RaisedBy, SignalSummary, default_project, pack_for_canvas,
};

use super::{deck_project, engine};

/// [`deck_project`] with the camera on: thumbs up raises "yes", thumbs down "no", and a raised
/// hand "maybe".
pub fn camera_project() -> Project {
    let mut project = deck_project();
    let deck = &mut project.decks[0];
    deck.camera = true;
    deck.cards[0].signal = Some(CardSignal::Sign(HandSign::ThumbsUp));
    deck.cards[1].signal = Some(CardSignal::Sign(HandSign::ThumbsDown));
    deck.cards[2].signal = Some(CardSignal::HandRaised);
    project
}

pub fn sign(sign: HandSign) -> SignalSummary {
    SignalSummary {
        signs: vec![sign],
        ..SignalSummary::default()
    }
}

pub fn nothing() -> SignalSummary {
    SignalSummary::default()
}

/// Feeds `summary` every 100 ms from `from_ms` through `to_ms`; true when any frame changed
/// the overlay.
pub fn show(engine: &mut Engine, summary: &SignalSummary, from_ms: u64, to_ms: u64) -> bool {
    let mut changed = false;
    for at_ms in (from_ms..=to_ms).step_by(100) {
        changed |= engine.observe_signals(summary, at_ms).changed;
    }
    changed
}

pub fn raised(engine: &Engine) -> Option<(String, RaisedBy)> {
    let deck = engine.snapshot(0).decks.into_iter().next().unwrap();
    deck.raised.zip(deck.raised_by)
}

pub fn by_camera(card: &str) -> Option<(String, RaisedBy)> {
    Some((card.to_owned(), RaisedBy::Camera))
}

/// An engine whose camera raised "yes" at 700 ms.
pub fn raised_by_camera() -> Engine {
    let mut engine = engine(camera_project());
    show(&mut engine, &sign(HandSign::ThumbsUp), 0, 700);
    assert_eq!(raised(&engine), by_camera("yes"));
    engine
}

/// The default project with the packs in `ids`, each deck's hand signals on.
pub fn listening(ids: &[&str]) -> Project {
    let mut project = default_project();
    for id in ids {
        project = project
            .add_pack(&pack_for_canvas(id, &project.canvas).unwrap())
            .unwrap();
    }
    for deck in &mut project.decks {
        deck.camera = true;
    }
    project
}

/// One hand showing `sign`, which the camera also counts as `fingers`.
pub fn pose(sign: HandSign, fingers: u8) -> SignalSummary {
    SignalSummary {
        fingers: Some(fingers),
        signs: vec![sign],
        ..SignalSummary::default()
    }
}

/// A hand showing `fingers` without a sign the camera knows.
pub fn count(fingers: u8) -> SignalSummary {
    SignalSummary {
        fingers: Some(fingers),
        ..SignalSummary::default()
    }
}

/// The card `deck` shows and who raised it.
pub fn shown(engine: &Engine, deck: &str) -> Option<(String, RaisedBy)> {
    let view = engine
        .snapshot(0)
        .decks
        .into_iter()
        .find(|view| view.id == deck)
        .unwrap();
    view.raised.zip(view.raised_by)
}
