// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises reactions by voice on a running engine: a reaction's trigger phrase plays it without
//! the wake word in always-on mode, as automation with the cooldown the camera shares, unless fun
//! reactions or its voice are off; a phrase after a negation, "break" alone, or the overlay's own
//! spoken card heard back plays nothing; a card phrase as long ties; and a reaction asked for by
//! name plays at once. reaction_presets.rs covers the voice switch in the saved project.

mod common;

use common::voice::{events, final_words, meeting_engine, pair, said, with_decks};
use sceneuron_core::{CardAction, DEFAULT_FLOOD_COOLDOWN_MS, Engine, Flood, Heard, Intent, VoiceMode};

fn flood(engine: &Engine) -> Option<(Flood, u64)> {
    engine.flood().map(|flood| (flood.flood, flood.at_ms))
}

fn result(engine: &Engine) -> (String, String) {
    let heard = engine.voice_view().heard.unwrap();
    (heard.text, heard.result)
}

fn change(engine: &mut Engine, edit: impl FnOnce(&mut sceneuron_core::Project)) {
    let mut project = engine.project().clone();
    edit(&mut project);
    engine.replace_project(project, 0).unwrap();
}

#[test]
fn a_trigger_phrase_plays_its_reaction_without_the_wake_word() {
    let mut engine = meeting_engine(VoiceMode::AlwaysOn);
    let outcome = engine.hear(&final_words("Honestly, this meeting could have been an email."), 100);
    assert!(outcome.changed && !outcome.project_dirty);
    assert_eq!(flood(&engine), Some((Flood::Email, 100)));
    assert_eq!(
        events(&engine, 2),
        [
            pair("voice", "heard the trigger phrase \"could have been an email\""),
            pair(
                "voice",
                "played 📧 Could have been an email, for the trigger phrase \"could have been an email\""
            ),
        ]
    );
    assert_eq!(
        result(&engine),
        (
            "could have been an email".to_owned(),
            "played 📧 Could have been an email".to_owned()
        ),
        "only the phrase is shown"
    );
    let mut push = meeting_engine(VoiceMode::PushToTalk);
    assert_eq!(push.hear(&final_words("mind blown"), 100), Default::default());
    assert_eq!(flood(&push), None, "push to talk hears no phrases");
}

#[test]
fn trigger_phrases_share_the_cooldown_and_need_automation() {
    let mut engine = meeting_engine(VoiceMode::AlwaysOn);
    engine.hear(&final_words("that totally blew my mind"), 1_000);
    assert_eq!(flood(&engine), Some((Flood::MindBlown, 1_000)));
    engine.hear(&final_words("ok ship it"), 3_000);
    assert_eq!(flood(&engine), Some((Flood::MindBlown, 1_000)));
    assert_eq!(
        result(&engine),
        (
            "ship it".to_owned(),
            "without \"Ship it\": the last flood began 2 s ago".to_owned()
        )
    );
    engine.hear(&final_words("ok ship it"), 1_000 + DEFAULT_FLOOD_COOLDOWN_MS);
    assert_eq!(flood(&engine), Some((Flood::ShipIt, 1_000 + DEFAULT_FLOOD_COOLDOWN_MS)));
    engine.set_automation(false, 30_000);
    engine.hear(&final_words("coffee break"), 60_000);
    assert_eq!(flood(&engine), Some((Flood::ShipIt, 1_000 + DEFAULT_FLOOD_COOLDOWN_MS)));
    assert_eq!(
        result(&engine),
        (
            "coffee break".to_owned(),
            "a trigger phrase, but automation is paused".to_owned()
        )
    );
}

#[test]
fn the_camera_and_trigger_phrases_wait_for_each_other() {
    let mut engine = meeting_engine(VoiceMode::AlwaysOn);
    for at_ms in (0..=1_000).step_by(200) {
        engine.observe_facepalm(true, at_ms);
    }
    assert_eq!(flood(&engine), Some((Flood::Email, 1_000)));
    engine.hear(&final_words("mind blown"), 3_000);
    assert_eq!(flood(&engine), Some((Flood::Email, 1_000)));
}

#[test]
fn switched_off_reactions_and_voices_hear_nothing_and_leave_no_trace() {
    let mut engine = meeting_engine(VoiceMode::AlwaysOn);
    change(&mut engine, |project| project.reactions.voice_off = vec![Flood::ShipIt]);
    let before = engine.snapshot(0).events.len();
    assert_eq!(engine.hear(&final_words("let's ship it"), 100), Default::default());
    assert_eq!((flood(&engine), engine.snapshot(0).events.len()), (None, before));
    engine.hear(&final_words("mind blown"), 200);
    assert_eq!(flood(&engine), Some((Flood::MindBlown, 200)), "only its own voice");
    change(&mut engine, |project| project.reactions.enabled = false);
    engine.hear(&final_words("coffee break"), 100_000);
    assert_eq!(flood(&engine), Some((Flood::MindBlown, 200)));
}

#[test]
fn negations_a_lone_break_and_card_phrases_as_long_play_nothing() {
    let mut engine = meeting_engine(VoiceMode::AlwaysOn);
    for heard in [
        "we shouldn't ship it yet",
        "I'm not mind blown",
        "I need a break",
        "more coffee please",
    ] {
        assert_eq!(engine.hear(&final_words(heard), 100), Default::default(), "{heard}");
    }
    assert_eq!(flood(&engine), None);
    let mut decks = engine.project().decks.clone();
    decks[0].cards[0].phrases = vec!["ship it".to_owned()];
    let unclear = Intent::Unclear {
        phrase: "ship it".to_owned(),
        reason: "the trigger phrase \"ship it\" could be the Ship it reaction or Meeting signals: Wrap it up"
            .to_owned(),
    };
    assert_eq!(said("let's ship it", &decks), unclear);
    decks[0].cards[0].phrases = vec!["ship it today".to_owned()];
    assert!(matches!(said("we ship it today", &decks), Intent::Raise { .. }));
    decks[0].cards[0].phrases = vec!["it".to_owned()];
    let play = Intent::Play {
        flood: Flood::ShipIt,
        phrase: "ship it".to_owned(),
    };
    assert_eq!(said("ship it", &decks), play, "the longer phrase wins");
    assert!(with_decks(&decks).reactions.hears(Flood::ShipIt));
}

#[test]
fn the_overlay_saying_a_phrase_plays_nothing() {
    let mut engine = meeting_engine(VoiceMode::AlwaysOn);
    change(&mut engine, |project| {
        let deck = &mut project.decks[0];
        deck.speak = true;
        deck.cards[0].speech = Some("Mind blown, everyone".to_owned());
    });
    engine.card("meeting", Some("wrap-up"), CardAction::Raise, 100).unwrap();
    let nonce = engine.speech().unwrap().nonce;
    engine.speech_ready(nonce, "audio", 900);
    engine.card("meeting", None, CardAction::Lower, 1_000).unwrap();
    assert_eq!(engine.hear(&final_words("mind blown"), 3_000), Default::default());
    assert_eq!(flood(&engine), None, "the overlay said it a moment ago");
    engine.hear(&final_words("mind blown"), 30_000);
    assert_eq!(flood(&engine), Some((Flood::MindBlown, 30_000)));
}

#[test]
fn a_reaction_asked_for_by_name_plays_at_once_like_try_it() {
    let mut engine = meeting_engine(VoiceMode::AlwaysOn);
    change(&mut engine, |project| project.reactions.enabled = false);
    engine.set_automation(false, 0);
    engine.hear(&final_words("mind blown"), 100);
    assert_eq!(flood(&engine), None);
    engine.hear(&final_words("Sceneuron, mind blown"), 200);
    assert_eq!(flood(&engine), Some((Flood::MindBlown, 200)));
    let held = Heard {
        text: "play ship it".to_owned(),
        is_final: true,
        addressed: true,
    };
    engine.hear(&held, 300);
    assert_eq!(flood(&engine), Some((Flood::ShipIt, 300)), "no cooldown");
    assert_eq!(
        events(&engine, 2),
        [
            pair("voice", "heard: \"play ship it\""),
            pair("voice", "flooded the overlay with 🚀 🎉 ✅ 🙌 and \"Ship it!\""),
        ]
    );
    assert_eq!(result(&engine).1, "played 🚀 Ship it");
}
