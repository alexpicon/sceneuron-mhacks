// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the voice matcher on the meeting, poker, and classroom packs: the wake word and its
//! near misses, commands, votes, and cards named by their name or text.

mod common;

use common::voice::{decks, held, raise, reason, said, unaddressed, with_decks};
use sceneuron_core::{Intent, VoiceMode, interpret};

#[test]
fn addressed_words_name_a_card_by_its_name_or_text() {
    let meeting = decks(&["meeting"]);
    assert_eq!(said("Sceneuron, wrap it up", &meeting), raise("meeting", "wrap-up"));
    assert_eq!(
        said("Hey Sceneuron, wrap it up please.", &meeting),
        raise("meeting", "wrap-up")
    );
    assert_eq!(
        said("sceneuron wrap up", &meeting),
        raise("meeting", "wrap-up"),
        "a word left out"
    );
    assert_eq!(
        said("Sceneuron we're off track", &meeting),
        raise("meeting", "off-track")
    );
    assert_eq!(said("Sceneuron, disagree", &meeting), raise("meeting", "disagree"));
    assert_eq!(said("Sceneuron, agree", &meeting), raise("meeting", "agree"));
    assert_eq!(
        held("wrap it up", &meeting),
        raise("meeting", "wrap-up"),
        "push to talk needs no wake word"
    );
    assert_eq!(
        held("can you show next topic", &meeting),
        raise("meeting", "next-topic")
    );
}

#[test]
fn the_wake_word_survives_small_slips_but_not_other_words() {
    let meeting = decks(&["meeting"]);
    for heard in [
        "Scene neuron, wrap it up",
        "Sceneron wrap it up",
        "ok so Sceneuron wrap it up",
    ] {
        assert_eq!(said(heard, &meeting), raise("meeting", "wrap-up"), "{heard}");
    }
    // "Neuron" is three slips from "Sceneuron", one more than a nine-letter wake word allows.
    for heard in [
        "seen your iron wrap it up",
        "wrap it up",
        "maybe Sceneuron wrap it up",
        "Neuron, wrap it up",
    ] {
        assert_eq!(said(heard, &meeting), unaddressed(), "{heard}");
    }
    assert_eq!(reason(said("Sceneuron.", &meeting)), "heard only the wake word");
    let custom = interpret(
        "Jarvis, agree",
        false,
        VoiceMode::AlwaysOn,
        "Jarvis",
        &with_decks(&meeting),
    );
    assert_eq!(custom, raise("meeting", "agree"));
}

#[test]
fn votes_name_a_card_exactly() {
    let both = decks(&["meeting", "poker"]);
    for heard in [
        "Sceneuron, my vote is eight",
        "Sceneuron vote eight",
        "Sceneuron, I vote 8",
        "Sceneuron vote for 8 points",
    ] {
        assert_eq!(said(heard, &both), raise("poker", "p8"), "{heard}");
    }
    assert_eq!(said("Sceneuron vote twenty one", &both), raise("poker", "p21"));
    assert_eq!(said("Sceneuron, vote question mark", &both), raise("poker", "unsure"));
    assert_eq!(said("Sceneuron, vote coffee", &both), raise("poker", "break"));
    assert_eq!(said("Sceneuron, I vote for a break", &both), raise("poker", "break"));
    assert_eq!(reason(said("Sceneuron, vote 4", &both)), "no card for the vote \"4\"");
    let twice = [decks(&["poker"]), decks(&["poker"])].concat();
    let refused = reason(said("Sceneuron vote 3", &twice));
    assert_eq!(
        refused,
        "the vote \"3\" is on more than one deck: Planning poker: 3 or Planning poker: 3"
    );
}

#[test]
fn lower_pause_and_resume_are_commands() {
    let all = decks(&["meeting", "poker"]);
    let both = Intent::Lower {
        decks: vec!["meeting".to_owned(), "poker".to_owned()],
    };
    assert_eq!(said("Sceneuron, lower", &all), both);
    assert_eq!(said("Sceneuron lower it please", &all), both);
    assert_eq!(said("Sceneuron, hide everything", &all), both);
    assert_eq!(held("lower the card", &all), both);
    let meeting = Intent::Lower {
        decks: vec!["meeting".to_owned()],
    };
    assert_eq!(said("Sceneuron, lower the meeting card", &all), meeting);
    assert_eq!(said("Sceneuron lower meeting signals", &all), meeting);
    assert_eq!(
        held("lower the poker card", &all),
        Intent::Lower {
            decks: vec!["poker".to_owned()]
        }
    );
    assert_eq!(said("Sceneuron, pause", &all), Intent::Pause);
    assert_eq!(said("Sceneuron pause automation", &all), Intent::Pause);
    assert_eq!(said("Sceneuron, resume", &all), Intent::Resume);
    assert_eq!(held("resume automation", &all), Intent::Resume);
}
