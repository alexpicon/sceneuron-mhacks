// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises subtitles in overlay text: `{{subtitles}}` with the live words, a target by code or
//! by place, unknown targets, wrapping to fit the box, as many lines as the box is tall enough
//! for, hiding while empty, the snapshot and the elements it marks as showing subtitles, and the
//! reserved condition id.

mod common;

use std::fs;

use common::subtitles::{answer, partial, said, status};
use common::{text, visible};
use sceneuron_core::{
    Element, ElementKind, ElementProps, Engine, Project, PropsPatch, default_project, load_or_default, save_project,
};

/// The default project with a text element `id` showing `text` in a box `width` pixels wide at
/// 40 pixels, with subtitles on in English, translated into Spanish and French.
fn engine_with(elements: &[(&str, &str, f64)]) -> Engine {
    let mut project = default_project();
    for (id, words, width) in elements {
        project.elements.push(Element {
            id: (*id).to_owned(),
            name: format!("Text {id}"),
            kind: ElementKind::Text,
            props: ElementProps {
                text: (*words).to_owned(),
                width: *width,
                font_size: 40.0,
                ..ElementProps::default()
            },
        });
    }
    let mut engine = Engine::new(project, 0).unwrap();
    engine.set_subtitle_status(status("en", &["es", "fr"]), 0);
    engine
}

#[test]
fn placeholders_show_the_spoken_lines_with_the_live_words_and_each_target() {
    let wide = 2_000.0;
    let mut engine = engine_with(&[
        ("own", "{{subtitles}}", wide),
        ("spanish", "{{ subtitles:es }}", wide),
        ("first", "{{subtitles:1}}", wide),
        ("second", "{{subtitles: FR}}", wide),
        ("missing", "[{{subtitles:de}}{{subtitles:3}}{{subtitles:0}}]", wide),
    ]);
    engine.hear(&said("Good morning."), 0);
    engine.subtitles_translated(1, &answer(&[("es", "Buenos días."), ("fr", "Bonjour.")]), 10);
    engine.hear(&partial("Let's begin"), 20);
    assert_eq!(text(&engine, "own"), "Good morning.\nLet's begin");
    assert_eq!(text(&engine, "spanish"), "Buenos días.");
    assert_eq!(
        text(&engine, "first"),
        "Buenos días.",
        "targets can be named by their place"
    );
    assert_eq!(
        text(&engine, "second"),
        "Bonjour.",
        "codes match without regard to case"
    );
    assert_eq!(text(&engine, "missing"), "[]", "unknown targets show nothing");
}

#[test]
fn subtitles_wrap_to_fit_their_box_and_keep_the_last_lines() {
    let mut engine = engine_with(&[("narrow", "{{subtitles}}", 300.0), ("wide", "{{subtitles}}", 4_000.0)]);
    // Without a label beside them, the words have the whole box.
    let unlabeled = sceneuron_core::SubtitleStatus {
        labels: false,
        ..status("en", &["es", "fr"])
    };
    engine.set_subtitle_status(unlabeled, 0);
    engine.hear(&said("one two three four five six seven eight nine ten"), 0);
    let narrow = text(&engine, "narrow");
    assert_eq!(
        narrow, "seven eight\nnine ten",
        "about twelve characters fit at 40 px in 300 px"
    );
    let wide = text(&engine, "wide");
    assert_eq!(
        wide, "one two three four five six seven eight nine ten",
        "at most 84 characters a line"
    );
}

#[test]
fn subtitles_keep_only_the_lines_their_box_is_tall_enough_for() {
    let wide = 4_000.0;
    let mut engine = engine_with(&[
        ("tiny", "{{subtitles}}", wide),
        ("short", "{{subtitles}}", wide),
        ("tall", "{{subtitles}}", wide),
        ("computed", "{{subtitles}}", wide),
    ]);
    let mut project = engine.project().clone();
    let size = |project: &mut Project, id: &str, height: f64, font_size: f64| {
        let element = project.elements.iter_mut().find(|element| element.id == id).unwrap();
        (element.props.height, element.props.font_size) = (height, font_size);
    };
    // A line is 1.15 times the font size: at 40 px, 90 px holds one line, a little short of two,
    // and 138 px holds three. A box too short for one still shows one.
    size(&mut project, "tiny", 30.0, 40.0);
    size(&mut project, "short", 90.0, 40.0);
    size(&mut project, "tall", 138.0, 40.0);
    // Three lines of 21 px multiplied out come to a hair under 72.45 px, and still hold three.
    size(&mut project, "computed", 21.0 * 3.0 * 1.15, 21.0);
    engine.replace_project(project, 0).unwrap();
    let mut three = status("en", &["es", "fr"]);
    three.lines = 3;
    engine.set_subtitle_status(three, 0);
    for (at, line) in ["One.", "Two.", "Three."].iter().enumerate() {
        engine.hear(&said(line), at as u64 * 100);
    }
    assert_eq!(text(&engine, "tiny"), "Three.");
    assert_eq!(text(&engine, "short"), "Three.", "the newest line stays");
    assert_eq!(text(&engine, "tall"), "One.\nTwo.\nThree.");
    assert_eq!(text(&engine, "computed"), "One.\nTwo.\nThree.");
}

#[test]
fn text_that_shows_subtitles_hides_while_it_renders_empty() {
    let mut engine = engine_with(&[
        ("bar", "{{subtitles}}", 1_160.0),
        ("labelled", "Live: {{subtitles}}", 1_160.0),
    ]);
    assert!(!visible(&engine, "bar"), "nothing said yet");
    assert!(
        visible(&engine, "labelled"),
        "a label around them is still something to show"
    );
    engine.hear(&partial("Hello"), 0);
    assert!(visible(&engine, "bar"));
    engine.hear(&said("Hello."), 100);
    engine.tick(4_100);
    assert!(!visible(&engine, "bar"), "hidden again once the lines clear");
    assert!(visible(&engine, "name-label"), "other text is untouched");
}

#[test]
fn the_snapshot_carries_the_lines_the_live_words_and_the_status() {
    let mut engine = engine_with(&[]);
    engine.hear(&said("Good morning."), 0);
    engine.subtitles_translated(1, &answer(&[("es", "Buenos días.")]), 10);
    engine.hear(&partial("and welcome"), 20);
    let json = serde_json::to_value(engine.snapshot(30)).unwrap();
    let subtitles = &json["subtitles"];
    assert_eq!(subtitles["lines"], serde_json::json!(["Good morning."]));
    assert_eq!(subtitles["live"], "and welcome");
    assert_eq!(
        subtitles["translations"],
        serde_json::json!({"es": ["Buenos días."], "fr": []})
    );
    assert_eq!(subtitles["status"]["state"], "listening");
    assert_eq!(subtitles["status"]["targets"], serde_json::json!(["es", "fr"]));
    assert_eq!(
        (
            subtitles["status"]["lines"].as_u64(),
            subtitles["status"]["clear_after_ms"].as_u64()
        ),
        (Some(2), Some(4_000))
    );
}

#[test]
fn the_snapshot_marks_text_that_shows_subtitles_until_a_hold_replaces_it() {
    let mut engine = engine_with(&[
        ("bar", "{{subtitles}}", 1_160.0),
        ("labelled", "Live: {{ subtitles:es }}", 1_160.0),
    ]);
    engine.hear(&partial("Hello"), 0);
    let marked = |engine: &Engine| {
        let json = serde_json::to_value(engine.snapshot(10)).unwrap();
        let elements = json["elements"].as_array().unwrap().clone();
        elements
            .iter()
            .filter(|element| element["subtitles"] == true)
            .map(|element| element["id"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(marked(&engine), ["bar", "labelled"]);
    let words = PropsPatch {
        text: Some("Held words".to_owned()),
        ..PropsPatch::default()
    };
    engine.set_override("bar", &words, 20).unwrap();
    let shown = PropsPatch {
        visible: Some(true),
        ..PropsPatch::default()
    };
    engine.set_override("labelled", &shown, 20).unwrap();
    assert_eq!(
        marked(&engine),
        ["labelled"],
        "held words are not live, a held visibility keeps them"
    );
    engine.release_overrides(None, 30).unwrap();
    assert_eq!(marked(&engine), ["bar", "labelled"]);
}

#[test]
fn no_condition_may_be_called_subtitles_and_a_saved_one_is_renamed() {
    let mut project = default_project();
    project.conditions[0].id = "subtitles".to_owned();
    let error = project.validate().unwrap_err();
    assert!(error.0.contains("reserved for live subtitles"), "{error}");
    let dir = std::env::temp_dir().join(format!("sceneuron-core-reserved-subtitles-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("project.json");
    for rule in &mut project.rules {
        if let sceneuron_core::Trigger::Condition { condition, .. } = &mut rule.when {
            *condition = "subtitles".to_owned();
        }
    }
    project.elements[0].props.text = "Seen: {{subtitles}}".to_owned();
    save_project(&path, &project).unwrap();
    let loaded = load_or_default(&path);
    let warning = loaded.warning.expect("a warning about the rename");
    assert!(warning.contains("\"subtitles\" to \"subtitles-2\""), "{warning}");
    let Project {
        conditions, elements, ..
    } = loaded.project;
    assert_eq!(conditions[0].id, "subtitles-2");
    assert_eq!(elements[0].props.text, "Seen: {{subtitles-2}}");
    let _ = fs::remove_dir_all(&dir);
}
