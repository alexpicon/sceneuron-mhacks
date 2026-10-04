// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises live subtitles in a hub against the in-process fake Scribe server and a scripted
//! translating provider: subtitles stream all the time, with voice control off and no keyterms;
//! their lines and translations reach the engine and the Subtitles pack; the seconds and tokens
//! count toward the spend; and a spent hourly budget stops subtitles until it allows more. Voice
//! control on the same stream is in hub_subtitles_voice.rs.

mod common;

use common::TEST_KEY;
use common::hub_subtitles::{fake, hosted, seconds, text, with_packs};
use common::sink::wait_until;
use common::speech::EL_KEY;
use sceneuron_core::default_project;
use sceneuron_server::Assistant as _;
use serde_json::json;

#[test]
fn subtitles_stream_all_the_time_and_their_lines_and_translations_reach_the_engine() {
    let fake = fake(&["Good morning everyone"], 0);
    let (hub, assistant, service, mock) = hosted(&fake, with_packs(&["subtitles"]));
    let update = json!({"elevenlabs_api_key": EL_KEY, "api_key": TEST_KEY,
        "subtitles": {"consent": true, "enabled": true, "targets": ["es"]}});
    service.update_settings(update).unwrap();
    let input = assistant.voice().input();
    let status = || hub.read(|engine| engine.subtitles_view().status);
    assert!(
        wait_until(|| input.wants_audio() && status().state == "listening"),
        "{:?}",
        status()
    );
    assert!(
        !hub.read(|engine| engine.voice_view().enabled),
        "voice control stays off"
    );
    assert!((0..10).all(|_| input.feed(&seconds(1.0))));
    assert!(wait_until(
        || text(&hub, "subtitles-translated") == "ES Good morning everyone"
    ));
    assert_eq!(text(&hub, "subtitles-spoken"), "Good morning everyone");
    let view = hub.read(|engine| engine.subtitles_view());
    assert_eq!(
        (view.lines, view.translations["es"].clone()),
        (
            vec!["Good morning everyone".to_owned()],
            vec!["ES Good morning everyone".to_owned()]
        )
    );
    assert_eq!(mock.calls(), 1, "each line is translated once");
    let ai = || hub.read(|engine| engine.snapshot(0).ai);
    assert!(
        wait_until(|| ai().voice_seconds > 0.0 && ai().translation_calls == 1),
        "{:?}",
        ai()
    );
    let expected = 10.0 * 0.39 / 3_600.0;
    assert!(
        (ai().voice_cost_usd - expected).abs() < 1e-12,
        "no keyterm surcharge: {:?}",
        ai()
    );
    let session = &fake.sessions()[0];
    assert_eq!(session.query_values("commit_strategy"), ["vad"]);
    assert!(
        session.query_values("keyterms").is_empty(),
        "subtitles alone send no keyterms"
    );
    let events: Vec<String> = hub.read(|engine| {
        engine
            .snapshot(0)
            .events
            .into_iter()
            .map(|event| event.detail)
            .collect()
    });
    assert!(events.iter().all(|detail| !detail.contains("morning")), "{events:?}");
    assistant.shutdown();
    hub.shutdown();
}

#[test]
fn a_spent_hourly_budget_stops_subtitles_until_the_budget_allows() {
    let fake = fake(&[], 0);
    let (hub, assistant, service, _mock) = hosted(&fake, default_project());
    let budget = |max_cost_per_hour| {
        let mut project = hub.read(|engine| engine.project().clone());
        project.ai.max_cost_per_hour = max_cost_per_hour;
        hub.try_update(|engine, now| engine.replace_project(project, now))
            .unwrap();
    };
    budget(0.0);
    let update = json!({"elevenlabs_api_key": EL_KEY, "subtitles": {"consent": true, "enabled": true}});
    service.update_settings(update).unwrap();
    let input = assistant.voice().input();
    let status = || hub.read(|engine| engine.subtitles_view().status);
    // Audio fed before the stream is open waits in a buffer of a few seconds, so wait for it.
    let listening = || input.wants_audio() && status().state == "listening";
    assert!(wait_until(listening), "{:?}", status());
    assert!((0..10).all(|_| input.feed(&seconds(1.0))));
    assert!(wait_until(|| status().state == "paused"), "{:?}", status());
    assert_eq!(
        status().message,
        "the hourly budget is spent; subtitles resume when the budget allows"
    );
    assert!(
        !status().enabled && !input.wants_audio(),
        "no audio is taken while the budget is spent"
    );
    budget(2.0);
    assert!(
        wait_until(|| status().state == "listening" && input.wants_audio()),
        "{:?}",
        status()
    );
    assistant.shutdown();
    hub.shutdown();
}

#[test]
fn the_subtitle_status_follows_the_settings_and_the_key() {
    let fake = fake(&[], 0);
    let (hub, assistant, service, _mock) = hosted(&fake, default_project());
    let state = || hub.read(|engine| engine.subtitles_view().status.state);
    assert!(wait_until(|| state() == "off"));
    service
        .update_settings(json!({"subtitles": {"consent": true, "enabled": true}}))
        .unwrap();
    assert!(wait_until(|| state() == "needs_key"), "{}", state());
    assert!(fake.sessions().is_empty(), "nothing is sent without a key");
    service.update_settings(json!({"elevenlabs_api_key": EL_KEY})).unwrap();
    assert!(wait_until(|| state() == "listening"), "{}", state());
    service
        .update_settings(json!({"subtitles": {"enabled": false}}))
        .unwrap();
    assert!(wait_until(|| state() == "off"), "{}", state());
    assert!(
        !assistant.voice().input().wants_audio(),
        "turning subtitles off stops the stream"
    );
    assistant.shutdown();
    hub.shutdown();
}
