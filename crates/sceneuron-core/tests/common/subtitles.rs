// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Subtitle fixtures: engines with subtitles on, words heard, and readers for what each language
//! and the event log show.

use std::collections::BTreeMap;

use sceneuron_core::{Engine, Heard, SubtitleStatus, default_project};

/// Subtitles on, spoken in `language`, translated into `targets`.
pub fn status(language: &str, targets: &[&str]) -> SubtitleStatus {
    SubtitleStatus {
        enabled: true,
        state: "listening".to_owned(),
        message: "listening".to_owned(),
        language: language.to_owned(),
        targets: targets.iter().map(|code| (*code).to_owned()).collect(),
        ..SubtitleStatus::default()
    }
}

/// The default project with subtitles on in English, translated into `targets`.
pub fn subtitled(targets: &[&str]) -> Engine {
    let mut engine = Engine::new(default_project(), 0).unwrap();
    engine.set_subtitle_status(status("en", targets), 0);
    engine
}

/// Words still being said, not addressed to Sceneuron.
pub fn partial(text: &str) -> Heard {
    Heard {
        text: text.to_owned(),
        is_final: false,
        addressed: false,
    }
}

/// A final segment, not addressed to Sceneuron.
pub fn said(text: &str) -> Heard {
    Heard {
        text: text.to_owned(),
        is_final: true,
        addressed: false,
    }
}

/// A final segment said while push to talk was held.
pub fn held(text: &str) -> Heard {
    Heard {
        addressed: true,
        ..said(text)
    }
}

/// The final lines in the spoken language.
pub fn lines(engine: &Engine) -> Vec<String> {
    engine.subtitles_view().lines
}

/// The final lines in `target`.
pub fn translated(engine: &Engine, target: &str) -> Vec<String> {
    engine
        .subtitles_view()
        .translations
        .get(target)
        .cloned()
        .unwrap_or_default()
}

/// One translation per `(code, text)`.
pub fn answer(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(code, text)| ((*code).to_owned(), (*text).to_owned()))
        .collect()
}

/// Every event detail logged under `source`, oldest first.
pub fn logged(engine: &Engine, source: &str) -> Vec<String> {
    let events = engine.snapshot(0).events.into_iter().rev();
    events
        .filter(|event| event.source == source)
        .map(|event| event.detail)
        .collect()
}
