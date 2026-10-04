// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the voice parts of decks and cards: their JSON shape, the validation of trigger
//! phrases and speech lines, and the phrases and speech lines the packs bring, with voice and
//! speech off in every pack.

mod common;

use common::deck_project;
use sceneuron_core::{Canvas, Card, Project, packs};
use serde_json::json;

fn refusal(edit: impl FnOnce(&mut Card)) -> Option<String> {
    let mut project: Project = deck_project();
    edit(&mut project.decks[0].cards[0]);
    project.validate().err().map(|error| error.0)
}

#[test]
fn phrases_and_speech_appear_in_json_only_when_set() {
    let mut project = deck_project();
    let card = &mut project.decks[0].cards[0];
    card.phrases = vec!["yes please".to_owned()];
    card.speech = Some("Yes!".to_owned());
    let json = serde_json::to_value(&project.decks[0]).unwrap();
    assert_eq!(json["voice"], false);
    assert_eq!(json["speak"], false);
    assert_eq!(json["cards"][0]["phrases"], json!(["yes please"]));
    assert_eq!(json["cards"][0]["speech"], "Yes!");
    assert!(json["cards"][1].get("phrases").is_none() && json["cards"][1].get("speech").is_none());
    let read: Card = serde_json::from_value(json["cards"][0].clone()).unwrap();
    assert_eq!(&read, &project.decks[0].cards[0]);
}

#[test]
fn trigger_phrases_are_short_plain_words() {
    for good in [
        "ok",
        "let's take this offline",
        "We’re off-track, folks!",
        "vote 8?",
        "¿qué tal?",
        "मुझे सवाल है",
        "ขอถาม",
    ] {
        assert_eq!(refusal(|card| card.phrases = vec![good.to_owned()]), None, "{good}");
    }
    let four = |card: &mut Card| card.phrases = vec!["one two".to_owned(); 4];
    assert_eq!(refusal(four), None);
    let five = refusal(|card| card.phrases = vec!["one two".to_owned(); 5]).unwrap();
    assert_eq!(five, "votes: yes: a card has at most 4 trigger phrases");
    for bad in [
        "a",
        &"x".repeat(61),
        "50% off",
        "line\nbreak",
        "ขอถามหน่อย",
        "{{scene}}",
        "👍👍",
        "?!",
    ] {
        let refused = refusal(|card| card.phrases = vec![bad.to_owned()]).unwrap_or_else(|| panic!("{bad:?}"));
        assert!(refused.starts_with("votes: yes: trigger phrase"), "{refused}");
    }
}

#[test]
fn speech_lines_are_one_line_of_up_to_200_characters() {
    assert_eq!(refusal(|card| card.speech = Some("Yes, I agree.".to_owned())), None);
    assert_eq!(refusal(|card| card.speech = Some("x".repeat(200))), None);
    for bad in [String::new(), "x".repeat(201), "two\nlines".to_owned()] {
        let refused = refusal(|card| card.speech = Some(bad.clone())).unwrap();
        assert_eq!(
            refused,
            "votes: yes: the speech line must be 1-200 characters on one line"
        );
    }
}

#[test]
fn packs_bring_phrases_and_speech_lines_with_voice_and_speech_off() {
    let phrases: Vec<(String, String, Vec<String>)> = packs()
        .into_iter()
        .flat_map(|pack| pack.decks)
        .flat_map(|deck| {
            assert!(!deck.voice && !deck.speak, "{} starts quiet", deck.id);
            deck.cards
                .into_iter()
                .filter(|card| !card.phrases.is_empty())
                .map(move |card| (deck.id.clone(), card.id, card.phrases))
        })
        .collect();
    let expected = [
        ("meeting", "off-track", "we're getting off track"),
        ("meeting", "question", "I have a question"),
        ("meeting", "cant-hear", "I can't hear you"),
        ("meeting", "offline", "let's take this offline"),
        ("meeting", "next-topic", "next topic"),
        ("classroom", "question", "I have a question"),
        ("status", "brb", "I'll be right back"),
    ];
    let expected: Vec<(String, String, Vec<String>)> = expected
        .iter()
        .map(|(deck, card, phrase)| (deck.to_string(), card.to_string(), vec![phrase.to_string()]))
        .collect();
    assert_eq!(phrases, expected);
    let poker = sceneuron_core::pack_for_canvas("poker", &Canvas::default()).unwrap();
    let speech: Vec<_> = poker.decks[0]
        .cards
        .iter()
        .filter_map(|card| Some((card.id.as_str(), card.speech.as_deref()?)))
        .collect();
    assert_eq!(speech, [("unsure", "I'm not sure"), ("break", "Coffee break")]);
}
