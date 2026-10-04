// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Simulated plans for meetings, planning poker, and classrooms: the matching ready-made pack,
//! proposed as its card slot and deck so it is reviewed and applied like any other plan.

use sceneuron_core::{Canvas, EditOp, Pack, Project, pack_for_canvas};

use super::features::Planner;
use super::scenes::mentions;

/// Pack ids and the words that ask for them, in the order a plan adds them.
const KEYWORDS: [(&str, &[&str]); 3] = [
    ("poker", &["poker", "sprint", "estimat", "story point", "scrum"]),
    (
        "classroom",
        &["classroom", "student", "teach", "lecture", "lesson", "school"],
    ),
    ("meeting", &["meeting", "standup", "video call"]),
];

/// Call apps ask for meeting signals only when no other pack matched: sprint planning on Zoom
/// wants poker cards.
const CALL_APPS: [&str; 4] = ["zoom", "teams", "webex", "google meet"];

/// The packs a lowercase request asks for, scaled to `canvas`.
pub(crate) fn packs_for(words: &str, canvas: &Canvas) -> Vec<Pack> {
    let has = |keywords: &[&str]| keywords.iter().any(|keyword| mentions(words, keyword));
    // "class" is matched as a whole word, so "classic" and "classy" stay restyle requests.
    let class = words
        .split(|c: char| !c.is_alphanumeric())
        .any(|word| word == "class" || word == "classes");
    let mut ids: Vec<&str> = KEYWORDS
        .iter()
        .filter(|(id, keywords)| has(keywords) || (*id == "classroom" && class))
        .map(|(id, _)| *id)
        .collect();
    if ids.is_empty() && has(&CALL_APPS) {
        ids.push("meeting");
    }
    ids.into_iter().filter_map(|id| pack_for_canvas(id, canvas)).collect()
}

/// Whether the project already has one of the pack's decks.
pub(crate) fn in_project(project: &Project, pack: &Pack) -> bool {
    pack.decks.iter().any(|deck| project.deck(&deck.id).is_some())
}

/// Adds the pack's card slots and decks unless the project already has one of its decks. A slot
/// id the project already uses gets a numbered id. Returns what this does, for the plan summary.
pub(crate) fn add_pack(planner: &mut Planner, mut pack: Pack) -> String {
    if in_project(planner.project(), &pack) {
        return format!("keep the {} deck you already have", pack.name);
    }
    for element in &mut pack.elements {
        let id = planner.element_id(&element.id);
        for deck in pack.decks.iter_mut().filter(|deck| deck.element == element.id) {
            deck.element.clone_from(&id);
        }
        element.id = id;
        planner.push(EditOp::AddElement {
            element: element.clone(),
        });
    }
    let cards: usize = pack.decks.iter().map(|deck| deck.cards.len()).sum();
    for deck in pack.decks {
        planner.push(EditOp::AddDeck { deck });
    }
    format!("add the {} deck of {cards} cards to raise during calls", pack.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(words: &str) -> Vec<String> {
        packs_for(words, &Canvas::default())
            .into_iter()
            .map(|pack| pack.id)
            .collect()
    }

    #[test]
    fn requests_ask_for_packs_by_topic() {
        assert_eq!(ids("i run sprint planning on zoom"), ["poker"]);
        assert_eq!(ids("we estimate story points"), ["poker"]);
        assert_eq!(ids("signals for our team meetings"), ["meeting"]);
        assert_eq!(ids("cards for my zoom calls"), ["meeting"]);
        assert_eq!(ids("i teach a class on teams"), ["classroom"]);
        assert_eq!(ids("my students raise hands"), ["classroom"]);
        assert_eq!(ids("planning poker and meeting signals"), ["poker", "meeting"]);
    }

    #[test]
    fn other_requests_ask_for_no_pack() {
        for words in ["make it classy", "a classic look", "say hi when i wave", "my cat jumps"] {
            assert!(ids(words).is_empty(), "{words}");
        }
    }
}
