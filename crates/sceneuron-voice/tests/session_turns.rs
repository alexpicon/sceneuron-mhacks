// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises push to talk in always-on mode with made-up times: a press closes the stream and
//! opens a turn of its own, so a final from before the press, however late, and speech after
//! the release are never addressed, and the stream opens again once the turn is over. Usage
//! says whether the session carried keyterms.

mod common;

use common::{config, count, final_text, heard, pcm, sends, statuses};
use sceneuron_voice::session::{Action, CHUNK_BYTES, COMMIT_WAIT_MS, Kind, Session};
use sceneuron_voice::{Heard, Mode, ServerMessage, VoiceEvent, VoiceStatus};

fn listening() -> Session {
    let mut session = Session::new(config(Mode::AlwaysOn), 0);
    session.connected(100);
    session.take_actions();
    assert_eq!(session.kind(), Kind::Stream);
    session
}

/// The connections closed and opened among `actions`, in order.
fn links(actions: &[Action]) -> Vec<&Action> {
    actions
        .iter()
        .filter(|action| matches!(action, Action::Disconnect | Action::Connect))
        .collect()
}

#[test]
fn a_press_closes_the_stream_and_held_speech_goes_to_a_turn_of_its_own() {
    let mut session = listening();
    session.audio(&pcm(150), 150);
    assert_eq!(sends(&session.take_actions()), [(CHUNK_BYTES, false)]);
    session.talk(true, 200);
    let actions = session.take_actions();
    assert_eq!(links(&actions), [&Action::Disconnect, &Action::Connect]);
    assert_eq!(statuses(&actions), [VoiceStatus::Connecting]);
    assert_eq!(session.kind(), Kind::Turn, "the turn commits by hand");
    session.audio(&pcm(150), 220);
    session.connected(250);
    session.message(ServerMessage::Partial("wrap".to_owned()), 300);
    session.talk(false, 400);
    let actions = session.take_actions();
    assert_eq!(
        sends(&actions),
        [(CHUNK_BYTES, false), (CHUNK_BYTES / 2, true)],
        "only held audio goes to the turn"
    );
    let partial = Heard {
        text: "wrap".to_owned(),
        is_final: false,
        addressed: true,
    };
    assert_eq!(heard(&actions), [partial]);
    session.message(ServerMessage::Committed("wrap it up".to_owned()), 600);
    let actions = session.take_actions();
    assert_eq!(heard(&actions), [final_text("wrap it up", true)]);
    assert_eq!(
        links(&actions),
        [&Action::Disconnect, &Action::Connect],
        "the stream opens again"
    );
    assert_eq!(session.kind(), Kind::Stream);
    session.connected(700);
    session.message(ServerMessage::Committed("pause".to_owned()), 900);
    assert_eq!(heard(&session.take_actions()), [final_text("pause", false)]);
}

#[test]
fn a_late_final_from_before_the_press_is_never_addressed() {
    let mut session = listening();
    session.audio(&pcm(400), 150);
    session.talk(true, 200);
    // The stream's final for "lower", said before the press, comes after it.
    session.message(ServerMessage::Committed("lower".to_owned()), 250);
    session.connected(300);
    session.audio(&pcm(300), 350);
    session.talk(false, 700);
    session.message(ServerMessage::Committed("wrap it up".to_owned()), 900);
    assert_eq!(
        heard(&session.take_actions()),
        [final_text("wrap it up", true)],
        "the closed stream's final is dropped, the held command is addressed"
    );
}

#[test]
fn speech_after_the_release_is_never_addressed_while_the_turn_waits() {
    let mut session = listening();
    session.talk(true, 200);
    session.connected(300);
    session.audio(&pcm(300), 350);
    session.talk(false, 700);
    session.take_actions();
    session.audio(&pcm(500), 1_000);
    assert!(
        sends(&session.take_actions()).is_empty(),
        "speech after the release waits for the stream instead of joining the turn"
    );
    session.tick(700 + COMMIT_WAIT_MS);
    let actions = session.take_actions();
    assert_eq!(links(&actions), [&Action::Disconnect, &Action::Connect]);
    session.connected(700 + COMMIT_WAIT_MS + 100);
    let actions = session.take_actions();
    assert_eq!(sends(&actions), [(CHUNK_BYTES, false); 5]);
    session.message(ServerMessage::Committed("pause".to_owned()), 700 + COMMIT_WAIT_MS + 500);
    assert_eq!(
        heard(&session.take_actions()),
        [final_text("pause", false)],
        "no commit wait addresses what follows"
    );
}

#[test]
fn usage_says_whether_the_session_carried_keyterms() {
    for (keyterms, expected) in [(vec!["Sceneuron".to_owned()], true), (Vec::new(), false)] {
        let mut config = config(Mode::AlwaysOn);
        config.keyterms = keyterms;
        let mut session = Session::new(config, 0);
        session.connected(100);
        for second in 0..10 {
            session.audio(&pcm(1_000), 200 + second * 1_000);
        }
        let reported: Vec<(f64, bool)> = session
            .take_actions()
            .iter()
            .filter_map(|action| match action {
                Action::Emit(VoiceEvent::Usage {
                    audio_seconds,
                    keyterms,
                }) => Some((*audio_seconds, *keyterms)),
                _ => None,
            })
            .collect();
        assert_eq!(reported, [(10.0, expected)]);
        assert_eq!(count(&session.take_actions(), &Action::Connect), 0);
    }
}
