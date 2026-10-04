// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The words the room shows: what `{{room}}` renders, and each revealed card by its name in this
//! Sceneuron's deck.

use sceneuron_core::Deck;

use crate::backend::{RoomView, Shown};

/// "Voted: 3 of 5" before the reveal, then each vote by name in deck order with the average of
/// the cards that are numbers, such as "Maya 3 · You 5 · Sam 8 · Average 5.3", or "No votes"
/// when nobody voted.
pub fn summary(view: &RoomView, deck: Option<&Deck>) -> String {
    if !view.revealed {
        let voted = view.members.iter().filter(|member| member.voted).count();
        return format!("Voted: {voted} of {}", view.members.len());
    }
    let shown = in_deck_order(&view.results, deck);
    if shown.is_empty() {
        return "No votes".to_owned();
    }
    let labels: Vec<String> = shown.iter().map(|shown| label(&shown.card, deck)).collect();
    let mut parts: Vec<String> = shown
        .iter()
        .zip(&labels)
        .map(|(shown, label)| format!("{} {label}", voter(view, &shown.id)))
        .collect();
    let numbers: Vec<f64> = labels
        .iter()
        .filter_map(|label| label.trim().parse::<f64>().ok())
        .collect();
    if !numbers.is_empty() {
        let average = numbers.iter().sum::<f64>() / numbers.len() as f64;
        let rounded = (average * 10.0).round() / 10.0;
        parts.push(format!("Average {rounded}"));
    }
    parts.join(" · ")
}

/// Who voted: "You" for this Sceneuron, else the member's name, or "Someone" once they left.
fn voter(view: &RoomView, id: &str) -> String {
    match view.members.iter().find(|member| member.id == id) {
        Some(member) if member.me => "You".to_owned(),
        Some(member) => member.name.clone(),
        None => "Someone".to_owned(),
    }
}

/// A card's name in `deck`, or its id when this Sceneuron's deck has no such card.
pub fn label(card: &str, deck: Option<&Deck>) -> String {
    deck.and_then(|deck| deck.card(card))
        .map_or_else(|| card.to_owned(), |found| found.name.clone())
}

/// The revealed votes ordered as the cards are in `deck`, as a spread reads from low to high;
/// cards the deck does not have come last, in the order given.
pub fn in_deck_order<'a>(results: &'a [Shown], deck: Option<&Deck>) -> Vec<&'a Shown> {
    let position = |card: &str| {
        deck.and_then(|deck| deck.cards.iter().position(|candidate| candidate.id == card))
            .unwrap_or(usize::MAX)
    };
    let mut ordered: Vec<&Shown> = results.iter().collect();
    ordered.sort_by_key(|shown| position(&shown.card));
    ordered
}

#[cfg(test)]
mod tests {
    use sceneuron_core::{Card, PropsPatch};

    use super::*;
    use crate::backend::Member;

    fn deck() -> Deck {
        let card = |id: &str, name: &str| Card {
            id: id.to_owned(),
            name: name.to_owned(),
            props: PropsPatch::default(),
            signal: None,
            phrases: Vec::new(),
            speech: None,
        };
        Deck {
            id: "poker".to_owned(),
            name: "Planning poker".to_owned(),
            element: "poker-card".to_owned(),
            cards: vec![card("p3", "3"), card("p5", "5"), card("p8", "8")],
            auto_hide_ms: 0,
            camera: false,
            voice: false,
            speak: false,
        }
    }

    fn member(id: &str, voted: bool) -> Member {
        Member {
            id: id.to_owned(),
            name: id.to_uppercase(),
            voted,
            me: false,
        }
    }

    fn shown(id: &str, card: &str) -> Shown {
        Shown {
            id: id.to_owned(),
            card: card.to_owned(),
        }
    }

    #[test]
    fn before_the_reveal_it_counts_votes() {
        let view = RoomView {
            members: vec![member("a", true), member("b", false), member("c", true)],
            ..RoomView::default()
        };
        assert_eq!(summary(&view, Some(&deck())), "Voted: 2 of 3");
        assert_eq!(summary(&RoomView::default(), None), "Voted: 0 of 0");
    }

    #[test]
    fn after_the_reveal_it_lists_each_vote_by_name_in_deck_order_with_the_average() {
        let view = RoomView {
            revealed: true,
            results: vec![
                shown("a", "p8"),
                shown("b", "p3"),
                shown("c", "custom"),
                shown("d", "p5"),
                shown("e", "p5"),
            ],
            ..RoomView::default()
        };
        let named = RoomView {
            members: vec![
                member("a", true),
                member("b", true),
                member("c", true),
                member("d", true),
            ],
            ..view.clone()
        };
        assert_eq!(
            summary(&named, Some(&deck())),
            "B 3 · D 5 · Someone 5 · A 8 · C custom · Average 5.3"
        );
        let mut mine = named.clone();
        mine.members[1].me = true;
        assert!(summary(&mine, Some(&deck())).starts_with("You 3 · "));
        assert_eq!(
            summary(&view, None),
            "Someone p8 · Someone p3 · Someone custom · Someone p5 · Someone p5",
            "without the deck, ids in order and no average"
        );
    }

    #[test]
    fn a_reveal_without_votes_says_so() {
        let view = RoomView {
            revealed: true,
            members: vec![member("a", false)],
            ..RoomView::default()
        };
        assert_eq!(summary(&view, Some(&deck())), "No votes");
    }
}
