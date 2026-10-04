// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises subtitles written in a script of their own on the overlay: which languages have one,
//! a target the streamer keeps in Latin letters, the Latin line under each line and the room it
//! takes, a text that mixes languages, the editor naming the same scripts, and what keeps the
//! romanization: the editor's preview, stream captions, and spoken cards.

mod common;

use common::subtitles::{answer, said, status, translated};
use common::{deck_project, text};
use sceneuron_core::{
    CardAction, Element, ElementKind, ElementProps, Engine, NativeScript, PropsPatch, SubtitleBar, SubtitleStatus,
    constructed_languages, natural_languages,
};

const QUENYA: &str = "Elen síla lúmenn' omentielvo";
const SINDARIN: &str = "Mae govannen";
const KLINGON: &str = "nuqneH";
const TOKI_PONA: &str = "toki pona";

/// The votes deck's project, which speaks its cards, with a text element per `(id, text)`, 2000 by
/// `height` pixels at 40, and subtitles on as `setup` says.
fn engine_with(elements: &[(&str, &str)], height: f64, setup: SubtitleStatus) -> Engine {
    let mut project = deck_project();
    project.decks[0].speak = true;
    for (id, words) in elements {
        project.elements.push(Element {
            id: (*id).to_owned(),
            name: format!("Text {id}"),
            kind: ElementKind::Text,
            props: ElementProps {
                text: (*words).to_owned(),
                width: 2_000.0,
                height,
                font_size: 40.0,
                ..ElementProps::default()
            },
        });
    }
    let mut engine = Engine::new(project, 0).unwrap();
    engine.set_subtitle_status(setup, 0);
    engine
}

fn bar(engine: &Engine, id: &str) -> SubtitleBar {
    let snapshot = engine.snapshot(0);
    let element = snapshot.elements.into_iter().find(|element| element.id == id).unwrap();
    element.subtitle_bar.expect("a subtitle bar")
}

/// Says one line and answers it in every target with `translations`.
fn say(engine: &mut Engine, id: u64, translations: &[(&str, &str)]) {
    engine.hear(&said(&format!("Line {id}.")), id * 100);
    engine.subtitles_translated(id, &answer(translations), id * 100 + 10);
}

#[test]
fn each_language_with_a_script_is_written_in_it() {
    assert_eq!(NativeScript::of("qya"), Some(NativeScript::QuenyaTengwar));
    assert_eq!(NativeScript::of("sjn"), Some(NativeScript::SindarinTengwar));
    assert_eq!(NativeScript::of("TLH"), Some(NativeScript::Piqad));
    assert_eq!(NativeScript::of("tok-x-test"), Some(NativeScript::SitelenPona));
    for code in ["en", "ja", "eo", "jbo", "art-x-dothraki", "art-x-navi", "ldn", ""] {
        assert_eq!(NativeScript::of(code), None, "{code}");
    }
    let names: Vec<(&str, &str)> = [
        NativeScript::QuenyaTengwar,
        NativeScript::Piqad,
        NativeScript::SitelenPona,
    ]
    .iter()
    .map(|script| (script.id(), script.name()))
    .collect();
    assert_eq!(
        names,
        [
            ("tengwar", "Tengwar"),
            ("piqad", "pIqaD"),
            ("sitelen-pona", "sitelen pona")
        ]
    );

    let targets = ["qya", "sjn", "tlh"];
    let elements = [
        ("1", "{{subtitles:1}}"),
        ("2", "{{subtitles:2}}"),
        ("3", "{{subtitles:3}}"),
        ("own", "{{subtitles}}"),
    ];
    let mut engine = engine_with(&elements, 100.0, status("en", &targets));
    say(&mut engine, 1, &[("qya", QUENYA), ("sjn", SINDARIN), ("tlh", KLINGON)]);
    assert_eq!(text(&engine, "1"), NativeScript::QuenyaTengwar.write(QUENYA));
    assert_eq!(
        text(&engine, "2"),
        NativeScript::SindarinTengwar.write(SINDARIN),
        "Sindarin in its own mode"
    );
    assert_eq!(text(&engine, "3"), NativeScript::Piqad.write(KLINGON));
    assert_eq!(text(&engine, "own"), "Line 1.");
    let scripts: Vec<String> = ["1", "2", "3", "own"]
        .iter()
        .map(|id| bar(&engine, id).script)
        .collect();
    assert_eq!(scripts, ["tengwar", "tengwar", "piqad", ""]);
}

#[test]
fn the_editor_names_the_same_scripts_for_the_same_languages() {
    let script = include_str!("../../../web/editor/languages.js");
    let start = script.find("const SCRIPTS = {").expect("languages.js has them") + "const SCRIPTS = {".len();
    let entries = &script[start..start + script[start..].find('}').unwrap()];
    let mut editor: Vec<(String, String)> = entries
        .split(',')
        .map(|entry| {
            let (code, name) = entry.split_once(':').unwrap();
            (code.trim().to_owned(), name.trim().trim_matches('"').to_owned())
        })
        .collect();
    let mut core: Vec<(String, String)> = constructed_languages()
        .iter()
        .chain(natural_languages())
        .filter_map(|(code, _)| Some(((*code).to_owned(), NativeScript::of(code)?.name().to_owned())))
        .collect();
    editor.sort();
    core.sort();
    assert_eq!(editor, core);
}

#[test]
fn a_target_kept_in_latin_letters_shows_its_romanization() {
    let latin = SubtitleStatus {
        latin_targets: vec!["TOK".to_owned()],
        ..status("en", &["tok", "tlh"])
    };
    let elements = [("toki", "{{subtitles:tok}}"), ("klingon", "{{subtitles:tlh}}")];
    let mut engine = engine_with(&elements, 100.0, latin);
    say(&mut engine, 1, &[("tok", TOKI_PONA), ("tlh", KLINGON)]);
    assert_eq!(text(&engine, "toki"), TOKI_PONA, "codes match without regard to case");
    assert_eq!(bar(&engine, "toki").script, "");
    assert_eq!(
        text(&engine, "klingon"),
        NativeScript::Piqad.write(KLINGON),
        "the other stays in pIqaD"
    );
    engine.set_subtitle_status(status("en", &["tok", "tlh"]), 300);
    assert_eq!(
        text(&engine, "toki"),
        NativeScript::SitelenPona.write(TOKI_PONA),
        "and back"
    );
}

#[test]
fn latin_lines_go_under_each_line_and_take_their_room() {
    let glossed = |lines: u8| SubtitleStatus {
        latin_lines: true,
        lines,
        ..status("en", &["qya"])
    };
    // A line and its Latin line take 1.65 lines: 100 px at 40 holds one pair, 180 px two.
    let mut engine = engine_with(
        &[("short", "{{subtitles:qya}}"), ("tall", "{{ subtitles:1 }}")],
        100.0,
        glossed(3),
    );
    let mut project = engine.project().clone();
    project.elements.last_mut().unwrap().props.height = 180.0;
    engine.replace_project(project, 0).unwrap();
    say(&mut engine, 1, &[("qya", "Aiya")]);
    say(&mut engine, 2, &[("qya", QUENYA)]);
    say(&mut engine, 3, &[("qya", "Namárië")]);
    let short = bar(&engine, "short");
    assert_eq!(short.latin, ["Namárië"]);
    assert_eq!(text(&engine, "short"), NativeScript::QuenyaTengwar.write("Namárië"));
    let tall = bar(&engine, "tall");
    assert_eq!(tall.latin, [QUENYA, "Namárië"]);
    let written: Vec<String> = tall
        .latin
        .iter()
        .map(|line| NativeScript::QuenyaTengwar.write(line))
        .collect();
    assert_eq!(
        text(&engine, "tall"),
        written.join("\n"),
        "a written line over each Latin one"
    );

    engine.set_subtitle_status(status("en", &["qya"]), 400);
    assert!(
        bar(&engine, "tall").latin.is_empty(),
        "only when the streamer asks for them"
    );
    assert_eq!(
        text(&engine, "short").lines().count(),
        2,
        "and then the box holds two lines"
    );
}

#[test]
fn a_text_that_mixes_languages_writes_each_but_adds_no_latin_lines() {
    let glossed = SubtitleStatus {
        latin_lines: true,
        ..status("en", &["tlh"])
    };
    let mut engine = engine_with(&[("mixed", "{{subtitles}} / {{subtitles:tlh}}")], 200.0, glossed);
    say(&mut engine, 1, &[("tlh", KLINGON)]);
    let written = format!("Line 1. / {}", NativeScript::Piqad.write(KLINGON));
    assert_eq!(text(&engine, "mixed"), written);
    let mixed = bar(&engine, "mixed");
    assert_eq!((mixed.script.as_str(), mixed.latin.len()), ("", 0));
    assert_eq!(mixed.label, "English · Klingon");
}

#[test]
fn the_editor_captions_and_spoken_cards_keep_the_romanization() {
    let mut engine = engine_with(&[("quenya", "{{subtitles:qya}}")], 100.0, status("en", &["qya"]));
    say(&mut engine, 1, &[("qya", QUENYA)]);
    assert_ne!(text(&engine, "quenya"), QUENYA, "the overlay writes Tengwar");
    assert_eq!(translated(&engine, "qya"), [QUENYA], "the editor's preview");
    assert_eq!(engine.caption_lines("qya"), [(1, QUENYA.to_owned())], "stream captions");
    let mut project = engine.project().clone();
    project.decks[0].cards[0].props = PropsPatch {
        text: Some("{{subtitles:qya}}".to_owned()),
        ..PropsPatch::default()
    };
    engine.replace_project(project, 200).unwrap();
    engine.card("votes", Some("yes"), CardAction::Raise, 300).unwrap();
    assert_eq!(engine.speech().unwrap().text, QUENYA, "a spoken card");
}
