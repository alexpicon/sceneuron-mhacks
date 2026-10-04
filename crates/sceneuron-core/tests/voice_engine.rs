// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises voice control on a running engine: heard words raise and lower cards as voice
//! raises, pause and resume automation, land in the event log and the heard line on one short
//! line, and the camera leaves voice raises alone.

mod common;

use common::camera::{camera_project, raised, show, sign};
use common::last_event;
use common::voice::{events, final_words, meeting_engine, on, pair};
use sceneuron_core::{Engine, HandSign, Heard, RaisedBy, VoiceMode};

#[test]
fn addressed_words_raise_a_card_as_a_voice_raise_and_say_so() {
    let mut engine = meeting_engine(VoiceMode::AlwaysOn);
    let outcome = engine.hear(&final_words("Sceneuron, wrap it up"), 100);
    assert!(outcome.changed && !outcome.project_dirty);
    assert_eq!(engine.raised_card("meeting"), Some("wrap-up"));
    let view = engine
        .snapshot(100)
        .decks
        .into_iter()
        .find(|deck| deck.id == "meeting")
        .unwrap();
    assert_eq!(view.raised_by, Some(RaisedBy::Voice));
    assert_eq!(
        events(&engine, 2),
        [
            pair("voice", "heard: \"Sceneuron, wrap it up\""),
            pair("voice", "card raised: Meeting signals: Wrap it up")
        ]
    );
    let heard = engine.voice_view().heard.unwrap();
    assert_eq!(
        (heard.text.as_str(), heard.is_final, heard.at_ms),
        ("Sceneuron, wrap it up", true, 100)
    );
    assert_eq!(heard.result, "raised Meeting signals: Wrap it up");
    assert_eq!(
        (heard.deck.as_deref(), heard.card.as_deref()),
        (Some("meeting"), Some("wrap-up"))
    );
    engine.hear(&final_words("Sceneuron, wrap it up"), 100);
    assert_eq!(
        engine.voice_view().heard.unwrap().result,
        "Meeting signals already shows Wrap it up"
    );
}

#[test]
fn refusals_and_lowering_are_logged_and_shown() {
    let mut engine = meeting_engine(VoiceMode::PushToTalk);
    let held = |text: &str| Heard {
        text: text.to_owned(),
        is_final: true,
        addressed: true,
    };
    engine.hear(&held("um"), 5);
    assert_eq!(last_event(&engine, 5), pair("voice", "no card matched \"um\""));
    engine.set_voice_host(true);
    engine.hear(&held("order a pizza"), 10);
    assert_eq!(last_event(&engine, 10), pair("voice", "asking the AI"));
    assert_eq!(engine.voice_view().heard.unwrap().result, "asking the AI");
    engine.hear(&held("lower"), 20);
    assert_eq!(last_event(&engine, 20), pair("voice", "no card was up"));
    engine.hear(&held("my vote is five"), 30);
    engine.hear(&held("agree"), 40);
    engine.hear(&held("lower everything"), 50);
    assert_eq!(
        (engine.raised_card("meeting"), engine.raised_card("poker")),
        (None, None)
    );
    assert_eq!(
        engine.voice_view().heard.unwrap().result,
        "lowered Meeting signals, Planning poker"
    );
    let lowered = events(&engine, 2);
    assert!(
        lowered
            .iter()
            .all(|(source, detail)| source == "voice" && detail.starts_with("card lowered"))
    );
}

#[test]
fn pause_and_resume_change_the_saved_automation_switch() {
    let mut engine = meeting_engine(VoiceMode::AlwaysOn);
    let paused = engine.hear(&final_words("Sceneuron, pause"), 10);
    assert!(paused.project_dirty);
    assert!(!engine.project().automation_enabled);
    assert_eq!(last_event(&engine, 10), pair("voice", "paused automation"));
    let again = engine.hear(&final_words("Sceneuron, pause"), 20);
    assert!(!again.project_dirty);
    assert_eq!(
        engine.voice_view().heard.unwrap().result,
        "automation was already paused"
    );
    assert!(engine.hear(&final_words("Sceneuron resume"), 30).project_dirty);
    assert!(engine.project().automation_enabled);
}

#[test]
fn heard_text_is_kept_on_one_line_and_short() {
    let mut engine = meeting_engine(VoiceMode::PushToTalk);
    let long = Heard {
        text: format!("agree\n{}", "very ".repeat(100)),
        is_final: true,
        addressed: true,
    };
    engine.hear(&long, 10);
    let heard = engine.voice_view().heard.unwrap();
    assert!(heard.text.starts_with("agree very very"), "{}", heard.text);
    assert_eq!(heard.text.chars().count(), 200);
}

#[test]
fn the_camera_never_replaces_a_card_raised_by_voice() {
    let mut project = camera_project();
    project.decks[0].voice = true;
    let mut engine = Engine::new(project, 0).unwrap();
    engine.set_voice_status(on(VoiceMode::PushToTalk), 0);
    let held = Heard {
        text: "maybe".to_owned(),
        is_final: true,
        addressed: true,
    };
    engine.hear(&held, 10);
    assert_eq!(raised(&engine), Some(("maybe".to_owned(), RaisedBy::Voice)));
    assert!(!show(&mut engine, &sign(HandSign::ThumbsUp), 100, 2_000));
    assert_eq!(raised(&engine), Some(("maybe".to_owned(), RaisedBy::Voice)));
    engine.hear(
        &Heard {
            text: "yes".to_owned(),
            ..held
        },
        2_100,
    );
    assert_eq!(
        raised(&engine),
        Some(("yes".to_owned(), RaisedBy::Voice)),
        "voice replaces voice"
    );
}
