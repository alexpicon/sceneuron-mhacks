// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Card decks: sets of cards that one text element, the deck's card slot, shows one at a time.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::model::{ElementKind, Project, ValidationError};
use crate::props::PropsPatch;
use crate::signals::{CardSignal, MAX_FINGERS};
use crate::validate::{check_count, check_id, check_name, prefix};
use crate::voice;

const MAX_DECKS: usize = 16;
const MAX_CARDS: usize = 24;
const MIN_AUTO_HIDE_MS: u64 = 1_000;
const MAX_AUTO_HIDE_MS: u64 = 3_600_000;

/// A set of cards shown one at a time by a text element, such as meeting signals or poker values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Deck {
    pub id: String,
    pub name: String,
    /// The text element that shows the raised card; an element shows at most one deck.
    pub element: String,
    pub cards: Vec<Card>,
    /// How long a raised card stays up; 0 keeps it up until it is lowered.
    #[serde(default)]
    pub auto_hide_ms: u64,
    /// Whether the camera may raise cards by their signals; off until the user turns it on.
    #[serde(default)]
    pub camera: bool,
    /// Whether spoken commands and trigger phrases may raise these cards; off until the user
    /// turns it on.
    #[serde(default)]
    pub voice: bool,
    /// Whether a card is spoken aloud when it is raised; off until the user turns it on.
    #[serde(default)]
    pub speak: bool,
}

/// One card: what the deck's element shows while the card is raised.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub name: String,
    /// Text, colors, and other properties the element takes while the card is raised. Raising
    /// and lowering decide visibility, so cards never set `visible`.
    #[serde(default)]
    pub props: PropsPatch,
    /// What the camera must see to raise this card, when its deck's camera is on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signal: Option<CardSignal>,
    /// Up to four phrases that raise this card when they are said, while its deck's voice is on
    /// and voice control listens all the time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phrases: Vec<String>,
    /// What to say when the card is raised on a deck that speaks, instead of its text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speech: Option<String>,
}

/// Who raised a deck's current card.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RaisedBy {
    Manual,
    Camera,
    Voice,
}

/// What a control does to a deck.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardAction {
    /// Raises the named card, replacing any raised one.
    Raise,
    /// Lowers the named card when it is raised, else raises it.
    Toggle,
    /// Lowers whatever card is raised.
    Lower,
    /// Raises the card after the raised one, wrapping around; the first when none is raised.
    Next,
    /// Raises the card before the raised one, wrapping around; the last when none is raised.
    Previous,
}

impl Deck {
    /// Looks up a card by id.
    pub fn card(&self, id: &str) -> Option<&Card> {
        self.cards.iter().find(|card| card.id == id)
    }
}

impl Project {
    /// Looks up a deck by id.
    pub fn deck(&self, id: &str) -> Option<&Deck> {
        self.decks.iter().find(|deck| deck.id == id)
    }

    /// The deck an element shows, if any.
    pub fn deck_for_element(&self, element: &str) -> Option<&Deck> {
        self.decks.iter().find(|deck| deck.element == element)
    }

    /// Checks deck limits, ids, card slots, cards, signals, phrases, and auto-hide delays.
    pub(crate) fn validate_decks(&self) -> Result<(), ValidationError> {
        check_count("decks", self.decks.len(), MAX_DECKS)?;
        let mut ids = BTreeSet::new();
        let mut slots = BTreeSet::new();
        for deck in &self.decks {
            check_id(&deck.id)?;
            check_name(&deck.name)?;
            if !ids.insert(deck.id.as_str()) {
                let message = format!("duplicate deck id {:?}", deck.id);
                return Err(ValidationError(message));
            }
            match self.element(&deck.element) {
                None => {
                    let message = format!("deck {:?} uses unknown element {:?}", deck.id, deck.element);
                    return Err(ValidationError(message));
                }
                Some(element) if element.kind != ElementKind::Text => {
                    let message = format!(
                        "deck {:?} needs a text element, but {:?} is not one",
                        deck.id, element.id
                    );
                    return Err(ValidationError(message));
                }
                Some(_) => {}
            }
            if !slots.insert(deck.element.as_str()) {
                let message = format!("element {:?} already shows another deck", deck.element);
                return Err(ValidationError(message));
            }
            if !(deck.auto_hide_ms == 0 || (MIN_AUTO_HIDE_MS..=MAX_AUTO_HIDE_MS).contains(&deck.auto_hide_ms)) {
                let message = format!("auto_hide_ms must be 0 or {MIN_AUTO_HIDE_MS} to {MAX_AUTO_HIDE_MS}");
                return Err(prefix(&deck.id, ValidationError(message)));
            }
            deck.validate_cards()?;
        }
        Ok(())
    }
}

impl Deck {
    fn validate_cards(&self) -> Result<(), ValidationError> {
        if self.cards.is_empty() {
            let message = format!("deck {:?} needs at least one card", self.id);
            return Err(ValidationError(message));
        }
        check_count("cards per deck", self.cards.len(), MAX_CARDS)?;
        let mut ids = BTreeSet::new();
        for card in &self.cards {
            check_id(&card.id)?;
            check_name(&card.name)?;
            if !ids.insert(card.id.as_str()) {
                let message = format!("deck {:?} has duplicate card id {:?}", self.id, card.id);
                return Err(ValidationError(message));
            }
            let owner = format!("{}: {}", self.id, card.id);
            if card.props.visible.is_some() {
                let message = "cards must not set visible; raising and lowering the card does".to_owned();
                return Err(prefix(&owner, ValidationError(message)));
            }
            card.props.validate().map_err(|error| prefix(&owner, error))?;
            voice::validate_card(card).map_err(|error| prefix(&owner, error))?;
            if let Some(CardSignal::Fingers(count)) = card.signal
                && count > MAX_FINGERS
            {
                let message = format!("signal fingers must be 0 to {MAX_FINGERS}");
                return Err(prefix(&owner, ValidationError(message)));
            }
        }
        Ok(())
    }
}
