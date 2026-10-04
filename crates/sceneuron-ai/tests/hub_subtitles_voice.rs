// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises voice control in a hub while subtitles keep the stream to the fake Scribe server open:
//! in push-to-talk mode, speech that starts with the wake word but was not held stays in the
//! subtitles, raises nothing, and never reaches the event log, while held speech is a command left
//! out of them; and keyterms follow voice control's own mode, not the stream's.

mod common;

use common::hub_subtitles::{fake, hosted, seconds, with_packs};
use common::sink::wait_until;
use common::speech::EL_KEY;
use sceneuron_core::{RaisedBy, default_project};
use sceneuron_server::{Assistant as _, Hub};
use serde_json::json;

const PROBE: &str = "Sceneuron is honestly my favorite tool";
const COMMAND: &str = "Sceneuron, wrap it up";

fn details(hub: &Hub) -> Vec<String> {
    hub.read(|engine| {
        engine
            .snapshot(0)
            .events
            .into_iter()
            .map(|event| event.detail)
            .collect()
    })
}

#[test]
fn in_push_to_talk_mode_only_held_speech_is_a_command_while_subtitles_stream() {
    let fake = fake(&[PROBE, COMMAND, COMMAND], 16_000);
    let mut project = with_packs(&["meeting"]);
    project.decks[0].voice = true;
    let (hub, assistant, service, _mock) = hosted(&fake, project);
    let update = json!({"elevenlabs_api_key": EL_KEY,
        "voice": {"consent": true, "enabled": true, "mode": "push_to_talk"},
        "subtitles": {"consent": true, "enabled": true}});
    service.update_settings(update).unwrap();
    let input = assistant.voice().input();
    assert!(
        wait_until(|| input.wants_audio()),
        "subtitles stream without push to talk held"
    );
    let lines = || hub.read(|engine| engine.subtitles_view().lines);
    let raised = || hub.read(|engine| engine.raised_card("meeting").map(str::to_owned));
    assert!(input.feed(&seconds(0.5)) && input.feed(&seconds(0.5)));
    assert!(wait_until(|| lines() == [PROBE, COMMAND]), "{:?}", lines());
    assert_eq!(raised(), None, "the wake word alone raises nothing");
    let logged = details(&hub);
    assert!(
        logged
            .iter()
            .all(|detail| !detail.contains("favorite") && !detail.contains("wrap it up")),
        "speech not held never reaches the event log: {logged:?}"
    );
    hub.update(|engine, now| engine.set_talking(true, now));
    assert!(input.feed(&seconds(0.5)));
    hub.update(|engine, now| engine.set_talking(false, now));
    assert!(wait_until(|| raised().as_deref() == Some("wrap-up")));
    let deck = hub.read(|engine| engine.snapshot(0).decks.into_iter().find(|deck| deck.id == "meeting"));
    assert_eq!(
        deck.unwrap().raised_by,
        Some(RaisedBy::Voice),
        "held speech is a command"
    );
    assert_eq!(lines(), [PROBE, COMMAND], "the command is left out of the subtitles");
    assert!(
        fake.sessions()[0].query_values("keyterms").contains(&"Sceneuron"),
        "a deck that voice can raise brings its keyterms"
    );
    assistant.shutdown();
    hub.shutdown();
}

#[test]
fn push_to_talk_without_cards_that_voice_can_raise_sends_no_keyterms_while_subtitles_stream() {
    let fake = fake(&[], 0);
    let (hub, assistant, service, _mock) = hosted(&fake, default_project());
    let update = json!({"elevenlabs_api_key": EL_KEY,
        "voice": {"consent": true, "enabled": true, "mode": "push_to_talk"},
        "subtitles": {"consent": true, "enabled": true}});
    service.update_settings(update).unwrap();
    assert!(wait_until(|| !fake.sessions().is_empty()));
    assert!(
        fake.sessions()[0].query_values("keyterms").is_empty(),
        "push to talk needs no wake word"
    );
    assert!(assistant.voice().keyterms().is_empty());
    service
        .update_settings(json!({"voice": {"mode": "always_on"}}))
        .unwrap();
    assert_eq!(
        assistant.voice().keyterms(),
        ["Sceneuron"],
        "always-on listens for the wake word"
    );
    assistant.shutdown();
    hub.shutdown();
}
