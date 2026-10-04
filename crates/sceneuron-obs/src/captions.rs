// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Closed captions for the stream: the final subtitle lines of one chosen language, paced into
//! CEA-608 captions that OBS carries in the streaming output's video.
//!
//! libobs shows each caption text for at least its display duration before the next one it holds
//! (`obs_output_output_caption_text2`), so captions sent faster than they are shown pile up and
//! fall behind the speech. The sender keeps that queue short: one caption at a time, once the one
//! before had its time, and only the newest pieces when more wait. CEA-608 rows hold 32 columns
//! and libobs keeps at most 128 bytes of a caption, so a line goes out in pieces of at most two
//! rows that fit both. A piece with nothing CEA-608 can show, such as Japanese, is skipped, and an
//! empty caption clears the screen once the subtitles' clear delay has passed. Nothing here touches
//! OBS; the output is a trait, so the pacing is testable.

use std::collections::{BTreeSet, VecDeque};

use sceneuron_core::wrap;

/// Characters in a CEA-608 caption row.
pub const ROW_CHARS: usize = 32;
/// Rows a caption piece takes at most.
pub const PIECE_ROWS: usize = 2;
/// The most of a caption text libobs keeps (`CAPTION_LINE_BYTES` in libobs).
pub const MAX_CAPTION_BYTES: usize = 128;
/// Pieces that may wait; older ones are dropped, so captions stay with the speech.
pub const MAX_WAITING: usize = 4;
/// The shortest wait between polls, so an output that refuses captions never makes a thread spin.
pub const MIN_POLL_MS: u64 = 50;
/// The longest wait between polls while something is due, so a stream that stopped drops what
/// waits soon after.
pub const MAX_POLL_MS: u64 = 500;
/// How long a piece stays at least, by its length at a comfortable reading speed.
const CHARS_PER_SECOND: f64 = 15.0;
const MIN_SECONDS: f64 = 1.5;
const MAX_SECONDS: f64 = 4.0;

/// Where captions go: OBS's streaming output, or a stand-in in tests.
pub trait CaptionOutput {
    /// Whether the stream is live, so captions reach viewers.
    fn streaming(&self) -> bool;

    /// Hands `text` over to be shown for at least `seconds`; false when there is no output.
    fn send(&mut self, text: &str, seconds: f64) -> bool;
}

/// `line` in caption pieces of at most two rows and 128 bytes each, broken between words; pieces
/// with nothing CEA-608 can show are left out.
pub fn caption_pieces(line: &str) -> Vec<String> {
    let rows = wrap(line, ROW_CHARS);
    rows.chunks(PIECE_ROWS)
        .map(|rows| fit_bytes(&rows.join(" ")))
        .filter(|piece| piece.chars().any(|c| c.is_ascii_alphanumeric()))
        .collect()
}

/// `text` cut on a character boundary to at most [`MAX_CAPTION_BYTES`].
fn fit_bytes(text: &str) -> String {
    let mut end = text.len().min(MAX_CAPTION_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

/// How long `piece` stays at least, in seconds.
pub fn display_seconds(piece: &str) -> f64 {
    (piece.chars().count() as f64 / CHARS_PER_SECOND).clamp(MIN_SECONDS, MAX_SECONDS)
}

/// Which final lines are new for captions: each segment once, and none from before captions were
/// turned on or their language changed.
#[derive(Debug, Default)]
pub struct CaptionFeed {
    language: Option<String>,
    sent: BTreeSet<u64>,
}

impl CaptionFeed {
    /// The lines among `lines`, by segment id, not handed out yet for `language`. The first call
    /// for a language hands out none, so captions start with what is said next.
    pub fn new_lines(&mut self, language: &str, lines: &[(u64, String)]) -> Vec<String> {
        let ids = lines.iter().map(|(id, _)| *id);
        if self.language.as_deref() != Some(language) {
            self.language = Some(language.to_owned());
            self.sent = ids.collect();
            return Vec::new();
        }
        let fresh: Vec<String> = lines
            .iter()
            .filter(|(id, _)| !self.sent.contains(id))
            .map(|(_, text)| text.clone())
            .collect();
        // Only the ids still kept can come again, so the set stays small.
        self.sent = ids.collect();
        fresh
    }

    /// Forgets the language, as when captions are turned off.
    pub fn reset(&mut self) {
        self.language = None;
        self.sent.clear();
    }
}

/// Paces caption pieces to the output.
#[derive(Debug)]
pub struct CaptionSender {
    waiting: VecDeque<String>,
    /// When the caption on screen has had its time, on the caller's clock.
    free_at_ms: u64,
    /// When the last caption with words went out; it is cleared after the clear delay.
    shown_at_ms: Option<u64>,
    clear_after_ms: u64,
}

impl CaptionSender {
    pub fn new(clear_after_ms: u64) -> CaptionSender {
        CaptionSender {
            waiting: VecDeque::new(),
            free_at_ms: 0,
            shown_at_ms: None,
            clear_after_ms,
        }
    }

    pub fn set_clear_after(&mut self, clear_after_ms: u64) {
        self.clear_after_ms = clear_after_ms;
    }

    /// Queues a final line; when too many pieces wait, the oldest are dropped.
    pub fn push(&mut self, line: &str) {
        self.waiting.extend(caption_pieces(line));
        let excess = self.waiting.len().saturating_sub(MAX_WAITING);
        self.waiting.drain(..excess);
    }

    /// Sends what is due at `now_ms`: the next piece once the one on screen had its time, or an
    /// empty caption once the clear delay passed. Off the air, everything waiting is dropped. With
    /// nothing waiting and nothing on screen, it does not touch the output at all.
    pub fn poll(&mut self, now_ms: u64, output: &mut dyn CaptionOutput) {
        if self.waiting.is_empty() && self.shown_at_ms.is_none() {
            return;
        }
        if !output.streaming() {
            self.reset();
            return;
        }
        if now_ms < self.free_at_ms {
            return;
        }
        if let Some(piece) = self.waiting.pop_front() {
            let seconds = display_seconds(&piece);
            if output.send(&piece, seconds) {
                self.free_at_ms = now_ms + (seconds * 1_000.0) as u64;
                self.shown_at_ms = Some(now_ms);
            }
            return;
        }
        if self
            .shown_at_ms
            .is_some_and(|shown| now_ms >= shown.saturating_add(self.clear_after_ms))
            && output.send("", 0.0)
        {
            self.shown_at_ms = None;
        }
    }

    /// When [`CaptionSender::poll`] next has something to do, if ever.
    pub fn next_due_ms(&self) -> Option<u64> {
        if !self.waiting.is_empty() {
            return Some(self.free_at_ms);
        }
        let clear = self.shown_at_ms?.saturating_add(self.clear_after_ms);
        Some(clear.max(self.free_at_ms))
    }

    /// How long a thread should wait after polling at `now_ms` before it polls again: `None`
    /// while nothing is due, when only a new line or a stop should wake it.
    pub fn wait_ms(&self, now_ms: u64) -> Option<u64> {
        let due = self.next_due_ms()?;
        Some(due.saturating_sub(now_ms).clamp(MIN_POLL_MS, MAX_POLL_MS))
    }

    /// Drops what waits and forgets what is on screen, as when the stream stops.
    pub fn reset(&mut self) {
        self.waiting.clear();
        self.shown_at_ms = None;
        self.free_at_ms = 0;
    }
}
