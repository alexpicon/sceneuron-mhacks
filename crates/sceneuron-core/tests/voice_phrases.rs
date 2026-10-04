// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises trigger phrases: they need no wake word in always-on mode, settle addressed words
//! that name no card clearly, count as automation, never raise the card already shown or a card
//! just spoken that the microphone hears again, and leave the rest of what was said out of the
//! heard line, the event log, and the snapshot.

mod common;

use common::last_event;
use common::voice::{
    decks, events, final_words, meeting_engine, on, pair, phrase_raise, raise, reason, said, with_decks,
};
use sceneuron_core::{
    Canvas, Engine, Heard, Intent, VoiceMode, VoiceStatus, default_project, interpret, pack_for_canvas,
};

#[test]
fn trigger_phrases_need_no_wake_word_in_always_on_mode() {
    let meeting = decks(&["meeting"]);
    assert_eq!(
        said("OK so let's take this offline, everyone", &meeting),
        phrase_raise("meeting", "offline", "let's take this offline")
    );
    assert_eq!(
        said("I think we're getting off track here", &meeting),
        phrase_raise("meeting", "off-track", "we're getting off track")
    );
    assert_eq!(
        said("Next topic!", &meeting),
        phrase_raise("meeting", "next-topic", "next topic")
    );
    let push = interpret(
        "let's take this offline",
        false,
        VoiceMode::PushToTalk,
        "Sceneuron",
        &with_decks(&meeting),
    );
    assert!(matches!(push, Intent::Ignore { addressed: false, .. }), "{push:?}");
    let both = decks(&["meeting", "classroom"]);
    let unclear = Intent::Unclear {
        phrase: "I have a question".to_owned(),
        reason: "the trigger phrase \"I have a question\" could be Meeting signals: I have a question or \
                 Classroom: ❓ Question"
            .to_owned(),
    };
    assert_eq!(said("well I have a question", &both), unclear);
}

#[test]
fn addressed_words_that_name_no_card_clearly_fall_back_to_a_phrase() {
    let meeting = decks(&["meeting"]);
    assert_eq!(said("Sceneuron, next topic", &meeting), raise("meeting", "next-topic"));
    assert_eq!(
        said("Sceneuron, gibberish next topic", &meeting),
        raise("meeting", "next-topic"),
        "the words name the card well enough"
    );
    assert_eq!(
        said("Sceneuron, agree, next topic", &meeting),
        raise("meeting", "next-topic"),
        "a one-word name is too little of what was said"
    );
    assert_eq!(
        said("Sceneuron, wrap it up, next topic", &meeting),
        phrase_raise("meeting", "next-topic", "next topic"),
        "two cards tie, the phrase decides"
    );
    let push = interpret(
        "wrap it up, next topic",
        true,
        VoiceMode::PushToTalk,
        "Sceneuron",
        &with_decks(&meeting),
    );
    assert_eq!(
        reason(push),
        "\"wrap it up next topic\" could be Meeting signals: Wrap it up or Meeting signals: Next topic"
    );
}

#[test]
fn trigger_phrases_are_automation_but_commands_work_while_paused() {
    let mut engine = meeting_engine(VoiceMode::AlwaysOn);
    engine.set_automation(false, 5);
    engine.hear(&final_words("we should take it... let's take this offline"), 10);
    assert_eq!(engine.raised_card("meeting"), None);
    assert_eq!(
        events(&engine, 2),
        [
            pair("voice", "heard the trigger phrase \"let's take this offline\""),
            pair("voice", "a trigger phrase, but automation is paused")
        ]
    );
    engine.hear(&final_words("Sceneuron, next topic"), 20);
    assert_eq!(engine.raised_card("meeting"), Some("next-topic"));
    engine.set_automation(true, 30);
    engine.hear(&final_words("let's take this offline"), 40);
    assert_eq!(engine.raised_card("meeting"), Some("offline"));
}

#[test]
fn only_the_trigger_phrase_of_other_speech_is_shown_or_logged() {
    let mut project = default_project();
    for id in ["meeting", "classroom"] {
        project = project
            .add_pack(&pack_for_canvas(id, &Canvas::default()).unwrap())
            .unwrap();
    }
    project.decks.iter_mut().for_each(|deck| deck.voice = true);
    let mut engine = Engine::new(project, 0).unwrap();
    engine.set_voice_status(on(VoiceMode::AlwaysOn), 0);
    engine.hear(
        &final_words("between us Dana's numbers look wrong, but let's take this offline"),
        10,
    );
    assert_eq!(engine.raised_card("meeting"), Some("offline"));
    let heard = engine.voice_view().heard.unwrap();
    assert_eq!(
        (heard.text.as_str(), heard.result.as_str()),
        (
            "let's take this offline",
            "raised Meeting signals: Let's take this offline"
        )
    );
    assert_eq!(
        events(&engine, 2)[0],
        pair("voice", "heard the trigger phrase \"let's take this offline\"")
    );
    engine.hear(&final_words("between us Dana said I have a question"), 20);
    assert_eq!(engine.raised_card("classroom"), None);
    let tie = "the trigger phrase \"I have a question\" could be Meeting signals: I have a question or \
               Classroom: ❓ Question";
    assert_eq!(last_event(&engine, 20), pair("voice", tie));
    assert_eq!(engine.voice_view().heard.unwrap().text, "I have a question");
    let state = serde_json::to_string(&engine.snapshot(20)).unwrap();
    assert!(
        !state.contains("Dana") && !state.contains("between us"),
        "other words stay out of the snapshot"
    );
}

#[test]
fn a_trigger_phrase_never_raises_the_card_already_shown_again() {
    let mut engine = meeting_engine(VoiceMode::AlwaysOn);
    let mut project = engine.project().clone();
    project.decks[0].speak = true;
    engine.replace_project(project, 0).unwrap();
    engine.hear(&final_words("next topic"), 100);
    let cue = engine.speech().cloned().expect("the card is spoken");
    assert_eq!(cue.text, "Next topic");
    let outcome = engine.hear(&final_words("Next topic."), 5_000);
    assert!(!outcome.project_dirty);
    assert_eq!(
        engine.voice_view().heard.unwrap().result,
        "Meeting signals already shows Next topic"
    );
    assert_eq!(
        engine.speech().map(|cue| cue.nonce),
        Some(cue.nonce),
        "not spoken again"
    );
    assert!(
        engine.tick(15_100).changed,
        "the auto-hide was not restarted, so the card hides 15 s after it rose"
    );
    assert_eq!(engine.raised_card("meeting"), None);
}

#[test]
fn a_spoken_card_heard_back_through_the_microphone_raises_nothing() {
    let mut engine = meeting_engine(VoiceMode::AlwaysOn);
    let mut project = engine.project().clone();
    project.decks[0].speak = true;
    engine.replace_project(project, 0).unwrap();
    engine.hear(&final_words("let's take this offline"), 100);
    let nonce = engine.speech().unwrap().nonce;
    engine.speech_ready(nonce, "audio", 900);
    engine
        .card("meeting", None, sceneuron_core::CardAction::Lower, 2_000)
        .unwrap();
    let before = engine.snapshot(2_000).events.len();
    let echo = engine.hear(&final_words("Let's take this offline"), 3_000);
    assert_eq!(echo, Default::default());
    assert_eq!(engine.raised_card("meeting"), None);
    assert_eq!(engine.snapshot(3_000).events.len(), before, "an echo leaves no trace");
    engine.hear(&final_words("let's take this offline"), 25_000);
    assert_eq!(
        engine.raised_card("meeting"),
        Some("offline"),
        "said again later, it is a phrase again"
    );
}

#[test]
fn other_speech_partial_words_and_a_switched_off_voice_leave_no_trace() {
    let mut engine = meeting_engine(VoiceMode::AlwaysOn);
    let events_before = engine.snapshot(0).events.len();
    let revision = engine.revision();
    assert_eq!(
        engine.hear(&final_words("so what did everyone do this weekend"), 10),
        Default::default()
    );
    let partial = Heard {
        text: "so what did".to_owned(),
        is_final: false,
        addressed: false,
    };
    assert_eq!(engine.hear(&partial, 20), Default::default());
    assert_eq!((engine.revision(), engine.voice_view().heard), (revision, None));
    let addressed = Heard {
        text: "Sceneuron wrap".to_owned(),
        ..partial
    };
    let outcome = engine.hear(&addressed, 30);
    assert!(outcome.telemetry && !outcome.changed);
    let heard = engine.voice_view().heard.unwrap();
    assert_eq!(
        (heard.text.as_str(), heard.is_final, heard.result.as_str()),
        ("Sceneuron wrap", false, "")
    );
    assert_eq!(engine.snapshot(30).events.len(), events_before);
    let outcome = engine.hear(&final_words("scene and a wrap"), 35);
    assert!(
        outcome.telemetry && engine.voice_view().heard.is_none(),
        "the partial words are not left behind"
    );
    engine.hear(&addressed, 36);
    engine.set_voice_status(VoiceStatus::default(), 40);
    assert_eq!(last_event(&engine, 40), pair("voice", "voice control turned off"));
    assert_eq!(
        engine.hear(&final_words("Sceneuron, wrap it up"), 50),
        Default::default()
    );
    assert_eq!(engine.raised_card("meeting"), None);
}
