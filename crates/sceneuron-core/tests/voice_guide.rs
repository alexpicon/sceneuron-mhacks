// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the guide to what can be said: on the starter project it lists every reaction,
//! trigger phrase, voice deck, theme, element that is no card slot, and the other commands, each
//! way to say them does what its entry says when read back, a reaction named like a card is
//! listed only with "play", the theme names are the overlay's own, and the guide follows the
//! project's switches.

mod common;

use std::fs;
use std::path::PathBuf;

use common::voice::on;
use sceneuron_core::{Engine, Flood, Intent, Project, VoiceGuide, VoiceMode, interpret, starter_project};

fn guide(project: Project) -> VoiceGuide {
    let mut engine = Engine::new(project, 0).unwrap();
    engine.set_voice_status(on(VoiceMode::AlwaysOn), 0);
    engine.voice_guide()
}

fn entries<'g>(guide: &'g VoiceGuide, title: &str) -> Vec<&'g sceneuron_core::GuideEntry> {
    let group = guide.groups.iter().find(|group| group.title == title);
    group.map(|group| group.entries.iter().collect()).unwrap_or_default()
}

fn ways(guide: &VoiceGuide, title: &str) -> Vec<String> {
    entries(guide, title)
        .iter()
        .flat_map(|entry| entry.say.clone())
        .collect()
}

#[test]
fn the_starter_guide_lists_every_kind_of_command_in_order() {
    let guide = guide(starter_project().unwrap());
    let titles: Vec<&str> = guide.groups.iter().map(|group| group.title.as_str()).collect();
    assert_eq!(
        titles,
        [
            "Reactions",
            "Trigger phrases",
            "Cards",
            "Themes",
            "Overlay elements",
            "Camera, subtitles, and automation",
            "Anything else",
        ]
    );
    assert_eq!(
        (guide.wake_word.as_str(), guide.mode),
        ("Sceneuron", VoiceMode::AlwaysOn)
    );
    let addressed: Vec<bool> = guide.groups.iter().map(|group| group.addressed).collect();
    assert_eq!(addressed, [true, false, true, true, true, true, true]);
    let reactions = ways(&guide, "Reactions");
    for way in [
        "mind blown",
        "play ship it",
        "tongue out",
        "could have been an email",
        "play coffee",
    ] {
        assert!(reactions.contains(&way.to_owned()), "{way} in {reactions:?}");
    }
    assert!(
        !reactions.contains(&"coffee break".to_owned()),
        "Planning poker's ☕ has that name too"
    );
    let phrases = ways(&guide, "Trigger phrases");
    for way in [
        "could have been an email",
        "blew my mind",
        "coffee break",
        "let's take this offline",
    ] {
        assert!(phrases.contains(&way.to_owned()), "{way} in {phrases:?}");
    }
    let elements = ways(&guide, "Overlay elements");
    assert!(elements.contains(&"show be right back".to_owned()), "{elements:?}");
    assert!(
        elements.contains(&"hide translated subtitles".to_owned()),
        "{elements:?}"
    );
    assert!(
        !elements.iter().any(|way| way.contains("poker card")),
        "card slots are left to their cards"
    );
    let cards = ways(&guide, "Cards");
    for way in ["Wrap it up", "my vote is 0", "lower", "lower planning poker"] {
        assert!(cards.contains(&way.to_owned()), "{way} in {cards:?}");
    }
    assert_eq!(
        ways(&guide, "Themes")[..4],
        ["theme off", "no theme", "graphite theme", "use the graphite theme"]
    );
}

#[test]
fn every_way_listed_does_what_its_entry_says() {
    let project = starter_project().unwrap();
    let guide = guide(project.clone());
    for group in &guide.groups {
        for entry in &group.entries {
            for way in &entry.say {
                let intent = interpret(way, group.addressed, VoiceMode::AlwaysOn, "Sceneuron", &project);
                let ok = match (group.title.as_str(), &intent) {
                    ("Anything else", Intent::Ask { .. }) => true,
                    ("Trigger phrases", Intent::Play { .. } | Intent::Raise { phrase: Some(_), .. }) => true,
                    (_, Intent::Ask { .. } | Intent::Ignore { .. } | Intent::Unclear { .. }) => false,
                    _ => group.title != "Trigger phrases" && group.title != "Anything else",
                };
                assert!(ok, "{}: {way:?} reads as {intent:?}", group.title);
            }
        }
    }
}

#[test]
fn the_guide_follows_the_switches_and_tells_how_to_raise_cards() {
    let mut project = starter_project().unwrap();
    project.reactions.voice_off = vec![Flood::ShipIt];
    project.decks.iter_mut().for_each(|deck| deck.voice = false);
    let guide = guide(project);
    let phrases = ways(&guide, "Trigger phrases");
    assert!(!phrases.contains(&"ship it".to_owned()) && phrases.contains(&"mind blown".to_owned()));
    assert!(!phrases.contains(&"let's take this offline".to_owned()));
    let cards = entries(&guide, "Cards");
    assert_eq!(cards.len(), 1);
    assert!(cards[0].say.is_empty() && cards[0].does.contains("Turn a deck's voice on"));
    assert!(
        ways(&guide, "Reactions").contains(&"coffee break".to_owned()),
        "no card has it now"
    );
}

#[test]
fn the_themes_are_named_as_the_overlay_names_them() {
    let guide = guide(starter_project().unwrap());
    let said: Vec<String> = entries(&guide, "Themes")
        .iter()
        .map(|entry| entry.does.clone())
        .collect();
    let themes = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/themes");
    let mut checked = 0;
    for file in fs::read_dir(themes).unwrap() {
        let source = fs::read_to_string(file.unwrap().path()).unwrap();
        let name = source
            .lines()
            .find_map(|line| line.trim().strip_prefix("name: \""))
            .and_then(|rest| rest.strip_suffix("\","))
            .expect("a theme name");
        let expected = format!("switches the overlay to the {name} theme");
        assert!(said.contains(&expected), "{expected:?} in {said:?}");
        checked += 1;
    }
    assert_eq!(checked, 6);
}
