// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Shared test fixtures: the default project with a small card deck, readers for what an
//! element shows, and camera-signal, voice, voice request, subtitle, and Tengwar fixtures.

#![allow(dead_code)]

pub mod camera;
pub mod requests;
pub mod subtitles;
pub mod tengwar;
pub mod voice;

use sceneuron_core::{Card, Deck, Element, ElementKind, ElementProps, Engine, Project, PropsPatch, default_project};

/// The default project plus a "Votes" deck of three cards shown by the hidden text element
/// `slot`.
pub fn deck_project() -> Project {
    let mut project = default_project();
    project.elements.push(text_element("slot", "Vote slot"));
    project.decks.push(Deck {
        id: "votes".to_owned(),
        name: "Votes".to_owned(),
        element: "slot".to_owned(),
        cards: vec![
            card("yes", "Yes", "#16a34aff"),
            card("no", "No", "#dc2626ff"),
            card("maybe", "Maybe", "#2563ebff"),
        ],
        auto_hide_ms: 0,
        camera: false,
        voice: false,
        speak: false,
    });
    project
}

/// [`deck_project`] whose deck hides a raised card after `auto_hide_ms`.
pub fn auto_hide_project(auto_hide_ms: u64) -> Project {
    let mut project = deck_project();
    project.decks[0].auto_hide_ms = auto_hide_ms;
    project
}

pub fn engine(project: Project) -> Engine {
    Engine::new(project, 0).unwrap()
}

/// A hidden text element whose base text is "base".
pub fn text_element(id: &str, name: &str) -> Element {
    Element {
        id: id.to_owned(),
        name: name.to_owned(),
        kind: ElementKind::Text,
        props: ElementProps {
            visible: false,
            text: "base".to_owned(),
            ..ElementProps::default()
        },
    }
}

/// A card whose text is its name, on the given background.
pub fn card(id: &str, name: &str, background: &str) -> Card {
    Card {
        id: id.to_owned(),
        name: name.to_owned(),
        props: PropsPatch {
            text: Some(name.to_owned()),
            background: Some(background.to_owned()),
            ..PropsPatch::default()
        },
        signal: None,
        phrases: Vec::new(),
        speech: None,
    }
}

pub fn visible(engine: &Engine, id: &str) -> bool {
    engine.effective_props(id).unwrap().visible
}

pub fn text(engine: &Engine, id: &str) -> String {
    engine.effective_props(id).unwrap().text
}

/// The newest event's source and detail.
pub fn last_event(engine: &Engine, now_ms: u64) -> (String, String) {
    let event = engine.snapshot(now_ms).events.into_iter().next().unwrap();
    (event.source, event.detail)
}
