// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the Subtitles pack: its two bars, where they sit, how large their type is, scaling to
//! other canvases, what they show and when they hide in an engine, how many lines each shows, and
//! adding it to a project. Its colors are checked with every other pack's in pack_colors.rs, and
//! its place among the other packs in pack_layout.rs.

mod common;

use common::subtitles::{answer, said, status};
use common::{text, visible};
use sceneuron_core::{Canvas, ElementKind, Engine, Pack, default_project, pack_for_canvas};

/// Title-safe margins: 5% of each side, inside which text is never cut off by a display.
const SAFE: f64 = 0.05;
const SHAPES: [(u32, u32); 5] = [(1920, 1080), (1280, 720), (2560, 1080), (1024, 768), (1080, 1920)];

fn pack(width: u32, height: u32) -> Pack {
    pack_for_canvas("subtitles", &Canvas { width, height }).unwrap()
}

#[test]
fn the_pack_is_two_text_bars_without_a_deck() {
    let pack = pack(1920, 1080);
    assert_eq!((pack.id.as_str(), pack.name.as_str()), ("subtitles", "Subtitles"));
    assert!(pack.decks.is_empty());
    let bars: Vec<_> = pack
        .elements
        .iter()
        .map(|bar| (bar.id.as_str(), bar.props.text.as_str()))
        .collect();
    assert_eq!(
        bars,
        [
            ("subtitles-spoken", "{{subtitles}}"),
            ("subtitles-translated", "{{subtitles:1}}")
        ]
    );
    for bar in &pack.elements {
        assert_eq!(bar.kind, ElementKind::Text);
        assert!(bar.props.visible, "{} hides by itself while empty", bar.id);
        assert!(bar.props.font_size >= 40.0, "large type for compressed video");
        assert!(
            bar.props.height >= 2.0 * 1.15 * bar.props.font_size,
            "two lines fit in {}",
            bar.id
        );
    }
    let (spoken, translated) = (&pack.elements[0].props, &pack.elements[1].props);
    assert!(spoken.y + spoken.height <= translated.y, "the spoken line comes first");
}

#[test]
fn the_bars_stay_inside_the_title_safe_area_of_every_canvas() {
    for (width, height) in SHAPES {
        let (w, h) = (f64::from(width), f64::from(height));
        for bar in pack(width, height).elements {
            let props = &bar.props;
            let inside = props.x >= w * SAFE
                && props.x + props.width <= w * (1.0 - SAFE)
                && props.y >= h * SAFE
                && props.y + props.height <= h * (1.0 - SAFE);
            assert!(inside, "{} on {width}x{height}: {props:?}", bar.id);
        }
    }
    let small = pack(1280, 720);
    assert_eq!(small.elements[0].props.font_size, 27.0, "type scales with the canvas");
    assert_eq!(small.elements[0].props.width, 773.0);
}

#[test]
fn the_bars_show_the_spoken_line_and_its_first_translation_and_hide_between_sentences() {
    let base = default_project();
    let project = base
        .add_pack(&pack_for_canvas("subtitles", &base.canvas).unwrap())
        .unwrap();
    let mut engine = Engine::new(project, 0).unwrap();
    engine.set_subtitle_status(status("en", &["es"]), 0);
    assert!(!visible(&engine, "subtitles-spoken") && !visible(&engine, "subtitles-translated"));
    engine.hear(&said("Thanks for joining, everyone."), 100);
    assert_eq!(text(&engine, "subtitles-spoken"), "Thanks for joining, everyone.");
    assert!(
        !visible(&engine, "subtitles-translated"),
        "the translation has not arrived"
    );
    engine.subtitles_translated(1, &answer(&[("es", "Gracias por venir a todos.")]), 900);
    assert_eq!(text(&engine, "subtitles-translated"), "Gracias por venir a todos.");
    assert!(visible(&engine, "subtitles-spoken") && visible(&engine, "subtitles-translated"));
    engine.tick(5_000);
    assert!(!visible(&engine, "subtitles-spoken") && !visible(&engine, "subtitles-translated"));
}

#[test]
fn three_lines_asked_for_are_cut_to_what_each_bar_holds_on_every_canvas() {
    for (width, height) in SHAPES {
        let mut base = default_project();
        base.canvas = Canvas { width, height };
        let project = base.add_pack(&pack(width, height)).unwrap();
        let mut engine = Engine::new(project, 0).unwrap();
        let mut setup = status("en", &["es"]);
        setup.lines = 3;
        engine.set_subtitle_status(setup, 0);
        for (id, line) in [
            (1, "First line here."),
            (2, "Second line here."),
            (3, "Third line here."),
        ] {
            engine.hear(&said(line), id * 100);
            engine.subtitles_translated(id, &answer(&[("es", line)]), id * 100 + 50);
        }
        assert_eq!(engine.subtitles_view().lines.len(), 3, "the editor shows all three");
        for bar in ["subtitles-spoken", "subtitles-translated"] {
            let props = engine.effective_props(bar).unwrap();
            let shown = props.text.lines().count();
            assert!(
                shown >= 1 && shown as f64 * 1.15 * props.font_size <= props.height,
                "{bar} on {width}x{height}: {shown} lines of {} px in {} px",
                props.font_size,
                props.height
            );
            assert!(props.text.ends_with("Third line here."), "the newest line stays");
        }
    }
}

#[test]
fn adding_the_pack_twice_or_over_a_used_id_is_refused() {
    let base = default_project();
    let pack = pack_for_canvas("subtitles", &base.canvas).unwrap();
    let once = base.add_pack(&pack).unwrap();
    assert_eq!(once.elements.len(), base.elements.len() + 2);
    assert_eq!(
        once.add_pack(&pack).unwrap_err().0,
        "Subtitles is already in this project"
    );
    let mut half = base.clone();
    half.elements.push(pack.elements[1].clone());
    let error = half.add_pack(&pack).unwrap_err();
    assert!(error.0.contains("element id \"subtitles-translated\""), "{error}");
}
