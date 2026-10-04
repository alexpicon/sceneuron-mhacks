// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises translated subtitles in the engine: one job per final segment for the targets in
//! other languages, with earlier segments as context; answers matched by segment id in any order;
//! failures that keep the previous line and are logged once, without words; targets in the spoken
//! language; a long answer cut to 700 characters on one line; simulated lines; clearing per
//! language; and the lines closed captions read.

mod common;

use common::subtitles::{answer, lines, logged, partial, said, status, subtitled, translated};
use sceneuron_core::{ContextLine, Engine, TranslationJob, default_project};

#[test]
fn each_final_segment_is_one_job_for_the_targets_in_other_languages() {
    let mut engine = subtitled(&["es", "en-GB", "fr"]);
    engine.hear(&partial("Good"), 0);
    assert!(
        engine.translation_jobs(0).is_empty(),
        "partial words are never translated"
    );
    engine.hear(&said("Good morning."), 100);
    let job = TranslationJob {
        id: 1,
        text: "Good morning.".to_owned(),
        language: "en".to_owned(),
        targets: vec!["es".to_owned(), "fr".to_owned()],
        context: Vec::new(),
    };
    assert_eq!(engine.translation_jobs(0), [job]);
    assert!(engine.translation_jobs(1).is_empty(), "a job is handed out once, by id");
    assert_eq!(
        translated(&engine, "en-GB"),
        ["Good morning."],
        "the spoken language shows the original"
    );
    assert!(translated(&engine, "es").is_empty());
}

#[test]
fn a_detected_language_sends_every_target_to_the_translator() {
    let mut engine = Engine::new(default_project(), 0).unwrap();
    engine.set_subtitle_status(status("", &["en", "es"]), 0);
    engine.hear(&said("Hola a todos."), 0);
    assert_eq!(engine.translation_jobs(0)[0].targets, ["en", "es"]);
}

#[test]
fn jobs_carry_up_to_three_earlier_segments_with_their_translations() {
    let mut engine = subtitled(&["es"]);
    for (id, text) in [(1, "One."), (2, "Two."), (3, "Three."), (4, "Four.")] {
        engine.hear(&said(text), id * 100);
        let spanish = ["Uno.", "Dos.", "Tres.", "Cuatro."][id as usize - 1];
        engine.subtitles_translated(id, &answer(&[("es", spanish)]), id * 100 + 50);
    }
    engine.hear(&said("Five."), 500);
    let jobs = engine.translation_jobs(4);
    let context: Vec<ContextLine> = [("Two.", "Dos."), ("Three.", "Tres."), ("Four.", "Cuatro.")]
        .iter()
        .map(|(text, spanish)| ContextLine {
            text: (*text).to_owned(),
            translations: answer(&[("es", spanish)]),
        })
        .collect();
    assert_eq!(jobs.len(), 1);
    assert_eq!((jobs[0].id, &jobs[0].context), (5, &context));
}

#[test]
fn translations_arriving_out_of_order_take_their_place_by_segment_id() {
    let mut engine = subtitled(&["es"]);
    engine.hear(&said("One."), 0);
    engine.hear(&said("Two."), 100);
    assert!(engine.subtitles_translated(2, &answer(&[("es", "Dos.")]), 200).changed);
    assert_eq!(translated(&engine, "es"), ["Dos."]);
    assert!(engine.subtitles_translated(1, &answer(&[("es", "Uno.")]), 300).changed);
    assert_eq!(translated(&engine, "es"), ["Uno.", "Dos."]);
    assert!(engine.translation_jobs(0).is_empty(), "both arrived");
    assert!(
        !engine
            .subtitles_translated(1, &answer(&[("es", "Uno otra vez.")]), 400)
            .changed
    );
    assert_eq!(translated(&engine, "es"), ["Uno.", "Dos."], "an answer counts once");
}

#[test]
fn a_failed_translation_keeps_the_previous_line_and_is_logged_once_without_words() {
    let mut engine = subtitled(&["es"]);
    let unavailable = "the AI provider is unavailable";
    engine.hear(&said("One."), 0);
    engine.subtitles_translated(1, &answer(&[("es", "Uno.")]), 50);
    engine.hear(&said("Two, a secret."), 100);
    assert!(engine.subtitles_translation_failed(2, unavailable, 150).changed);
    assert_eq!(translated(&engine, "es"), ["Uno."]);
    assert_eq!(lines(&engine), ["One.", "Two, a secret."]);
    assert!(
        engine.translation_jobs(0).is_empty(),
        "a failed segment is not asked for again"
    );
    engine.hear(&said("Three."), 200);
    assert!(!engine.subtitles_translation_failed(3, unavailable, 250).changed);
    engine.hear(&said("Four."), 300);
    engine.subtitles_translated(4, &answer(&[("es", "Cuatro.")]), 350);
    engine.hear(&said("Five."), 400);
    engine.subtitles_translation_failed(5, unavailable, 450);
    let failure = format!("translation failed: {unavailable}");
    assert_eq!(
        logged(&engine, "subtitles")[1..],
        [failure.clone(), failure],
        "again after a success"
    );
}

#[test]
fn answers_for_unknown_segments_or_unwanted_targets_change_nothing() {
    let mut engine = subtitled(&["es", "fr"]);
    engine.hear(&said("One."), 0);
    assert!(
        !engine
            .subtitles_translated(99, &answer(&[("es", "Noventa y nueve.")]), 10)
            .changed
    );
    assert!(
        !engine
            .subtitles_translated(1, &answer(&[("de", "Eins."), ("es", "  ")]), 20)
            .changed
    );
    engine.set_subtitle_status(status("en", &["es"]), 30);
    assert!(
        !engine.subtitles_translated(1, &answer(&[("fr", "Un.")]), 40).changed,
        "fr is no longer a target"
    );
    engine.subtitles_translated(1, &answer(&[("es", "Uno.\nDe verdad.")]), 50);
    assert_eq!(translated(&engine, "es"), ["Uno. De verdad."], "kept on one line");
    assert!(!engine.subtitles_view().translations.contains_key("fr"));
}

#[test]
fn a_translation_is_kept_on_one_line_of_at_most_700_characters() {
    let mut engine = subtitled(&["es"]);
    engine.hear(&said("One."), 0);
    let long = format!("{}\n{}", "palabra ".repeat(60).trim_end(), "fin ".repeat(60));
    assert!(long.chars().count() > 700);
    engine.subtitles_translated(1, &answer(&[("es", &long)]), 10);
    let kept = engine.caption_lines("es")[0].1.clone();
    assert_eq!(kept.chars().count(), 700);
    let flat = long.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.starts_with(&kept), "the start of the answer, on one line");
    engine.hear(&said("Two."), 100);
    let context = &engine.translation_jobs(1)[0].context;
    assert_eq!(
        context[0].translations["es"], kept,
        "a model's long answer goes back to it as context only cut"
    );
}

#[test]
fn simulated_lines_show_even_while_subtitles_are_off_and_are_never_translated() {
    let mut engine = subtitled(&["es", "en-US"]);
    assert!(engine.simulate_subtitle("  Testing\nthe bar ", 0).changed);
    assert_eq!(lines(&engine), ["Testing the bar"]);
    assert!(engine.translation_jobs(0).is_empty());
    assert_eq!(
        translated(&engine, "en-US"),
        ["Testing the bar"],
        "a target in the spoken language"
    );
    let mut off = Engine::new(default_project(), 0).unwrap();
    off.simulate_subtitle("Shown anyway.", 0);
    assert_eq!(lines(&off), ["Shown anyway."]);
    assert!(!off.simulate_subtitle("   ", 10).changed);
}

#[test]
fn each_language_clears_a_while_after_it_last_changed() {
    let mut engine = subtitled(&["es"]);
    engine.hear(&said("One."), 1_000);
    engine.subtitles_translated(1, &answer(&[("es", "Uno.")]), 3_000);
    engine.tick(5_000);
    assert!(lines(&engine).is_empty());
    assert_eq!(
        translated(&engine, "es"),
        ["Uno."],
        "the translation gets its full time"
    );
    engine.tick(7_000);
    assert!(translated(&engine, "es").is_empty());
}

#[test]
fn caption_lines_follow_the_chosen_language() {
    let mut engine = subtitled(&["es"]);
    engine.hear(&said("One."), 0);
    engine.hear(&said("Two."), 100);
    engine.subtitles_translated(2, &answer(&[("es", "Dos.")]), 200);
    engine.tick(10_000);
    let own = [(1, "One.".to_owned()), (2, "Two.".to_owned())];
    assert_eq!(
        engine.caption_lines(""),
        own,
        "cleared lines are still there to caption"
    );
    assert_eq!(engine.caption_lines("es"), [(2, "Dos.".to_owned())]);
    assert!(engine.caption_lines("fr").is_empty());
}
