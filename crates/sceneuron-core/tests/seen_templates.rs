// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the `{{seen}}` and `{{scene:age}}` placeholders: the live description bar's two
//! lines, the caption's age as it grows, the spellings they accept, lines of placeholders alone
//! that show nothing, which are left out, and "seen" as a reserved condition id.

mod common;

use std::fs;

use common::{engine, text, text_element, visible};
use sceneuron_core::{
    Engine, Project, SceneObservation, Sighting, default_project, load_or_default, pack_for_canvas, save_project,
    shows_caption,
};

/// The default project with the live description pack added.
fn bar() -> Engine {
    let base = default_project();
    let pack = pack_for_canvas("live-description", &base.canvas).unwrap();
    engine(base.add_pack(&pack).unwrap())
}

fn caption(text: &str) -> SceneObservation {
    SceneObservation {
        caption: text.to_owned(),
        verdicts: Vec::new(),
    }
}

fn person(engine: &mut Engine, from_ms: u64) {
    let sighting = Sighting {
        people: 1,
        ..Sighting::default()
    };
    for at in [from_ms, from_ms + 200] {
        engine.observe_sighting(&sighting, at);
    }
}

#[test]
fn the_bar_shows_both_lines_leaves_out_an_empty_one_and_hides_while_both_are() {
    let mut engine = bar();
    assert_eq!(text(&engine, "live-description"), "");
    assert!(
        !visible(&engine, "live-description"),
        "nothing to say while no camera is analyzed"
    );
    person(&mut engine, 0);
    assert_eq!(text(&engine, "live-description"), "1 person");
    assert!(visible(&engine, "live-description"));
    engine.tick(1_000);
    assert!(engine.observe_scene(caption("A person waving."), 1_000).changed);
    assert_eq!(
        text(&engine, "live-description"),
        "1 person\nA person waving. (just now)"
    );
    let stale = engine.project().detection.effective_stale_ms();
    engine.tick(200 + stale + 1);
    assert_eq!(text(&engine, "live-description"), "A person waving. (just now)");
    assert!(visible(&engine, "live-description"));
}

#[test]
fn the_caption_age_grows_in_steps_and_publishes_at_once_only_while_shown() {
    let mut engine = bar();
    let mut plain = common::engine(default_project());
    for engine in [&mut engine, &mut plain] {
        engine.tick(1_000);
        engine.observe_scene(caption("A cat on a keyboard"), 1_000);
    }
    let steps = [
        (10_999, false, "just now"),
        (11_000, true, "10 s ago"),
        (20_999, false, "10 s ago"),
        (21_000, true, "20 s ago"),
        (61_000, true, "1 min ago"),
        (1_000 + 3_600_000, true, "over an hour ago"),
    ];
    for (now, changes, age) in steps {
        assert_eq!(engine.tick(now).changed, changes, "at {now} ms");
        assert_eq!(engine.snapshot(now).scene.age, age);
        assert_eq!(
            text(&engine, "live-description"),
            format!("A cat on a keyboard ({age})")
        );
        let quiet = plain.tick(now);
        assert!(
            !quiet.changed && quiet.telemetry == changes,
            "without the bar at {now} ms: {quiet:?}"
        );
    }
}

#[test]
fn a_newer_look_with_the_same_caption_starts_its_age_again() {
    let mut engine = bar();
    engine.tick(1_000);
    engine.observe_scene(caption("A desk"), 1_000);
    engine.tick(31_000);
    assert_eq!(text(&engine, "live-description"), "A desk (30 s ago)");
    assert!(engine.observe_scene(caption("A desk"), 30_500).changed);
    assert_eq!(text(&engine, "live-description"), "A desk (just now)");
}

#[test]
fn scene_understanding_stopping_clears_the_caption_and_its_age() {
    let mut engine = bar();
    engine.tick(1_000);
    engine.observe_scene(caption("A desk"), 1_000);
    assert!(engine.reset_conditions("scene understanding failed", 2_000).changed);
    assert_eq!(engine.snapshot(2_000).scene.age, "");
    assert_eq!(text(&engine, "live-description"), "");
}

#[test]
fn the_placeholders_accept_spaces_and_an_unknown_variant_shows_nothing() {
    let mut project: Project = default_project();
    project.elements.push(text_element("spelled", "Spelled"));
    project.elements.last_mut().unwrap().props.text =
        "{{ seen }}|{{scene: age}}|{{ scene:age }}|{{scene:agee}}|{{scene :age}}|{{scene}}".to_owned();
    let mut engine = engine(project);
    person(&mut engine, 0);
    engine.observe_scene(caption("A desk"), 100);
    assert_eq!(
        text(&engine, "spelled"),
        "1 person|A desk (just now)|A desk (just now)|||A desk"
    );
}

#[test]
fn only_lines_of_placeholders_alone_that_all_show_nothing_are_left_out() {
    let mut project: Project = default_project();
    project.elements.push(text_element("lines", "Lines"));
    project.elements.last_mut().unwrap().props.text =
        "Top\n{{seen}}\n  {{scene}}  \nCamera: {{seen}}\n\n{{room}}{{seen}}{{typo}}\n{{ unclosed\nEnd".to_owned();
    let engine = engine(project);
    assert_eq!(text(&engine, "lines"), "Top\nCamera: \n\n{{ unclosed\nEnd");
}

#[test]
fn looks_ask_for_a_caption_for_either_scene_placeholder() {
    for text in ["{{scene}}", "Now: {{ scene:age }}", "{{seen}}\n{{scene: age}}"] {
        assert!(shows_caption(text), "{text}");
    }
    for text in ["{{seen}}", "{{scenery}}", "{{scene", "{{scene:agee}}", "scene"] {
        assert!(!shows_caption(text), "{text}");
    }
}

#[test]
fn seen_is_reserved_for_the_live_description() {
    let mut project = default_project();
    let mut seen = project.conditions[0].clone();
    seen.id = "seen".to_owned();
    project.conditions.push(seen);
    let error = project.validate().unwrap_err().to_string();
    assert!(error.contains("reserved for the live description"), "{error}");

    let dir = std::env::temp_dir().join(format!("sceneuron-core-reserved-seen-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("project.json");
    project.elements[0].props.text = "{{seen}} and {{scene}}".to_owned();
    save_project(&path, &project).unwrap();
    let loaded = load_or_default(&path);
    let warning = loaded.warning.expect("a warning about the rename");
    assert!(warning.contains("\"seen\" to \"seen-2\""), "{warning}");
    assert!(warning.contains("the live description"), "{warning}");
    assert_eq!(loaded.project.elements[0].props.text, "{{seen-2}} and {{scene}}");
    let _ = fs::remove_dir_all(&dir);
}
