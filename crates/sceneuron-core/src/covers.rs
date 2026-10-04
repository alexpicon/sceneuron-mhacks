// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Covered cards: a deck can show a cover, such as "✓ Voted", in place of its raised card, so a
//! card can be up without anyone seeing which one until it is uncovered.

use crate::engine::Engine;
use crate::model::{Effect, ValidationError};
use crate::props::PropsPatch;
use crate::types::{EngineError, Outcome};

/// What a covered deck shows instead of its raised card.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Cover {
    pub(crate) props: PropsPatch,
    /// Lowering the card keeps the cover, so every card raised stays covered until the deck is
    /// uncovered.
    pub(crate) lasting: bool,
}

impl Engine {
    /// Covers a deck's raised card with `cover`, or uncovers the deck with `None`.
    ///
    /// While a deck is covered, its card slot shows the cover, made visible, instead of the raised
    /// card's properties, and keeps showing it when another card of the deck is raised, so
    /// changing a covered card never shows it. Covering a lowered deck covers the next card
    /// raised, so that card never shows either. Lowering the card clears this cover, and
    /// uncovering a raised card shows it again with a pop.
    pub fn cover_card(&mut self, deck: &str, cover: Option<PropsPatch>, now_ms: u64) -> Result<Outcome, EngineError> {
        self.set_cover(deck, cover.map(|props| Cover { props, lasting: false }), now_ms)
    }

    /// Covers a deck until it is uncovered with [`Engine::cover_card`] and `None`.
    ///
    /// Unlike a card's cover, lowering keeps this one, so a card raised right after the deck was
    /// lowered, from anywhere, is covered in the same change that raises it.
    pub fn cover_deck(&mut self, deck: &str, cover: PropsPatch, now_ms: u64) -> Result<Outcome, EngineError> {
        self.set_cover(
            deck,
            Some(Cover {
                props: cover,
                lasting: true,
            }),
            now_ms,
        )
    }

    /// The cover a deck shows, or will show over the next card raised, if it is covered.
    pub fn cover(&self, deck: &str) -> Option<&PropsPatch> {
        self.covers.get(deck).map(|cover| &cover.props)
    }

    fn set_cover(&mut self, deck: &str, cover: Option<Cover>, now_ms: u64) -> Result<Outcome, EngineError> {
        let found = self
            .project
            .deck(deck)
            .ok_or_else(|| EngineError::UnknownDeck(deck.to_owned()))?;
        let (deck, name, element) = (found.id.clone(), found.name.clone(), found.element.clone());
        let raised = self.raised.contains_key(&deck);
        let Some(cover) = cover else {
            if self.covers.remove(&deck).is_none() || !raised {
                return Ok(Outcome::NONE);
            }
            self.start_effect(&element, Effect::Pop);
            self.record(now_ms, "cards", &format!("card uncovered: {name}"));
            return Ok(self.bump(Outcome::CHANGED));
        };
        if cover.props.visible.is_some() {
            let message = "a cover must not set visible; covering a raised card shows it".to_owned();
            return Err(EngineError::Invalid(ValidationError(message)));
        }
        cover.props.validate().map_err(EngineError::Invalid)?;
        let previous = self.covers.insert(deck, cover.clone());
        let looks_the_same = previous.as_ref().is_some_and(|old| old.props == cover.props);
        if !raised || looks_the_same {
            return Ok(Outcome::NONE);
        }
        if previous.is_none() {
            self.record(now_ms, "cards", &format!("card covered: {name}"));
        }
        Ok(self.bump(Outcome::CHANGED))
    }

    /// Clears what a deck's card held when it lowers: its cover, unless the cover lasts until
    /// uncovered, and the vote a press opened.
    pub(crate) fn lowered(&mut self, deck: &str) {
        if self.covers.get(deck).is_some_and(|cover| !cover.lasting) {
            self.covers.remove(deck);
        }
        self.end_press(deck);
    }
}
