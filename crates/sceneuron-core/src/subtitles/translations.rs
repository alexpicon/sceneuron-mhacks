// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Translated subtitles: final segments handed out as translation jobs, answers matched to their
//! segment by id in any order, failures, and the lines closed captions read.

use std::collections::BTreeMap;

use super::engine::one_line;
use super::lines::Track;
use super::{CONTEXT_SEGMENTS, ContextLine, Segment, SubtitleState, TranslationJob};
use crate::engine::Engine;
use crate::types::Outcome;

/// Longest translation kept, in characters.
const MAX_TRANSLATION_CHARS: usize = 700;

impl Engine {
    /// Every final segment still waiting for a translation whose id is above `after_id`, with
    /// the segments before it as context, oldest first.
    pub fn translation_jobs(&self, after_id: u64) -> Vec<TranslationJob> {
        let subtitles = &self.subtitles;
        let segments = &subtitles.segments;
        let waiting = |segment: &Segment| segment.id > after_id && !segment.waiting.is_empty();
        let jobs = segments.iter().enumerate().filter(|(_, segment)| waiting(segment));
        jobs.map(|(index, segment)| {
            let earlier = segments.range(index.saturating_sub(CONTEXT_SEGMENTS)..index);
            let context = earlier.map(|line| ContextLine {
                text: line.text.clone(),
                translations: line.translations.clone(),
            });
            let targets = subtitles.status.targets.iter();
            TranslationJob {
                id: segment.id,
                text: segment.text.clone(),
                language: subtitles.status.language.clone(),
                targets: targets
                    .filter(|code| segment.waiting.contains(*code))
                    .cloned()
                    .collect(),
                context: context.collect(),
            }
        })
        .collect()
    }

    /// The translations of segment `id`, which may arrive after later ones; each shows in its
    /// target's place among the lines. Answers for a segment no longer kept, or for a target no
    /// longer wanted, are dropped.
    pub fn subtitles_translated(&mut self, id: u64, translations: &BTreeMap<String, String>, now_ms: u64) -> Outcome {
        let SubtitleState {
            segments,
            targets,
            last_failure,
            ..
        } = &mut self.subtitles;
        let Some(segment) = segments.iter_mut().find(|segment| segment.id == id) else {
            return Outcome::NONE;
        };
        let mut shown = false;
        for (code, text) in translations {
            let text = one_line(text, MAX_TRANSLATION_CHARS);
            if text.is_empty() || !segment.waiting.remove(code) {
                continue;
            }
            targets
                .entry(code.clone())
                .or_insert_with(Track::default)
                .push(id, &text, now_ms);
            segment.translations.insert(code.clone(), text);
            shown = true;
        }
        if !shown {
            return Outcome::NONE;
        }
        *last_failure = None;
        self.bump(Outcome::CHANGED)
    }

    /// Segment `id` will not be translated: its targets keep showing the lines before it. The
    /// event log says why, without the words, once for a run of the same failure.
    pub fn subtitles_translation_failed(&mut self, id: u64, why: &str, now_ms: u64) -> Outcome {
        let subtitles = &mut self.subtitles;
        if let Some(segment) = subtitles.segments.iter_mut().find(|segment| segment.id == id) {
            segment.waiting.clear();
        }
        if subtitles.last_failure.as_deref() == Some(why) {
            return Outcome::NONE;
        }
        subtitles.last_failure = Some(why.to_owned());
        self.record(now_ms, "subtitles", &format!("translation failed: {why}"));
        self.bump(Outcome::CHANGED)
    }

    /// The kept final segments with their ids, oldest first, for closed captions: in the spoken
    /// language when `language` is empty, otherwise the translations into that target so far.
    pub fn caption_lines(&self, language: &str) -> Vec<(u64, String)> {
        let segments = self.subtitles.segments.iter();
        let text = |segment: &Segment| {
            if language.is_empty() {
                Some(segment.text.clone())
            } else {
                segment.translations.get(language).cloned()
            }
        };
        segments
            .filter_map(|segment| Some((segment.id, text(segment)?)))
            .collect()
    }
}
