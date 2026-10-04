// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the packs: their contents, validation on common, wide, and extreme canvases, and
//! adding them to a project. The packs without a deck have tests of their own, the subtitles bar
//! in subtitles_pack.rs and the live description bar in seen_templates.rs, and the packs' camera
//! signals are in pack_signals.rs.

use std::collections::BTreeSet;

use sceneuron_core::{Canvas, ElementKind, EnterAnimation, Pack, Project, default_project, pack_for_canvas, packs};

const CANVASES: [(u32, u32); 6] = [
    (1920, 1080),
    (1280, 720),
    (2560, 1080),
    (3440, 1440),
    (16, 16),
    (8192, 8192),
];

fn on(width: u32, height: u32) -> Project {
    Project {
        canvas: Canvas { width, height },
        ..default_project()
    }
}

fn card_ids(pack: &Pack) -> Vec<&str> {
    pack.decks[0].cards.iter().map(|card| card.id.as_str()).collect()
}

/// The packs that bring a deck of cards.
fn card_packs() -> Vec<Pack> {
    packs().into_iter().filter(|pack| !pack.decks.is_empty()).collect()
}

#[test]
fn the_packs_have_their_documented_ids_cards_and_delays() {
    let ids: Vec<_> = packs().into_iter().map(|pack| (pack.id, pack.name)).collect();
    let ids: Vec<_> = ids.iter().map(|(id, name)| (id.as_str(), name.as_str())).collect();
    assert_eq!(
        ids,
        [
            ("meeting", "Meeting signals"),
            ("poker", "Planning poker"),
            ("classroom", "Classroom"),
            ("status", "Status"),
            ("subtitles", "Subtitles"),
            ("live-description", "Live description")
        ]
    );
    let all = card_packs();
    let expected: [(&[&str], u64); 4] = [
        (
            &[
                "wrap-up",
                "off-track",
                "question",
                "agree",
                "disagree",
                "cant-hear",
                "offline",
                "next-topic",
            ],
            15_000,
        ),
        (
            &["p0", "p1", "p2", "p3", "p5", "p8", "p13", "p21", "unsure", "break"],
            0,
        ),
        (
            &[
                "hand", "question", "repeat", "slower", "got-it", "answer-a", "answer-b", "answer-c", "answer-d",
            ],
            30_000,
        ),
        (&["muted", "brb", "lunch", "focus", "presenting", "on-call"], 0),
    ];
    for (pack, (cards, auto_hide_ms)) in all.iter().zip(expected) {
        assert_eq!(card_ids(pack), cards, "{}", pack.id);
        assert_eq!(pack.decks[0].auto_hide_ms, auto_hide_ms, "{}", pack.id);
        assert!(!pack.description.is_empty());
    }
    let poker = &all[1].decks[0];
    let values: Vec<_> = poker
        .cards
        .iter()
        .map(|card| card.props.text.as_deref().unwrap())
        .collect();
    assert_eq!(values, ["0", "1", "2", "3", "5", "8", "13", "21", "?", "☕"]);
    assert_eq!(all[0].decks[0].card("wrap-up").unwrap().name, "Wrap it up");
    assert_eq!(all[3].decks[0].card("muted").unwrap().name, "🔇 I'm on mute");
}

#[test]
fn ids_are_namespaced_and_slots_are_hidden_text_elements_that_animate_in() {
    for pack in card_packs() {
        assert_eq!(pack.elements.len(), 1, "{}", pack.id);
        assert_eq!(pack.decks.len(), 1, "{}", pack.id);
        let slot = &pack.elements[0];
        let deck = &pack.decks[0];
        assert_eq!(slot.id, format!("{}-card", pack.id));
        assert_eq!(
            (deck.id.as_str(), deck.name.as_str()),
            (pack.id.as_str(), pack.name.as_str())
        );
        assert_eq!(deck.element, slot.id);
        assert_eq!(slot.kind, ElementKind::Text);
        assert!(!slot.props.visible, "{} starts hidden", slot.id);
        assert_ne!(slot.props.enter, EnterAnimation::None, "{}", slot.id);
        for card in &deck.cards {
            assert!(card.props.visible.is_none(), "{}", card.id);
            assert!(
                card.props.text.as_deref().is_some_and(|text| !text.is_empty()),
                "{}",
                card.id
            );
        }
    }
}

#[test]
fn meeting_cards_have_distinct_readable_colors() {
    let meeting = pack_for_canvas("meeting", &Canvas::default()).unwrap();
    let cards = &meeting.decks[0].cards;
    let backgrounds: BTreeSet<_> = cards
        .iter()
        .map(|card| card.props.background.clone().unwrap())
        .collect();
    assert_eq!(backgrounds.len(), cards.len());
    assert!(cards.iter().all(|card| card.props.color.is_some()));
    assert!(
        meeting.elements[0].props.font_size >= 64.0,
        "meeting apps compress video"
    );
}

#[test]
fn every_pack_validates_alone_and_together_on_every_canvas() {
    for (width, height) in CANVASES {
        let canvas = Canvas { width, height };
        let mut together = on(width, height);
        for pack in packs() {
            let scaled = pack_for_canvas(&pack.id, &canvas).unwrap();
            on(width, height).add_pack(&scaled).unwrap();
            Project {
                elements: Vec::new(),
                rules: Vec::new(),
                ..on(width, height)
            }
            .add_pack(&scaled)
            .unwrap();
            together = together.add_pack(&scaled).unwrap();
        }
        assert_eq!(together.decks.len(), 4, "{width}x{height}");
        assert!(together.element("subtitles-translated").is_some(), "{width}x{height}");
    }
}

#[test]
fn adding_a_pack_appends_its_slot_and_deck_and_nothing_else() {
    let base = default_project();
    let pack = pack_for_canvas("poker", &base.canvas).unwrap();
    let next = base.add_pack(&pack).unwrap();
    assert_eq!(next.elements.len(), base.elements.len() + 1);
    assert_eq!(next.elements.last().unwrap().id, "poker-card", "new slots draw on top");
    assert_eq!(next.decks, pack.decks);
    assert_eq!((&next.rules, &next.conditions), (&base.rules, &base.conditions));
    assert_eq!(base, default_project(), "the input project is untouched");
}

#[test]
fn adding_a_pack_twice_or_over_a_used_id_is_refused() {
    let pack = pack_for_canvas("poker", &Canvas::default()).unwrap();
    let once = default_project().add_pack(&pack).unwrap();
    let twice = once.add_pack(&pack).unwrap_err();
    assert_eq!(twice.0, "Planning poker is already in this project");
    let mut taken = default_project();
    taken.elements.push(pack.elements[0].clone());
    let error = taken.add_pack(&pack).unwrap_err();
    assert!(error.0.contains("element id \"poker-card\""), "{error}");
}

#[test]
fn adding_a_pack_respects_project_limits() {
    let pack = pack_for_canvas("meeting", &Canvas::default()).unwrap();
    let mut full = default_project();
    let filler = full.elements[1].clone();
    while full.elements.len() < 64 {
        let mut element = filler.clone();
        element.id = format!("filler-{}", full.elements.len());
        full.elements.push(element);
    }
    full.validate().unwrap();
    assert!(full.add_pack(&pack).unwrap_err().0.contains("at most 64 elements"));
}

#[test]
fn packs_serialize_to_the_documented_shape() {
    let json = serde_json::to_value(pack_for_canvas("status", &Canvas::default()).unwrap()).unwrap();
    let keys: Vec<_> = json.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(keys.len(), 5);
    for key in ["id", "name", "description", "elements", "decks"] {
        assert!(keys.contains(&key), "missing {key}");
    }
    assert_eq!(json["decks"][0]["cards"][0]["props"]["text"], "🔇 I'm on mute");
    assert!(json["decks"][0]["cards"][0]["props"].get("visible").is_none());
}
