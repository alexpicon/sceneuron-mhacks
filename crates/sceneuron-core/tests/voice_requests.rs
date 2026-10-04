// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises what voice asks of the host: a look, subtitles, and words for the AI wait as requests
//! that the heard line says it waits for, and the host's reply is shown once; requests that wait
//! too long give up, a fourth drops the oldest, a reply after newer words leaves the heard line,
//! and without a host that answers they are refused at once. Typed commands work with voice off.
//! voice_replies.rs covers carrying out the AI's actions.

mod common;

use common::last_event;
use common::requests::{heard_result, held, hosted, newest, reply};
use common::voice::{events, final_words, meeting_engine, pair};
use sceneuron_core::{
    Engine, Theme, VOICE_REQUEST_TIMEOUT_MS, VoiceAction, VoiceMode, VoiceReply, VoiceStep, VoiceTask, default_project,
};

#[test]
fn a_look_waits_for_the_host_and_its_reply_is_shown_once() {
    let mut engine = hosted();
    held(&mut engine, "look now", 100);
    let (nonce, task) = newest(&engine);
    assert_eq!(
        task,
        VoiceTask::Act {
            action: VoiceAction::Look
        }
    );
    assert_eq!(heard_result(&engine), "asking the vision model to look");
    let done = Ok(VoiceReply {
        understood: String::new(),
        steps: vec![VoiceStep::Done("the vision model looks at the camera now".to_owned())],
    });
    let outcome = engine.voice_replied(nonce, done.clone(), 900);
    assert!(outcome.changed && !outcome.project_dirty);
    assert_eq!(heard_result(&engine), "the vision model looks at the camera now");
    assert_eq!(
        last_event(&engine, 900),
        pair("voice", "the vision model looks at the camera now")
    );
    assert!(engine.voice_requests().is_empty());
    assert_eq!(
        engine.voice_replied(nonce, done, 950),
        Default::default(),
        "answered once"
    );
    held(&mut engine, "subtitles in Spanish", 1_000);
    assert_eq!(heard_result(&engine), "translating subtitles into Spanish (es)");
}

#[test]
fn requests_give_up_after_a_while_and_a_fourth_drops_the_oldest() {
    let mut engine = hosted();
    held(&mut engine, "make it rain", 100);
    let (nonce, _) = newest(&engine);
    assert!(!engine.tick(100 + VOICE_REQUEST_TIMEOUT_MS).changed);
    assert!(engine.tick(101 + VOICE_REQUEST_TIMEOUT_MS).changed);
    assert_eq!(heard_result(&engine), "the AI did not answer in time");
    assert_eq!(
        engine.voice_replied(nonce, reply(Vec::new()), 200_000),
        Default::default()
    );
    for (index, words) in ["one more thing", "another thing", "a third thing", "a fourth thing"]
        .into_iter()
        .enumerate()
    {
        held(&mut engine, words, 300_000 + index as u64);
    }
    let waiting: Vec<_> = engine
        .voice_requests()
        .iter()
        .map(|request| request.task.clone())
        .collect();
    assert_eq!(waiting.len(), 3);
    assert_eq!(
        waiting[0],
        VoiceTask::Ask {
            text: "another thing".to_owned()
        }
    );
    let logged = engine
        .snapshot(0)
        .events
        .iter()
        .any(|event| event.detail == "dropped, because newer commands were waiting");
    assert!(logged);
}

#[test]
fn without_a_host_requests_are_refused_at_once() {
    let mut engine = meeting_engine(VoiceMode::PushToTalk);
    for (words, refused) in [
        (
            "order a pizza",
            "no command matched, and Sceneuron's AI is not running to take others",
        ),
        (
            "look now",
            "the vision model cannot look: Sceneuron's AI is not running",
        ),
        ("subtitles on", "subtitles cannot change: Sceneuron's AI is not running"),
        (
            "subtitles in Spanish",
            "subtitles cannot change: Sceneuron's AI is not running",
        ),
    ] {
        held(&mut engine, words, 100);
        assert_eq!(heard_result(&engine), refused, "{words}");
        assert_eq!(last_event(&engine, 100), pair("voice", refused));
    }
    assert!(engine.voice_requests().is_empty());
    held(&mut engine, "mind blown", 200);
    assert_eq!(
        heard_result(&engine),
        "played 🤯 Mind blown",
        "local commands need no host"
    );
}

#[test]
fn a_reply_after_newer_words_is_logged_but_leaves_the_heard_line() {
    let mut engine = hosted();
    held(&mut engine, "make it rain", 100);
    let (nonce, _) = newest(&engine);
    held(&mut engine, "agree", 200);
    engine.voice_replied(nonce, Err("the hourly budget is spent".to_owned()), 300);
    assert_eq!(heard_result(&engine), "raised Meeting signals: Agree");
    assert_eq!(last_event(&engine, 300), pair("voice", "the hourly budget is spent"));
}

#[test]
fn typed_commands_work_with_voice_off_and_say_they_were_typed() {
    let mut engine = Engine::new(default_project(), 0).unwrap();
    assert_eq!(
        engine.hear(&final_words("Sceneuron, hacker theme"), 10),
        Default::default()
    );
    let revision = engine.project_revision();
    let outcome = engine.type_command("hacker theme", 20);
    assert!(outcome.changed && outcome.project_dirty);
    assert_eq!(engine.project().theme, Theme::Hacker);
    assert_eq!(engine.project_revision(), revision + 1, "the editor catches up");
    let heard = engine.voice_view().heard.unwrap();
    assert!(heard.typed && heard.is_final);
    assert_eq!(heard.result, "switched the overlay to the Hacker theme");
    assert_eq!(
        events(&engine, 2),
        [
            pair("voice", "typed: \"hacker theme\""),
            pair("voice", "switched the overlay to the Hacker theme"),
        ]
    );
    let again = engine.type_command("hacker theme", 30);
    assert!(!again.project_dirty);
    assert_eq!(heard_result(&engine), "the Hacker theme was already on");
    engine.type_command("theme off", 40);
    assert_eq!(
        (engine.project().theme, heard_result(&engine).as_str()),
        (Theme::None, "turned the theme off")
    );
    assert_eq!(engine.type_command("  ", 50), Default::default());
}
