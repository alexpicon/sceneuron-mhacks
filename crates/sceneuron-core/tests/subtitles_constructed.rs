// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises subtitles translated into constructed and fictional languages through the engine:
//! `{{subtitles:qya}}` and a private-use tag in overlay text, the Subtitles pack showing a first
//! target that is fictional, and the jobs that ask for them. The overlay writes a language with a
//! script of its own in it, while the editor's preview keeps the romanization.

mod common;

use common::subtitles::{answer, said, status, translated};
use common::{text, visible};
use sceneuron_core::{Element, ElementKind, ElementProps, Engine, NativeScript, default_project, pack_for_canvas};

const QUENYA: &str = "Aiya, ilyë quén!";
const DOTHRAKI: &str = "M'athchomaroon, yer ray nakho!";

/// The default project with one wide text element per `(id, text)`, subtitles on in English,
/// translated into `targets`.
fn engine_with(elements: &[(&str, &str)], targets: &[&str]) -> Engine {
    let mut project = default_project();
    for (id, words) in elements {
        project.elements.push(Element {
            id: (*id).to_owned(),
            name: format!("Text {id}"),
            kind: ElementKind::Text,
            props: ElementProps {
                text: (*words).to_owned(),
                width: 2_000.0,
                font_size: 40.0,
                ..ElementProps::default()
            },
        });
    }
    let mut engine = Engine::new(project, 0).unwrap();
    engine.set_subtitle_status(status("en", targets), 0);
    engine
}

#[test]
fn placeholders_show_quenya_and_a_private_use_tag_by_code_or_place() {
    let mut engine = engine_with(
        &[
            ("quenya", "{{subtitles:qya}}"),
            ("dothraki", "{{ subtitles:ART-X-Dothraki }}"),
            ("second", "{{subtitles:2}}"),
        ],
        &["qya", "art-x-dothraki"],
    );
    engine.hear(&said("Hello, everyone!"), 0);
    let jobs = engine.translation_jobs(0);
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].targets, ["qya", "art-x-dothraki"], "both are asked for");
    engine.subtitles_translated(1, &answer(&[("qya", QUENYA), ("art-x-dothraki", DOTHRAKI)]), 10);
    assert_eq!(
        text(&engine, "quenya"),
        NativeScript::QuenyaTengwar.write(QUENYA),
        "in Tengwar"
    );
    assert_eq!(
        text(&engine, "dothraki"),
        DOTHRAKI,
        "codes match without regard to case"
    );
    assert_eq!(text(&engine, "second"), DOTHRAKI);
    assert_eq!(
        translated(&engine, "qya"),
        [QUENYA],
        "the editor's preview has it in Latin letters"
    );
}

#[test]
fn klingon_is_translated_while_the_spoken_language_is_detected() {
    let mut engine = engine_with(&[("klingon", "{{subtitles:tlh}}")], &["tlh"]);
    engine.set_subtitle_status(status("", &["tlh"]), 0);
    engine.hear(&said("Today is a good day."), 0);
    assert_eq!(engine.translation_jobs(0)[0].targets, ["tlh"]);
    engine.subtitles_translated(1, &answer(&[("tlh", "DaHjaj QaQ jaj.")]), 10);
    assert_eq!(text(&engine, "klingon"), NativeScript::Piqad.write("DaHjaj QaQ jaj."));
}

#[test]
fn the_subtitles_pack_shows_a_fictional_first_target_below_the_spoken_line() {
    let base = default_project();
    let project = base
        .add_pack(&pack_for_canvas("subtitles", &base.canvas).unwrap())
        .unwrap();
    let mut engine = Engine::new(project, 0).unwrap();
    engine.set_subtitle_status(status("en", &["qya", "es"]), 0);
    engine.hear(&said("Hello, everyone!"), 100);
    engine.subtitles_translated(1, &answer(&[("qya", QUENYA), ("es", "¡Hola a todos!")]), 900);
    assert_eq!(text(&engine, "subtitles-spoken"), "Hello, everyone!");
    assert_eq!(
        text(&engine, "subtitles-translated"),
        NativeScript::QuenyaTengwar.write(QUENYA)
    );
    assert!(visible(&engine, "subtitles-spoken") && visible(&engine, "subtitles-translated"));
}
