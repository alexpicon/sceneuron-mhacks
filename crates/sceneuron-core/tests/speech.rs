// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises spoken cards: a card raised on a deck that speaks becomes a speech cue with a
//! rising nonce, says its speech line or its rendered text without emoji, and gets its audio
//! only while it is current and its card is still up; a covered deck says nothing.

mod common;

use common::{deck_project, engine};
use sceneuron_core::{
    CardAction, Engine, Heard, PropsPatch, SceneObservation, SpeechCue, VoiceMode, VoiceStatus, default_project,
    pack_for_canvas,
};

fn speaking() -> Engine {
    let mut project = deck_project();
    project.decks[0].speak = true;
    engine(project)
}

fn raise(engine: &mut Engine, card: &str, now_ms: u64) {
    engine.card("votes", Some(card), CardAction::Raise, now_ms).unwrap();
}

fn cue(engine: &Engine) -> SpeechCue {
    engine.snapshot(0).speech.expect("a speech cue")
}

#[test]
fn raising_a_card_on_a_speaking_deck_cues_it() {
    let mut engine = speaking();
    assert_eq!(engine.snapshot(0).speech, None);
    raise(&mut engine, "yes", 100);
    let first = cue(&engine);
    assert_eq!(
        (
            first.text.as_str(),
            first.deck.as_str(),
            first.card.as_str(),
            first.at_ms,
            first.audio
        ),
        ("Yes", "votes", "yes", 100, None)
    );
    raise(&mut engine, "no", 200);
    let second = cue(&engine);
    assert_eq!(second.text, "No");
    assert!(second.nonce > first.nonce, "nonces only grow");
    engine
        .trigger_effect("slot", sceneuron_core::Effect::Flash, 300)
        .unwrap();
    raise(&mut engine, "maybe", 400);
    assert!(
        cue(&engine).nonce > second.nonce + 1,
        "effects and cues share the counter"
    );
}

#[test]
fn quiet_decks_lowering_and_raising_the_shown_card_again_cue_nothing() {
    let mut engine = engine(deck_project());
    raise(&mut engine, "yes", 100);
    assert_eq!(engine.snapshot(0).speech, None);
    let mut engine = speaking();
    raise(&mut engine, "yes", 100);
    let first = cue(&engine);
    raise(&mut engine, "yes", 100);
    engine.card("votes", None, CardAction::Lower, 200).unwrap();
    assert_eq!(cue(&engine), first);
}

#[test]
fn a_cue_says_the_speech_line_or_the_rendered_text_without_emoji() {
    let pack = pack_for_canvas("poker", &Default::default()).unwrap();
    let mut project = default_project().add_pack(&pack).unwrap();
    let meeting = pack_for_canvas("meeting", &Default::default()).unwrap();
    project = project.add_pack(&meeting).unwrap();
    for deck in &mut project.decks {
        deck.speak = true;
    }
    project.decks[1].cards[0].props.text = Some("⏱️ Time's up, {{showing-item}}!".to_owned());
    let mut engine = Engine::new(project, 0).unwrap();
    let spoken = |engine: &mut Engine, deck: &str, card: &str| {
        engine.card(deck, Some(card), CardAction::Raise, 10).unwrap();
        engine.speech().unwrap().text.clone()
    };
    assert_eq!(spoken(&mut engine, "poker", "p8"), "8");
    assert_eq!(spoken(&mut engine, "poker", "unsure"), "I'm not sure");
    assert_eq!(spoken(&mut engine, "poker", "break"), "Coffee break");
    assert_eq!(spoken(&mut engine, "meeting", "agree"), "Agree");
    let observation = SceneObservation {
        caption: String::new(),
        verdicts: vec![sceneuron_core::Verdict {
            condition: "showing-item".to_owned(),
            holds: true,
            confidence: 0.9,
            detail: "a mug".to_owned(),
            reason: String::new(),
        }],
    };
    engine.observe_scene(observation, 20);
    assert_eq!(spoken(&mut engine, "meeting", "wrap-up"), "Time's up, a mug!");
}

#[test]
fn audio_attaches_only_to_the_current_cue_while_its_card_is_up() {
    let mut engine = speaking();
    raise(&mut engine, "yes", 100);
    let first = cue(&engine).nonce;
    raise(&mut engine, "no", 200);
    let second = cue(&engine).nonce;
    assert!(
        !engine.speech_ready(first, "aaa", 300).changed,
        "a replaced cue stays silent"
    );
    let revision = engine.revision();
    assert!(engine.speech_ready(second, "bbb", 300).changed);
    assert_eq!(engine.revision(), revision + 1);
    assert_eq!(cue(&engine).audio.as_deref(), Some("bbb"));
    assert!(!engine.speech_ready(second, "ccc", 400).changed, "audio is set once");
    raise(&mut engine, "maybe", 500);
    let third = cue(&engine).nonce;
    engine.card("votes", None, CardAction::Lower, 600).unwrap();
    assert!(
        !engine.speech_ready(third, "ddd", 700).changed,
        "a lowered card is not spoken"
    );
    assert_eq!(cue(&engine).audio, None);
}

#[test]
fn voice_raises_are_spoken_too() {
    let mut project = deck_project();
    project.decks[0].speak = true;
    project.decks[0].voice = true;
    let mut engine = engine(project);
    let status = VoiceStatus {
        enabled: true,
        mode: VoiceMode::PushToTalk,
        ..VoiceStatus::default()
    };
    engine.set_voice_status(status, 0);
    let heard = Heard {
        text: "maybe".to_owned(),
        is_final: true,
        addressed: true,
    };
    engine.hear(&heard, 10);
    assert_eq!(cue(&engine).text, "Maybe");
    let json = serde_json::to_value(engine.snapshot(10).speech).unwrap();
    assert_eq!(json["audio"], serde_json::Value::Null);
    assert_eq!(json["card"], "maybe");
}

#[test]
fn a_covered_deck_never_says_its_hidden_card() {
    let mut engine = speaking();
    let cover = || {
        Some(PropsPatch {
            text: Some("✓ Voted".to_owned()),
            ..PropsPatch::default()
        })
    };
    engine.cover_card("votes", cover(), 50).unwrap();
    raise(&mut engine, "yes", 100);
    assert_eq!(engine.snapshot(100).speech, None, "a hidden vote is not said");
    engine.cover_card("votes", None, 200).unwrap();
    raise(&mut engine, "no", 300);
    let nonce = cue(&engine).nonce;
    engine.cover_card("votes", cover(), 400).unwrap();
    assert!(
        !engine.speech_ready(nonce, "aaa", 500).changed,
        "a card covered before its audio came stays silent"
    );
    assert_eq!(cue(&engine).audio, None);
}
