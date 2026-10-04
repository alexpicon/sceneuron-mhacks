// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The engine's side of subtitles: heard words become lines, lines clear a while after they
//! change, and text placeholders and the snapshot read them. Translations are in translations.rs.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    FinalSegment, KEPT_SEGMENTS, MAX_LINE_CHARS, MAX_LINES, Segment, SubtitleState, SubtitleStatus, SubtitlesView,
    same_language,
};
use crate::engine::Engine;
use crate::types::Outcome;
use crate::voice::Heard;

/// Longest final segment kept, in characters.
const MAX_SEGMENT_CHARS: usize = 500;

impl SubtitleState {
    fn keep(&self) -> usize {
        usize::from(self.status.lines.clamp(1, MAX_LINES))
    }

    /// The target `name` names: one of the status's codes, without regard to case, or its place
    /// in the list, counting from 1.
    pub(super) fn target(&self, name: &str) -> Option<&str> {
        let targets = &self.status.targets;
        match name.parse::<usize>() {
            Ok(place) => targets.get(place.checked_sub(1)?).map(String::as_str),
            Err(_) => targets
                .iter()
                .find(|code| code.eq_ignore_ascii_case(name))
                .map(String::as_str),
        }
    }

    /// What `{{subtitles}}`, with `target` `None`, or `{{subtitles:<target>}}` shows in a box
    /// that fits `max_chars` characters on a line and `max_lines` lines: the last lines, one per
    /// text line, as many as the settings ask for and the box fits. With `native`, a target with a
    /// script of its own is written in it, line by line, as the overlay shows it.
    pub(crate) fn text(&self, target: Option<&str>, max_chars: usize, max_lines: usize, native: bool) -> String {
        let keep = self.keep().min(max_lines);
        let lines = match target {
            None => self.own.lines(max_chars, keep, true),
            Some(name) => self
                .target(name)
                .and_then(|code| {
                    let lines = self.targets.get(code)?.lines(max_chars, keep, false);
                    Some(match self.native(code).filter(|_| native) {
                        Some(script) => lines.iter().map(|line| script.write(line)).collect(),
                        None => lines,
                    })
                })
                .unwrap_or_default(),
        };
        lines.join("\n")
    }

    fn clear_all(&mut self) {
        self.own.clear();
        self.targets.clear();
        self.segments.clear();
    }
}

impl Engine {
    /// Takes the host's report on subtitles. Turning them off clears every line, and is logged.
    pub fn set_subtitle_status(&mut self, mut status: SubtitleStatus, now_ms: u64) -> Outcome {
        status.lines = status.lines.clamp(1, MAX_LINES);
        let subtitles = &mut self.subtitles;
        if subtitles.status == status {
            return Outcome::NONE;
        }
        let turned = subtitles.status.enabled != status.enabled;
        if !status.enabled {
            subtitles.clear_all();
        }
        subtitles.targets.retain(|code, _| status.targets.contains(code));
        for segment in &mut subtitles.segments {
            segment.waiting.retain(|code| status.targets.contains(code));
        }
        let detail = if status.enabled {
            "subtitles turned on"
        } else {
            "subtitles turned off"
        };
        subtitles.status = status;
        if turned {
            self.record(now_ms, "subtitles", detail);
        }
        self.bump(Outcome::CHANGED)
    }

    /// Words heard for subtitles: partial words become the live line and a final segment a line
    /// of its own. Speech addressed to Sceneuron is left out while voice control is on, unless the
    /// host says to show it, and words shown live before they turned out to be addressed are
    /// taken back. What is addressed follows voice control's mode (see
    /// [`VoiceStatus::addresses`](crate::VoiceStatus)), so in push-to-talk mode only held speech
    /// is left out.
    pub(crate) fn hear_subtitles(&mut self, heard: &Heard, now_ms: u64) -> Outcome {
        if !self.subtitles.status.enabled {
            return Outcome::NONE;
        }
        let text = one_line(&heard.text, MAX_SEGMENT_CHARS);
        let voice = &self.voice.status;
        let command = voice.enabled && voice.addresses(&text, heard.addressed);
        if text.is_empty() || (command && self.subtitles.status.hide_commands) {
            let taken_back = self.subtitles.own.clear_live();
            return self.bump(if taken_back { Outcome::CHANGED } else { Outcome::NONE });
        }
        if heard.is_final {
            self.add_segment(&text, true, now_ms);
            return self.bump(Outcome::CHANGED);
        }
        let changed = self.subtitles.own.set_live(&text, now_ms);
        self.bump(if changed { Outcome::CHANGED } else { Outcome::NONE })
    }

    /// Adds a final segment: a line in the spoken language, the same line for every target in that
    /// language, and, when `translate`, a translation job for the other targets.
    fn add_segment(&mut self, text: &str, translate: bool, now_ms: u64) {
        let subtitles = &mut self.subtitles;
        subtitles.last_id += 1;
        let id = subtitles.last_id;
        subtitles.own.clear_live();
        subtitles.own.push(id, text, now_ms);
        let mut segment = Segment {
            id,
            text: text.to_owned(),
            translations: BTreeMap::new(),
            waiting: BTreeSet::new(),
        };
        for target in &subtitles.status.targets {
            if same_language(target, &subtitles.status.language) {
                segment.translations.insert(target.clone(), text.to_owned());
                let track = subtitles.targets.entry(target.clone()).or_default();
                track.push(id, text, now_ms);
            } else if translate {
                segment.waiting.insert(target.clone());
            }
        }
        subtitles.segments.push_back(segment);
        if subtitles.segments.len() > KEPT_SEGMENTS {
            subtitles.segments.pop_front();
        }
    }

    /// Shows `text` as a final line in the spoken language, as if it were heard, even while
    /// subtitles are off, so a layout can be checked without speaking. It is never translated, and
    /// the event log notes only that it happened, never the words.
    pub fn simulate_subtitle(&mut self, text: &str, now_ms: u64) -> Outcome {
        let text = one_line(text, MAX_SEGMENT_CHARS);
        if text.is_empty() {
            return Outcome::NONE;
        }
        self.add_segment(&text, false, now_ms);
        self.record(now_ms, "test", "simulated a subtitle line");
        self.bump(Outcome::CHANGED)
    }

    /// Clears each language whose lines have not changed for the clear delay.
    pub(crate) fn expire_subtitles(&mut self, now_ms: u64) -> Outcome {
        let subtitles = &mut self.subtitles;
        let clear_after_ms = subtitles.status.clear_after_ms;
        let mut cleared = false;
        for track in std::iter::once(&mut subtitles.own).chain(subtitles.targets.values_mut()) {
            if track.expired(now_ms, clear_after_ms) {
                cleared |= track.clear();
            }
        }
        self.bump(if cleared { Outcome::CHANGED } else { Outcome::NONE })
    }

    /// How the host last said subtitles are set up.
    pub fn subtitle_status(&self) -> &SubtitleStatus {
        &self.subtitles.status
    }

    /// Subtitles as the editor and the overlay see them.
    pub fn subtitles_view(&self) -> SubtitlesView {
        let subtitles = &self.subtitles;
        let keep = subtitles.keep();
        let translations = subtitles.status.targets.iter().map(|code| {
            let track = subtitles.targets.get(code);
            let lines = track.map(|track| track.lines(MAX_LINE_CHARS, keep, false));
            (code.clone(), lines.unwrap_or_default())
        });
        let finals = subtitles.own.entries().iter().map(|(id, text)| FinalSegment {
            id: *id,
            text: text.clone(),
        });
        SubtitlesView {
            lines: subtitles.own.lines(MAX_LINE_CHARS, keep, false),
            finals: finals.collect(),
            live: subtitles.own.live().to_owned(),
            translations: translations.collect(),
            status: subtitles.status.clone(),
        }
    }
}

/// `text` on one line, with its whitespace collapsed, cut to `max_chars`.
pub(super) fn one_line(text: &str, max_chars: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    words.join(" ").chars().take(max_chars).collect()
}
