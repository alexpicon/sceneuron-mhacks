// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Builders for the parts of a pack: the pack and its one deck, the card slot, and cards.

use super::Pack;
use crate::decks::{Card, Deck};
use crate::model::{Element, ElementKind};
use crate::props::{ElementProps, PropsPatch};
use crate::signals::CardSignal;

/// A one-deck pack whose deck shares the pack's id and name.
pub(super) fn pack(
    id: &str,
    name: &str,
    description: &str,
    slot: Element,
    auto_hide_ms: u64,
    cards: Vec<Card>,
) -> Pack {
    let deck = Deck {
        id: id.to_owned(),
        name: name.to_owned(),
        element: slot.id.clone(),
        cards,
        auto_hide_ms,
        camera: false,
        voice: false,
        speak: false,
    };
    Pack {
        id: id.to_owned(),
        name: name.to_owned(),
        description: description.to_owned(),
        elements: vec![slot],
        decks: vec![deck],
    }
}

pub(super) fn slot(id: &str, name: &str, props: ElementProps) -> Element {
    Element {
        id: id.to_owned(),
        name: name.to_owned(),
        kind: ElementKind::Text,
        props,
    }
}

pub(super) fn card(id: &str, name: &str, text: &str) -> Card {
    Card {
        id: id.to_owned(),
        name: name.to_owned(),
        props: PropsPatch {
            text: Some(text.to_owned()),
            ..PropsPatch::default()
        },
        signal: None,
        phrases: Vec::new(),
        speech: None,
    }
}

pub(super) fn colored(id: &str, name: &str, text: &str, background: &str, color: &str) -> Card {
    let mut card = card(id, name, text);
    card.props.background = Some(background.to_owned());
    card.props.color = Some(color.to_owned());
    card
}

/// Cards whose name is also their text: `(id, label, background, color)`.
pub(super) fn labeled(cards: &[(&str, &str, &str, &str)]) -> Vec<Card> {
    cards
        .iter()
        .map(|(id, label, background, color)| colored(id, label, label, background, color))
        .collect()
}

/// Gives the card `id` a camera signal.
pub(super) fn signal(cards: &mut [Card], id: &str, signal: CardSignal) {
    for card in cards.iter_mut().filter(|card| card.id == id) {
        card.signal = Some(signal);
    }
}

/// Gives the card `id` a trigger phrase.
pub(super) fn phrase(cards: &mut [Card], id: &str, phrase: &str) {
    for card in cards.iter_mut().filter(|card| card.id == id) {
        card.phrases.push(phrase.to_owned());
    }
}

/// Gives the card `id` a speech line, for a card whose text a voice cannot say.
pub(super) fn speech(cards: &mut [Card], id: &str, speech: &str) {
    for card in cards.iter_mut().filter(|card| card.id == id) {
        card.speech = Some(speech.to_owned());
    }
}
