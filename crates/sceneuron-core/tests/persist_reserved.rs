// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises loading a project saved before "room" became the shared room's placeholder: a
//! condition with that id is renamed to a free id, with its rules and its placeholders in element,
//! rule, and card texts, instead of the file being set aside as unreadable.

use std::fs;
use std::path::PathBuf;

use sceneuron_core::{Action, Canvas, Trigger, default_project, load_or_default, pack_for_canvas, save_project};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sceneuron-core-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_condition_called_room_is_renamed_with_its_rules_and_placeholders() {
    let dir = scratch("reserved-room");
    let path = dir.join("project.json");
    let pack = pack_for_canvas("poker", &Canvas::default()).unwrap();
    let mut project = default_project().add_pack(&pack).unwrap();
    let mut second = project.conditions[0].clone();
    project.conditions[0].id = "room".to_owned();
    second.id = "room-2".to_owned();
    project.conditions.push(second);
    for rule in &mut project.rules {
        if let Trigger::Condition { condition, .. } = &mut rule.when {
            *condition = "room".to_owned();
        }
        for action in &mut rule.actions {
            if let Action::SetProps { props, .. } = action {
                props.text = Some("Tidy {{room}}".to_owned());
            }
        }
    }
    project.elements[0].props.text = "In the {{ room }} and {{room-2}}, {{scene}}, {{room".to_owned();
    project.decks[0].cards[0].props.text = Some("{{room}}!".to_owned());
    save_project(&path, &project).unwrap();

    let loaded = load_or_default(&path);
    let warning = loaded.warning.expect("a warning about the rename");
    assert!(warning.contains("\"room\" to \"room-3\""), "{warning}");
    let project = loaded.project;
    let ids: Vec<&str> = project
        .conditions
        .iter()
        .map(|condition| condition.id.as_str())
        .collect();
    assert_eq!(
        ids,
        ["room-3", "pet-in-view", "room-2"],
        "the first free id, after the one already taken"
    );
    assert_eq!(
        project.elements[0].props.text,
        "In the {{room-3}} and {{room-2}}, {{scene}}, {{room"
    );
    assert_eq!(project.decks[0].cards[0].props.text.as_deref(), Some("{{room-3}}!"));
    let watched = project
        .rules
        .iter()
        .filter(|rule| matches!(&rule.when, Trigger::Condition { .. }));
    for rule in watched {
        assert!(matches!(&rule.when, Trigger::Condition { condition, .. } if condition == "room-3"));
        for action in &rule.actions {
            if let Action::SetProps { props, .. } = action {
                assert_eq!(props.text.as_deref(), Some("Tidy {{room-3}}"));
            }
        }
    }
    assert!(!dir.join("project.json.unreadable").exists(), "nothing was set aside");
}

#[test]
fn a_project_without_reserved_ids_loads_unchanged_and_without_a_warning() {
    let path = scratch("reserved-none").join("project.json");
    let mut project = default_project();
    project.elements[0].props.text = "{{room}} {{scene}}".to_owned();
    save_project(&path, &project).unwrap();
    let loaded = load_or_default(&path);
    assert_eq!(loaded.warning, None);
    assert_eq!(
        loaded.project, project,
        "{{room}} with no such condition is the room's summary"
    );
}
