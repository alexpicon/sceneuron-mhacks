// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises what the server serves of subtitle bars: the subtitle status with the language
//! detected and how the overlay shows subtitles, each subtitle element's bar in the snapshot with
//! its label, script, and Latin lines, its words written in that script for the overlay but kept
//! in the romanization for the editor's view, and a control view that carries none of it.

mod common;

use common::{TestServer, element};
use sceneuron_core::{Heard, NativeScript, SubtitleStatus, default_project, pack_for_canvas};
use sceneuron_server::HubConfig;
use serde_json::{Value, json};

const QUENYA: &str = "Elen síla lúmenn' omentielvo";

/// A server whose project has the Subtitles pack and a Quenya bar, with subtitles on, the spoken
/// language detected as English, translated into Japanese and Quenya, with Latin lines.
fn subtitled() -> TestServer {
    let base = default_project();
    let mut project = base
        .add_pack(&pack_for_canvas("subtitles", &base.canvas).unwrap())
        .unwrap();
    let mut quenya = project.element("subtitles-translated").unwrap().clone();
    quenya.id = "subtitles-quenya".to_owned();
    quenya.props.y = 838.0;
    quenya.props.text = "{{subtitles:qya}}".to_owned();
    project.elements.push(quenya);
    let server = TestServer::with(project, HubConfig::default());
    server.hub.update(|engine, now| {
        let status = SubtitleStatus {
            enabled: true,
            state: "listening".to_owned(),
            detected: "eng".to_owned(),
            targets: vec!["ja".to_owned(), "qya".to_owned()],
            latin_lines: true,
            ..SubtitleStatus::default()
        };
        engine.set_subtitle_status(status, now);
        let heard = Heard {
            text: "Good morning everyone".to_owned(),
            is_final: true,
            addressed: false,
        };
        engine.hear(&heard, now);
        let translations = [("ja", "皆さん、おはようございます"), ("qya", QUENYA)];
        let translations = translations
            .iter()
            .map(|(code, text)| ((*code).to_owned(), (*text).to_owned()));
        engine.subtitles_translated(1, &translations.collect(), now)
    });
    server
}

#[test]
fn the_subtitle_status_carries_the_detected_language_and_how_the_overlay_shows_subtitles() {
    let server = subtitled();
    let status = server.get_json("/api/subtitles")["status"].clone();
    assert_eq!(status["detected"], "eng");
    assert_eq!(status["language"], "");
    assert_eq!(
        (
            status["labels"].clone(),
            status["latin_targets"].clone(),
            status["latin_lines"].clone()
        ),
        (json!(true), json!([]), json!(true))
    );
    let view = server.get_json("/api/subtitles");
    assert_eq!(
        view["translations"]["qya"],
        json!([QUENYA]),
        "the editor's view keeps the romanization"
    );
}

#[test]
fn each_subtitle_element_carries_its_bar_and_others_carry_none() {
    let server = subtitled();
    let state = server.get_json("/api/state");
    let bar = |id: &str| element(&state, id)["subtitle_bar"].clone();
    assert_eq!(
        bar("subtitles-spoken"),
        json!({"label": "English (detected)", "script": "", "latin": []})
    );
    assert_eq!(
        bar("subtitles-translated"),
        json!({"label": "Japanese", "script": "", "latin": []})
    );
    assert_eq!(
        bar("subtitles-quenya"),
        json!({"label": "Quenya (Elvish)", "script": "tengwar", "latin": [QUENYA]})
    );
    assert_eq!(bar("name-label"), Value::Null);
    let quenya = element(&state, "subtitles-quenya");
    assert_eq!(quenya["props"]["text"], NativeScript::QuenyaTengwar.write(QUENYA));
    assert_eq!(quenya["subtitles"], true);
}

#[test]
fn the_control_view_carries_no_label_script_or_latin_line() {
    let server = subtitled();
    let reply = server.control("GET", "/api/control/state").send(server.port);
    assert_eq!(reply.status, 200, "{}", reply.text());
    let text = reply.text();
    for private in ["subtitle_bar", "English (detected)", "Quenya", QUENYA] {
        assert!(!text.contains(private), "{private}");
    }
}
