// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the labels that name each subtitle bar's language on the overlay: the spoken language
//! as set or as detected, targets by name or by code, several languages in one text, labels turned
//! off, the room a label takes from the words and the overlay's same measure of it, and
//! three-letter codes as speech to text reports them.

mod common;

use common::subtitles::{answer, said, status};
use sceneuron_core::{
    Element, ElementKind, ElementProps, Engine, LABEL_SCALE, LATIN_LINE, LATIN_SCALE, SubtitleBar, SubtitleStatus,
    default_project, label_room, language_name,
};

/// The default project with a text element per `(id, text)`, 2000 by 100 pixels at 40, and
/// subtitles on as `setup` says.
fn engine_with(elements: &[(&str, &str)], setup: SubtitleStatus) -> Engine {
    let mut project = default_project();
    for (id, words) in elements {
        project.elements.push(Element {
            id: (*id).to_owned(),
            name: format!("Text {id}"),
            kind: ElementKind::Text,
            props: ElementProps {
                text: (*words).to_owned(),
                width: 2_000.0,
                height: 100.0,
                font_size: 40.0,
                ..ElementProps::default()
            },
        });
    }
    let mut engine = Engine::new(project, 0).unwrap();
    engine.set_subtitle_status(setup, 0);
    engine
}

fn bar(engine: &Engine, id: &str) -> Option<SubtitleBar> {
    let snapshot = engine.snapshot(0);
    snapshot
        .elements
        .into_iter()
        .find(|element| element.id == id)
        .unwrap()
        .subtitle_bar
}

fn label(engine: &Engine, id: &str) -> String {
    bar(engine, id).expect("a subtitle bar").label
}

fn detected(code: &str, targets: &[&str]) -> SubtitleStatus {
    SubtitleStatus {
        detected: code.to_owned(),
        ..status("", targets)
    }
}

#[test]
fn the_spoken_language_is_named_as_set_or_as_detected() {
    let mut engine = engine_with(&[("own", "{{subtitles}}")], status("en", &[]));
    assert_eq!(label(&engine, "own"), "English", "a label even before anything is said");
    let cases = [
        (status("spa", &[]), "Spanish"),
        (detected("eng", &[]), "English (detected)"),
        (detected("en", &[]), "English (detected)"),
        (detected("cmn", &[]), "Chinese (detected)"),
        (detected("jpn", &[]), "Japanese (detected)"),
        (detected("ckb", &[]), "Central Kurdish (Sorani, detected)"),
        (detected("xyz", &[]), "xyz (detected)"),
        (detected("", &[]), ""),
    ];
    for (setup, expected) in cases {
        let what = format!("{setup:?}");
        engine.set_subtitle_status(setup, 10);
        assert_eq!(label(&engine, "own"), expected, "{what}");
    }
    let set = SubtitleStatus {
        detected: "fra".to_owned(),
        ..status("en", &[])
    };
    engine.set_subtitle_status(set, 20);
    assert_eq!(
        label(&engine, "own"),
        "English",
        "a language set wins over one detected"
    );
}

#[test]
fn targets_are_named_by_code_or_place_and_unknown_codes_by_the_code() {
    let elements = [
        ("first", "{{subtitles:1}}"),
        ("quenya", "{{ subtitles:QYA }}"),
        ("third", "{{subtitles:3}}"),
        ("missing", "{{subtitles:4}}"),
        ("plain", "Hello"),
    ];
    let engine = engine_with(&elements, status("en", &["ja", "qya", "art-x-huttese"]));
    assert_eq!(label(&engine, "first"), "Japanese");
    assert_eq!(label(&engine, "quenya"), "Quenya (Elvish)");
    assert_eq!(label(&engine, "third"), "art-x-huttese");
    assert_eq!(label(&engine, "missing"), "", "a target that is not set names nothing");
    assert_eq!(bar(&engine, "plain"), None, "text without subtitles has no bar");
}

#[test]
fn a_text_with_several_languages_names_each_once_in_order() {
    let elements = [
        ("both", "{{subtitles}}\n{{subtitles:ja}}"),
        ("twice", "{{subtitles:1}} {{subtitles:ja}}"),
        ("framed", "Me: {{subtitles}}"),
    ];
    let engine = engine_with(&elements, detected("eng", &["ja"]));
    assert_eq!(label(&engine, "both"), "English (detected) · Japanese");
    assert_eq!(label(&engine, "twice"), "Japanese");
    assert_eq!(label(&engine, "framed"), "English (detected)");
}

#[test]
fn labels_turned_off_name_nothing() {
    let quiet = SubtitleStatus {
        labels: false,
        ..status("en", &["ja"])
    };
    let engine = engine_with(&[("own", "{{subtitles}}"), ("japanese", "{{subtitles:ja}}")], quiet);
    assert_eq!(label(&engine, "own"), "");
    assert_eq!(label(&engine, "japanese"), "");
}

#[test]
fn a_bar_hides_while_empty_yet_keeps_its_label() {
    let mut engine = engine_with(&[("japanese", "{{subtitles:ja}}")], status("en", &["ja"]));
    let shown = || -> bool {
        let snapshot = engine.snapshot(0);
        let element = snapshot
            .elements
            .iter()
            .find(|element| element.id == "japanese")
            .unwrap();
        element.props.visible
    };
    assert!(!shown(), "nothing to show yet");
    assert_eq!(label(&engine, "japanese"), "Japanese");
    engine.hear(&said("Good morning."), 0);
    engine.subtitles_translated(1, &answer(&[("ja", "おはようございます。")]), 10);
    assert!(engine.effective_props("japanese").unwrap().visible);
}

#[test]
fn a_label_takes_its_room_from_the_words() {
    assert_eq!(label_room("", 40.0), 0.0, "no label, no room");
    let english = label_room("English", 40.0);
    assert!(
        (english - ((7.0 * 0.64 + 1.0) * 0.4 + 0.35) * 40.0).abs() < 1e-9,
        "{english}"
    );
    assert!(label_room("English (detected)", 40.0) > english);
    assert!(
        (label_room("English", 20.0) * 2.0 - english).abs() < 1e-9,
        "it grows with the type"
    );
    // At 40 px a 600 px box fits 25 characters a line; beside "English", 21.
    let sentence = "a b c d e f g h i j k l m n o p q r s t u v w x y z";
    let lines = |labels: bool| {
        let setup = SubtitleStatus {
            labels,
            ..status("en", &[])
        };
        let mut engine = engine_with(&[("own", "{{subtitles}}")], setup);
        let mut project = engine.project().clone();
        project.elements.last_mut().unwrap().props.width = 600.0;
        engine.replace_project(project, 0).unwrap();
        engine.hear(&said(sentence), 0);
        let text = engine.effective_props("own").unwrap().text;
        text.lines().map(|line| line.chars().count()).max().unwrap()
    };
    assert_eq!(lines(false), 25);
    assert_eq!(lines(true), 21);
}

#[test]
fn the_overlay_sizes_labels_and_latin_lines_as_the_core_leaves_room_for_them() {
    let script = include_str!("../../../web/render-subtitles.js");
    let constant = |name: &str| -> f64 {
        let start = script
            .find(&format!("const {name} = "))
            .expect("render-subtitles.js has it")
            + name.len()
            + 9;
        script[start..start + script[start..].find(';').unwrap()]
            .parse()
            .unwrap()
    };
    assert_eq!(constant("LABEL_SCALE"), LABEL_SCALE);
    assert_eq!(constant("LATIN_SCALE"), LATIN_SCALE);
    assert_eq!(constant("LATIN_LINE"), LATIN_LINE);
    for label in ["", "Japanese", "English (detected)", "日本語"] {
        let chars = label.chars().count() as f64;
        let width = if chars > 0.0 {
            chars * constant("LABEL_CHAR_EM") + constant("LABEL_PADDING_EM")
        } else {
            0.0
        };
        let room = if chars > 0.0 {
            width * LABEL_SCALE + constant("LABEL_GAP_EM")
        } else {
            0.0
        };
        assert!((label_room(label, 40.0) - room * 40.0).abs() < 1e-9, "{label}");
    }
}

#[test]
fn three_letter_codes_name_the_languages_listed_by_two() {
    let names = [
        ("eng", "English"),
        ("ENG", "English"),
        ("spa", "Spanish"),
        ("ger", "German"),
        ("deu", "German"),
        ("zho", "Chinese"),
        ("yue", "Cantonese"),
        ("fil", "Filipino"),
        ("tgl", "Tagalog"),
        ("nob", "Norwegian"),
        ("por-BR", "Portuguese"),
    ];
    for (code, name) in names {
        assert_eq!(language_name(code), Some(name), "{code}");
    }
    assert_eq!(language_name("xyz"), None);
    assert_eq!(language_name("en-x-eng"), Some("English"));
}
