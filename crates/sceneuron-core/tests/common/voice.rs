// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Voice fixtures: pack decks with voice on, words heard in either mode, the intents they
//! should give, and engines that listen.

use sceneuron_core::{
    Canvas, Deck, Engine, Heard, Intent, Project, VoiceMode, VoiceStatus, default_project, interpret, pack_for_canvas,
};

/// The first deck of each pack in `ids`, with voice on.
pub fn decks(ids: &[&str]) -> Vec<Deck> {
    ids.iter()
        .map(|id| {
            let mut deck = pack_for_canvas(id, &Canvas::default()).unwrap().decks.remove(0);
            deck.voice = true;
            deck
        })
        .collect()
}

/// A project with only `decks`, as the matcher reads it: no elements, and fun reactions on.
pub fn with_decks(decks: &[Deck]) -> Project {
    Project {
        decks: decks.to_vec(),
        ..Project::default()
    }
}

/// Final words heard in always-on mode, addressed only if they start with the wake word.
pub fn said(text: &str, decks: &[Deck]) -> Intent {
    interpret(text, false, VoiceMode::AlwaysOn, "Sceneuron", &with_decks(decks))
}

/// Final words heard while push-to-talk was held.
pub fn held(text: &str, decks: &[Deck]) -> Intent {
    interpret(text, true, VoiceMode::PushToTalk, "Sceneuron", &with_decks(decks))
}

/// Addressed words for the AI.
pub fn ask(text: &str) -> Intent {
    Intent::Ask { text: text.to_owned() }
}

/// Raising a card by command.
pub fn raise(deck: &str, card: &str) -> Intent {
    Intent::Raise {
        deck: deck.to_owned(),
        card: card.to_owned(),
        phrase: None,
    }
}

/// Raising a card by its trigger phrase.
pub fn phrase_raise(deck: &str, card: &str, phrase: &str) -> Intent {
    Intent::Raise {
        deck: deck.to_owned(),
        card: card.to_owned(),
        phrase: Some(phrase.to_owned()),
    }
}

/// Words that were not meant for Sceneuron and do nothing.
pub fn unaddressed() -> Intent {
    Intent::Ignore {
        addressed: false,
        reason: String::new(),
    }
}

/// Why addressed words do nothing.
pub fn reason(intent: Intent) -> String {
    match intent {
        Intent::Ignore { reason, .. } => reason,
        other => panic!("expected nothing to happen, got {other:?}"),
    }
}

/// The default project with the meeting and poker packs, voice on for both, listening in `mode`.
pub fn meeting_engine(mode: VoiceMode) -> Engine {
    let mut project = default_project();
    for id in ["meeting", "poker"] {
        project = project
            .add_pack(&pack_for_canvas(id, &Canvas::default()).unwrap())
            .unwrap();
    }
    for deck in &mut project.decks {
        deck.voice = true;
    }
    let mut engine = Engine::new(project, 0).unwrap();
    engine.set_voice_status(on(mode), 0);
    engine
}

/// Voice control on in `mode`.
pub fn on(mode: VoiceMode) -> VoiceStatus {
    VoiceStatus {
        enabled: true,
        mode,
        ..VoiceStatus::default()
    }
}

/// Final words, not said while push-to-talk was held.
pub fn final_words(text: &str) -> Heard {
    Heard {
        text: text.to_owned(),
        is_final: true,
        addressed: false,
    }
}

/// The `count` newest events, oldest first, as (source, detail).
pub fn events(engine: &Engine, count: usize) -> Vec<(String, String)> {
    let mut recent: Vec<_> = engine
        .snapshot(0)
        .events
        .into_iter()
        .take(count)
        .map(|event| (event.source, event.detail))
        .collect();
    recent.reverse();
    recent
}

pub fn pair(source: &str, detail: &str) -> (String, String) {
    (source.to_owned(), detail.to_owned())
}
