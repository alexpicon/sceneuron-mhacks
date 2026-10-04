// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The card layer: which card each deck shows, who raised it, and when it hides by itself.
//!
//! A raised card's properties sit between the automatic and manual layers of the deck's
//! element, with `visible` forced on, so rules cannot hide a raised card but a manual hold can.
//! While its deck is covered (see [`Engine::cover_card`]), the deck's cover takes the card's
//! place, and lowering the card clears the cover, unless it lasts until the deck is uncovered
//! (see [`Engine::cover_deck`]).

use crate::decks::{CardAction, Deck, RaisedBy};
use crate::engine::Engine;
use crate::model::{Effect, Project};
use crate::props::PropsPatch;
use crate::types::{EngineError, Outcome};

/// The card a deck shows right now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RaisedCard {
    pub(crate) card: String,
    pub(crate) by: RaisedBy,
    /// When the card went up; raising the card already up keeps it.
    pub(crate) at_ms: u64,
    /// When the card lowers by itself; `None` keeps it up until it is lowered.
    pub(crate) hide_at_ms: Option<u64>,
}

impl Engine {
    /// Applies one control action to a deck as a single change.
    ///
    /// `card` names the card for [`CardAction::Raise`] and [`CardAction::Toggle`] and is ignored
    /// otherwise. The outcome reports a change only when the deck's state changed.
    pub fn card(
        &mut self,
        deck: &str,
        card: Option<&str>,
        action: CardAction,
        now_ms: u64,
    ) -> Result<Outcome, EngineError> {
        let found = self
            .project
            .deck(deck)
            .ok_or_else(|| EngineError::UnknownDeck(deck.to_owned()))?;
        let raised = self.raised_card(deck);
        let target = match action {
            CardAction::Raise => Some(named_card(found, card)?),
            CardAction::Toggle => {
                let id = named_card(found, card)?;
                (raised != Some(id)).then_some(id)
            }
            CardAction::Lower => None,
            CardAction::Next => Some(step(found, raised, true)),
            CardAction::Previous => Some(step(found, raised, false)),
        };
        let target = target.map(str::to_owned);
        let deck = found.id.clone();
        Ok(match target {
            Some(card) => self.raise_card(&deck, &card, RaisedBy::Manual, now_ms),
            None => self.lower_deck(&deck, "manual", now_ms),
        })
    }

    /// Lowers the raised card of every deck.
    pub fn lower_all_cards(&mut self, now_ms: u64) -> Outcome {
        let decks: Vec<String> = std::mem::take(&mut self.raised).into_keys().collect();
        if decks.is_empty() {
            return Outcome::NONE;
        }
        for deck in decks {
            self.lowered(&deck);
            self.record_lowered(&deck, "manual", now_ms);
        }
        self.bump(Outcome::CHANGED)
    }

    /// The id of the card a deck shows, if any.
    pub fn raised_card(&self, deck: &str) -> Option<&str> {
        self.raised.get(deck).map(|raised| raised.card.as_str())
    }

    /// Raises a card, restarting its deck's auto-hide delay. Replacing a different raised card
    /// pops the element so viewers notice the change; raising from lowered relies on the
    /// element's enter animation instead.
    pub(crate) fn raise_card(&mut self, deck: &str, card: &str, by: RaisedBy, now_ms: u64) -> Outcome {
        let Some(found) = self.project.deck(deck) else {
            return Outcome::NONE;
        };
        let Some(face) = found.card(card) else {
            return Outcome::NONE;
        };
        let detail = format!("card raised: {}: {}", found.name, face.name);
        let element = found.element.clone();
        let at_ms = match self.raised.get(deck) {
            Some(up) if up.card == card => up.at_ms,
            _ => now_ms,
        };
        let next = RaisedCard {
            card: card.to_owned(),
            by,
            at_ms,
            hide_at_ms: hide_at(found, now_ms),
        };
        let previous = self.raised.insert(deck.to_owned(), next.clone());
        if previous.as_ref() == Some(&next) {
            return Outcome::NONE;
        }
        if previous.as_ref().is_some_and(|previous| previous.card != card) {
            self.start_effect(&element, Effect::Pop);
        }
        // Raising the card already up, by the same source, restarts its auto-hide delay but not a
        // vote's window.
        if previous.is_none_or(|previous| previous.card != card || previous.by != by) {
            self.note_raised(deck, by, now_ms);
        }
        let source = match by {
            RaisedBy::Manual => "manual",
            RaisedBy::Camera => "camera",
            RaisedBy::Voice => "voice",
        };
        self.record(now_ms, source, &detail);
        self.cue_speech(deck, card, now_ms);
        self.bump(Outcome::CHANGED)
    }

    /// Lowers a deck's card, if one is raised.
    pub(crate) fn lower_deck(&mut self, deck: &str, source: &str, now_ms: u64) -> Outcome {
        if self.raised.remove(deck).is_none() {
            return Outcome::NONE;
        }
        self.lowered(deck);
        self.record_lowered(deck, source, now_ms);
        self.bump(Outcome::CHANGED)
    }

    /// Lowers every card whose auto-hide time has come.
    pub(crate) fn expire_cards(&mut self, now_ms: u64) -> Outcome {
        let due: Vec<String> = self
            .raised
            .iter()
            .filter(|(_, raised)| raised.hide_at_ms.is_some_and(|at| at <= now_ms))
            .map(|(deck, _)| deck.clone())
            .collect();
        if due.is_empty() {
            return Outcome::NONE;
        }
        for deck in due {
            self.raised.remove(&deck);
            self.lowered(&deck);
            self.record_lowered(&deck, "auto-hide", now_ms);
        }
        self.bump(Outcome::CHANGED)
    }

    /// What the card layer of an element shows when it is a deck's card slot and a card is raised:
    /// the deck's cover while the deck is covered, else the raised card's properties.
    pub(crate) fn card_layer(&self, element: &str) -> Option<&PropsPatch> {
        let deck = self.project.deck_for_element(element)?;
        let card = deck.card(&self.raised.get(&deck.id)?.card)?;
        Some(self.covers.get(&deck.id).map_or(&card.props, |cover| &cover.props))
    }

    /// Drops raised cards whose deck, card, or element the current project no longer has, with
    /// their covers unless they last, and the covers of removed decks, and restarts the auto-hide delay of a kept
    /// card whose deck's delay differs from `previous`, so the card follows the delay just saved.
    pub(crate) fn keep_raised_cards(&mut self, previous: &Project, now_ms: u64) {
        let project = &self.project;
        let covers = &mut self.covers;
        covers.retain(|id, _| project.deck(id).is_some());
        self.raised.retain(|id, raised| {
            let Some(deck) = project
                .deck(id)
                .filter(|deck| deck.card(&raised.card).is_some() && project.element(&deck.element).is_some())
            else {
                if covers.get(id).is_some_and(|cover| !cover.lasting) {
                    covers.remove(id);
                }
                return false;
            };
            if previous
                .deck(id)
                .is_none_or(|old| old.auto_hide_ms != deck.auto_hide_ms)
            {
                raised.hide_at_ms = hide_at(deck, now_ms);
            }
            true
        });
    }

    fn record_lowered(&mut self, deck: &str, source: &str, now_ms: u64) {
        let name = self.project.deck(deck).map_or(deck, |found| found.name.as_str());
        let detail = format!("card lowered: {name}");
        self.record(now_ms, source, &detail);
    }
}

/// When a card raised now lowers by itself; `None` when the deck keeps cards up.
fn hide_at(deck: &Deck, now_ms: u64) -> Option<u64> {
    (deck.auto_hide_ms > 0).then(|| now_ms.saturating_add(deck.auto_hide_ms))
}

/// The card a raise or toggle names, which must exist in the deck.
fn named_card<'a>(deck: &'a Deck, card: Option<&str>) -> Result<&'a str, EngineError> {
    let id = card.ok_or(EngineError::MissingCard)?;
    match deck.card(id) {
        Some(found) => Ok(found.id.as_str()),
        None => Err(EngineError::UnknownCard {
            deck: deck.id.clone(),
            card: id.to_owned(),
        }),
    }
}

/// The card after (or before) the raised one, wrapping around; from lowered, the first (or last).
/// Validation keeps every deck's card list non-empty.
fn step<'a>(deck: &'a Deck, raised: Option<&str>, forward: bool) -> &'a str {
    let count = deck.cards.len();
    let at = raised.and_then(|id| deck.cards.iter().position(|card| card.id == id));
    let index = match at {
        Some(at) if forward => (at + 1) % count,
        Some(at) => (at + count - 1) % count,
        None if forward => 0,
        None => count - 1,
    };
    &deck.cards[index].id
}
