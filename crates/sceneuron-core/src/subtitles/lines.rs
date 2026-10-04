// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Subtitle lines: wrapping on word boundaries, and one language's recent lines.

/// The longest line subtitles show, in characters.
pub const MAX_LINE_CHARS: usize = 84;
/// Lines shorter than this would hold too few words to read.
const MIN_LINE_CHARS: usize = 8;
/// Average width of a character of bold sans-serif text, as a fraction of the font size; a little
/// wide, so lines wrapped at the estimate rarely wrap again in the overlay.
const CHAR_WIDTH_EM: f64 = 0.55;
/// The overlay pads a text box by 0.4 em on each side.
const PADDING_EM: f64 = 0.8;
/// The overlay's line height, as a multiple of the font size (`line-height: 1.15`).
const LINE_HEIGHT_EM: f64 = 1.15;
/// Allowance for rounding, so a box exactly some lines tall fits all of them.
const ROUNDING: f64 = 1e-9;
/// Final segments a language keeps; more than its lines can show.
const KEPT_ENTRIES: usize = 6;

/// `text` in lines of at most `max_chars` characters, broken between words; a word longer than a
/// line is broken inside. Blank text has no lines.
pub fn wrap(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut count = 0;
    for word in text.split_whitespace() {
        let mut chars: Vec<char> = word.chars().collect();
        while chars.len() > max_chars {
            if count > 0 {
                lines.push(std::mem::take(&mut line));
                count = 0;
            }
            let rest = chars.split_off(max_chars);
            lines.push(chars.into_iter().collect());
            chars = rest;
        }
        if count > 0 && count + 1 + chars.len() > max_chars {
            lines.push(std::mem::take(&mut line));
            count = 0;
        }
        if count > 0 {
            line.push(' ');
            count += 1;
        }
        line.extend(chars.iter());
        count += chars.len();
    }
    if count > 0 {
        lines.push(line);
    }
    lines
}

/// How many characters fit on a line of a text box `width` pixels wide at `font_size`, at most
/// [`MAX_LINE_CHARS`].
pub(crate) fn chars_for_box(width: f64, font_size: f64) -> usize {
    let chars = ((width - font_size * PADDING_EM) / (font_size * CHAR_WIDTH_EM)).floor();
    if chars.is_finite() {
        // A negative estimate saturates to 0 and becomes the minimum.
        (chars as usize).clamp(MIN_LINE_CHARS, MAX_LINE_CHARS)
    } else {
        MAX_LINE_CHARS
    }
}

/// How many lines of text fit in a text box `height` pixels tall at `font_size`, at least one. The
/// overlay centers text vertically and hides what overflows, so more lines would be cut at both
/// edges.
pub(crate) fn lines_for_box(height: f64, font_size: f64) -> usize {
    let lines = (height / (font_size * LINE_HEIGHT_EM) + ROUNDING).floor();
    if lines.is_finite() && lines >= 1.0 {
        // Saturates for an absurd height, which validation refuses anyway.
        lines as usize
    } else {
        1
    }
}

/// One language's recent final segments, in segment order, and the words still being said.
#[derive(Clone, Debug, Default)]
pub(crate) struct Track {
    entries: Vec<(u64, String)>,
    live: String,
    /// When the track last changed, for clearing it a while after.
    changed_ms: u64,
}

impl Track {
    /// Adds the final segment `id` in its place among the others, which may have arrived later.
    pub(crate) fn push(&mut self, id: u64, text: &str, now_ms: u64) {
        let at = self.entries.partition_point(|(other, _)| *other < id);
        self.entries.insert(at, (id, text.to_owned()));
        if self.entries.len() > KEPT_ENTRIES {
            self.entries.remove(0);
        }
        self.changed_ms = now_ms;
    }

    /// Shows `text` as the words still being said; false when they did not change.
    pub(crate) fn set_live(&mut self, text: &str, now_ms: u64) -> bool {
        self.changed_ms = now_ms;
        if self.live == text {
            return false;
        }
        text.clone_into(&mut self.live);
        true
    }

    /// Drops the words still being said; false when there were none.
    pub(crate) fn clear_live(&mut self) -> bool {
        let had = !self.live.is_empty();
        self.live.clear();
        had
    }

    /// Drops everything; false when there was nothing.
    pub(crate) fn clear(&mut self) -> bool {
        let had = !self.is_empty();
        self.entries.clear();
        self.live.clear();
        had
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.live.is_empty()
    }

    /// Whether the track shows something that has not changed for `clear_after_ms`.
    pub(crate) fn expired(&self, now_ms: u64, clear_after_ms: u64) -> bool {
        !self.is_empty() && now_ms >= self.changed_ms.saturating_add(clear_after_ms)
    }

    pub(crate) fn live(&self) -> &str {
        &self.live
    }

    /// The kept final segments, by id, oldest first.
    pub(crate) fn entries(&self) -> &[(u64, String)] {
        &self.entries
    }

    /// The last `keep` lines of the final segments, each starting a new line, wrapped at
    /// `max_chars`, with the words still being said after them when `with_live`.
    pub(crate) fn lines(&self, max_chars: usize, keep: usize, with_live: bool) -> Vec<String> {
        let live = with_live.then_some(self.live.as_str());
        let texts = self.entries.iter().map(|(_, text)| text.as_str()).chain(live);
        let lines: Vec<String> = texts.flat_map(|text| wrap(text, max_chars)).collect();
        let skip = lines.len().saturating_sub(keep);
        lines.into_iter().skip(skip).collect()
    }
}
