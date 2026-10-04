// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises how heard words become subtitle lines in the engine: partial words on the live line,
//! final segments as lines of their own, how many lines are kept and how they wrap, whole final
//! segments with their ids, clearing a while after the last speech, speech addressed to Sceneuron
//! left out, and the status the host reports, including a spent budget.

mod common;

use common::subtitles::{held, lines, logged, partial, said, status, subtitled};
use sceneuron_core::{Engine, SubtitleStatus, VoiceMode, VoiceStatus, default_project};

fn voice_on(engine: &mut Engine) {
    let status = VoiceStatus {
        enabled: true,
        mode: VoiceMode::AlwaysOn,
        ..VoiceStatus::default()
    };
    engine.set_voice_status(status, 0);
}

#[test]
fn partial_words_update_the_live_line_and_a_final_segment_replaces_them() {
    let mut engine = subtitled(&[]);
    assert!(engine.hear(&partial("Hello"), 100).changed);
    assert!(
        !engine.hear(&partial("Hello"), 150).changed,
        "the same words change nothing"
    );
    engine.hear(&partial("Hello there"), 200);
    let view = engine.subtitles_view();
    assert_eq!((view.live.as_str(), view.lines.len()), ("Hello there", 0));
    assert!(engine.hear(&said("Hello there, everyone."), 300).changed);
    let view = engine.subtitles_view();
    assert_eq!(view.lines, ["Hello there, everyone."]);
    assert_eq!(view.live, "", "the final segment replaces the live words");
}

#[test]
fn the_last_lines_are_kept_two_by_default_and_one_to_three_when_set() {
    let mut engine = subtitled(&[]);
    for (at, text) in ["One.", "Two.", "Three.", "Four."].iter().enumerate() {
        engine.hear(&said(text), at as u64 * 100);
    }
    assert_eq!(lines(&engine), ["Three.", "Four."]);
    for (wanted, kept) in [(1, 1), (3, 3), (0, 1), (9, 3)] {
        let mut setup = status("en", &[]);
        setup.lines = wanted;
        engine.set_subtitle_status(setup, 500);
        assert_eq!(lines(&engine).len(), kept, "{wanted} lines asked for");
    }
    assert_eq!(lines(&engine), ["Two.", "Three.", "Four."]);
}

#[test]
fn long_segments_wrap_at_word_boundaries_within_84_characters() {
    let mut engine = subtitled(&[]);
    let sentence = "Today we are going to look at the quarterly numbers, then the roadmap for the next \
                    release, and finally the questions you sent in before the meeting.";
    engine.hear(&said(sentence), 0);
    let shown = lines(&engine);
    assert_eq!(shown.len(), 2, "{shown:?}");
    assert!(shown.iter().all(|line| line.chars().count() <= 84), "{shown:?}");
    assert!(
        shown[1].ends_with("before the meeting."),
        "the newest lines stay: {shown:?}"
    );
    assert!(sentence.contains(&shown[0]), "lines break between words: {shown:?}");
}

#[test]
fn final_segments_stay_whole_with_their_ids_so_each_can_be_announced_once() {
    let mut engine = subtitled(&[]);
    let long = "Today we are planning the next sprint together and we will look at every story in the \
                backlog before lunch.";
    engine.hear(&said(long), 0);
    assert_eq!(lines(&engine).len(), 2, "the segment wraps");
    engine.hear(&said("Yes."), 100);
    engine.hear(&said("Yes."), 200);
    let view = engine.subtitles_view();
    let finals: Vec<(u64, &str)> = view
        .finals
        .iter()
        .map(|segment| (segment.id, segment.text.as_str()))
        .collect();
    assert_eq!(finals, [(1, long), (2, "Yes."), (3, "Yes.")]);
    engine.tick(10_000);
    assert!(engine.subtitles_view().finals.is_empty(), "cleared with the lines");
}

#[test]
fn lines_clear_a_while_after_the_last_speech() {
    let mut engine = subtitled(&[]);
    engine.hear(&said("First."), 1_000);
    assert!(!engine.tick(4_999).changed);
    assert_eq!(lines(&engine), ["First."]);
    assert!(engine.tick(5_000).changed);
    assert!(lines(&engine).is_empty());
    engine.hear(&said("Second."), 6_000);
    engine.hear(&partial("still talking"), 9_000);
    engine.tick(10_000);
    assert_eq!(lines(&engine), ["Second."], "speech still going on keeps the lines");
    engine.tick(13_000);
    assert!(lines(&engine).is_empty() && engine.subtitles_view().live.is_empty());
    let mut slow = status("en", &[]);
    slow.clear_after_ms = 10_000;
    engine.set_subtitle_status(slow, 14_000);
    engine.hear(&said("Third."), 14_000);
    engine.tick(23_999);
    assert_eq!(lines(&engine), ["Third."]);
    engine.tick(24_000);
    assert!(lines(&engine).is_empty());
}

#[test]
fn speech_addressed_to_sceneuron_is_left_out_while_voice_control_is_on() {
    let mut engine = subtitled(&[]);
    voice_on(&mut engine);
    engine.hear(&said("Good morning."), 0);
    engine.hear(&held("wrap it up"), 100);
    engine.hear(&said("Sceneuron, lower everything"), 200);
    engine.hear(&partial("Hey Sceneuron, next"), 300);
    let view = engine.subtitles_view();
    assert_eq!(
        (view.lines, view.live),
        (vec!["Good morning.".to_owned()], String::new())
    );
    engine.hear(&partial("Scene"), 400);
    assert_eq!(engine.subtitles_view().live, "Scene");
    assert!(engine.hear(&partial("Sceneuron, wrap"), 500).changed);
    assert_eq!(
        engine.subtitles_view().live,
        "",
        "words shown before they were a command are taken back"
    );
}

#[test]
fn addressed_speech_shows_when_voice_control_is_off_or_hiding_is_off() {
    let mut engine = subtitled(&[]);
    engine.hear(&held("Sceneuron is our new tool."), 0);
    assert_eq!(
        lines(&engine),
        ["Sceneuron is our new tool."],
        "without voice control nothing is a command"
    );
    voice_on(&mut engine);
    let mut shown = status("en", &[]);
    shown.hide_commands = false;
    engine.set_subtitle_status(shown, 100);
    engine.hear(&said("Sceneuron, wrap it up"), 200);
    assert_eq!(lines(&engine).last().map(String::as_str), Some("Sceneuron, wrap it up"));
}

#[test]
fn nothing_shows_while_subtitles_are_off_and_turning_them_off_clears_every_line() {
    let mut engine = Engine::new(default_project(), 0).unwrap();
    assert!(!engine.hear(&said("Nobody asked for subtitles."), 0).changed);
    assert!(lines(&engine).is_empty());
    engine.set_subtitle_status(status("en", &[]), 100);
    engine.hear(&said("Now they are on."), 200);
    engine.hear(&partial("and this"), 300);
    let off = SubtitleStatus {
        enabled: false,
        state: "paused".to_owned(),
        message: "the hourly budget is spent; subtitles resume when the budget allows".to_owned(),
        ..status("en", &[])
    };
    assert!(engine.set_subtitle_status(off.clone(), 400).changed);
    let view = engine.subtitles_view();
    assert!(view.lines.is_empty() && view.live.is_empty());
    assert_eq!(view.status, off, "the editor learns why");
    engine.hear(&said("Spoken while paused."), 500);
    assert!(lines(&engine).is_empty(), "nothing is shown while the budget is spent");
    assert!(
        !engine.set_subtitle_status(off, 600).changed,
        "the same status changes nothing"
    );
    assert_eq!(
        logged(&engine, "subtitles"),
        ["subtitles turned on", "subtitles turned off"]
    );
}

#[test]
fn subtitle_text_never_reaches_the_event_log() {
    let mut engine = subtitled(&["es"]);
    voice_on(&mut engine);
    engine.hear(&partial("my private remark"), 0);
    engine.hear(&said("my private remark about the budget"), 100);
    engine.subtitles_translation_failed(1, "the provider is unavailable", 200);
    engine.simulate_subtitle("a simulated private line", 300);
    let details: Vec<String> = engine
        .snapshot(0)
        .events
        .into_iter()
        .map(|event| event.detail)
        .collect();
    assert!(details.iter().all(|detail| !detail.contains("private")), "{details:?}");
    assert!(details.contains(&"simulated a subtitle line".to_owned()), "{details:?}");
}
