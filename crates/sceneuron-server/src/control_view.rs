// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The control view: the small state a control token may read. It names decks, cards, and
//! elements, says what shows and which reaction plays, and leaves out the camera, captions,
//! reasons, and the event log, which other people in a meeting should not find on someone's
//! Stream Deck.

use std::sync::Arc;

use sceneuron_core::{Card, Deck, Flood, Project, RaisedBy, Snapshot};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ControlView {
    /// The snapshot revision this view was taken at.
    pub(crate) revision: u64,
    automation_enabled: bool,
    decks: Vec<ControlDeck>,
    elements: Vec<ControlElement>,
    /// The reaction playing on the overlay when the view was taken, so a key can light up while
    /// its preset plays; left out when none does.
    #[serde(skip_serializing_if = "Option::is_none")]
    flood: Option<ControlFlood>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
struct ControlFlood {
    flood: Flood,
    nonce: u64,
    /// How long it still plays after the view was taken.
    left_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct ControlDeck {
    id: String,
    name: String,
    raised: Option<String>,
    raised_by: Option<RaisedBy>,
    hide_at_ms: Option<u64>,
    /// The raised card shows its deck's cover, such as a hidden vote in a shared room.
    covered: bool,
    cards: Vec<CardFace>,
}

/// How a card looks when raised: the deck's element with the card's properties. The text is
/// left as written, so a `{{scene}}` placeholder never brings a caption to a control surface.
#[derive(Clone, Debug, PartialEq, Serialize)]
struct CardFace {
    id: String,
    name: String,
    text: String,
    color: String,
    background: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct ControlElement {
    id: String,
    name: String,
    visible: bool,
    /// A manual override holds some of its properties; releasing the element frees them all.
    held: bool,
}

impl ControlView {
    /// The view of `snapshot`, which must be taken from the engine that holds `project`.
    pub(crate) fn new(snapshot: &Snapshot, project: &Project) -> Self {
        let decks = snapshot
            .decks
            .iter()
            .map(|deck| ControlDeck {
                id: deck.id.clone(),
                name: deck.name.clone(),
                raised: deck.raised.clone(),
                raised_by: deck.raised_by,
                hide_at_ms: deck.hide_at_ms,
                covered: deck.covered,
                cards: project
                    .deck(&deck.id)
                    .map(|found| found.cards.iter().map(|card| face(project, found, card)).collect())
                    .unwrap_or_default(),
            })
            .collect();
        let elements = snapshot
            .elements
            .iter()
            .map(|element| ControlElement {
                id: element.id.clone(),
                name: element.name.clone(),
                visible: element.props.visible,
                held: !element.overridden.is_empty(),
            })
            .collect();
        let flood = snapshot.flood.and_then(|flood| {
            let left_ms = flood.left_ms(snapshot.now_ms);
            (left_ms > 0).then_some(ControlFlood {
                flood: flood.flood,
                nonce: flood.nonce,
                left_ms,
            })
        });
        Self {
            revision: snapshot.revision,
            automation_enabled: snapshot.automation_enabled,
            decks,
            elements,
            flood,
        }
    }

    /// Whether `other` shows the same, whatever the revisions and however long ago the same
    /// reaction began.
    fn shows_the_same_as(&self, other: &ControlView) -> bool {
        let nonce = |view: &ControlView| view.flood.map(|flood| flood.nonce);
        self.automation_enabled == other.automation_enabled
            && self.decks == other.decks
            && self.elements == other.elements
            && nonce(self) == nonce(other)
    }
}

fn face(project: &Project, deck: &Deck, card: &Card) -> CardFace {
    let mut props = project
        .element(&deck.element)
        .map(|element| element.props.clone())
        .unwrap_or_default();
    props.apply(&card.props);
    CardFace {
        id: card.id.clone(),
        name: card.name.clone(),
        text: props.text,
        color: props.color,
        background: props.background,
    }
}

/// What an event stream that last sent `last` sends for `batch`, the views published since,
/// oldest first: the newest view, when any of them shows something `last` did not.
///
/// A view that came and went within one batch still counts. A client may have taken it from
/// a request's reply, and it needs the newer view even when that matches `last` again.
pub(crate) fn view_to_send(
    last: &mut Option<Arc<ControlView>>,
    batch: Vec<Arc<ControlView>>,
) -> Option<Arc<ControlView>> {
    let changed = batch
        .iter()
        .any(|view| last.as_ref().is_none_or(|last| !last.shows_the_same_as(view)));
    let newest = batch.into_iter().last().filter(|_| changed)?;
    *last = Some(Arc::clone(&newest));
    Some(newest)
}

#[cfg(test)]
mod tests {
    use sceneuron_core::{Engine, PropsPatch, default_project};

    use super::*;

    fn view(engine: &Engine) -> Arc<ControlView> {
        Arc::new(ControlView::new(&engine.snapshot(0), engine.project()))
    }

    fn hold_brb_visible(engine: &mut Engine, visible: bool) {
        let patch = PropsPatch {
            visible: Some(visible),
            ..PropsPatch::default()
        };
        engine.set_override("brb", &patch, 0).unwrap();
    }

    #[test]
    fn the_first_view_is_sent() {
        let engine = Engine::new(default_project(), 0).unwrap();
        let mut last = None;
        let first = view(&engine);
        assert_eq!(
            view_to_send(&mut last, vec![Arc::clone(&first)]),
            Some(Arc::clone(&first))
        );
        assert_eq!(last, Some(first));
    }

    #[test]
    fn a_change_outside_the_view_is_not_sent() {
        let mut engine = Engine::new(default_project(), 0).unwrap();
        let mut last = Some(view(&engine));
        engine.note("system", "saved the project again", 0);
        let newer = view(&engine);
        assert_ne!(Some(newer.revision), last.as_ref().map(|view| view.revision));
        assert_eq!(view_to_send(&mut last, vec![newer]), None);
    }

    #[test]
    fn the_newest_view_is_sent_when_any_view_in_the_batch_changed() {
        let mut engine = Engine::new(default_project(), 0).unwrap();
        let first = view(&engine);
        let mut last = Some(Arc::clone(&first));
        hold_brb_visible(&mut engine, false);
        let hidden = view(&engine);
        engine.release_overrides(Some("brb"), 0).unwrap();
        let back = view(&engine);
        assert!(back.shows_the_same_as(&first));
        let sent = view_to_send(&mut last, vec![hidden, Arc::clone(&back)]);
        assert_eq!(sent, Some(Arc::clone(&back)));
        assert_eq!(last, Some(back));
    }

    #[test]
    fn a_reaction_shows_while_it_plays_and_each_new_one_is_sent() {
        let at = |engine: &Engine, now_ms| Arc::new(ControlView::new(&engine.snapshot(now_ms), engine.project()));
        let mut engine = Engine::new(default_project(), 0).unwrap();
        let mut last = Some(view(&engine));
        engine.trigger_flood(Flood::Email, 1_000);
        let playing = at(&engine, 1_500).flood.expect("the email plays");
        assert_eq!((playing.flood, playing.left_ms), (Flood::Email, 3_500));
        assert_eq!(at(&engine, 5_000).flood, None, "it ended");
        assert!(view_to_send(&mut last, vec![at(&engine, 1_000)]).is_some());
        assert_eq!(
            view_to_send(&mut last, vec![at(&engine, 2_000)]),
            None,
            "the same reaction"
        );
        engine.trigger_flood(Flood::Email, 2_500);
        let again = at(&engine, 2_500);
        assert!(
            view_to_send(&mut last, vec![Arc::clone(&again)]).is_some(),
            "played again"
        );
        let json = serde_json::to_value(&*again).unwrap();
        assert_eq!(
            json["flood"],
            serde_json::json!({"flood": "email", "nonce": playing.nonce + 1, "left_ms": 4_000})
        );
    }
}
