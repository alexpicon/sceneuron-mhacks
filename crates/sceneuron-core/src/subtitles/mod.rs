// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Live subtitles: what the speaker says, line by line, in their own language and in up to three
//! others, for `{{subtitles}}` and `{{subtitles:<code>}}` in overlay text.
//!
//! The host transcribes speech and feeds it through [`Engine::hear`](crate::Engine::hear):
//! partial words update the live line, and each final segment becomes a line of its own with an
//! id. When target languages are set, a final segment is also a translation job, which the host
//! carries out and reports back by that id, in any order. Each language keeps its last few lines
//! and clears a while after it last changed. Speech addressed to Sceneuron is left out unless the
//! host says otherwise. Subtitle text is meant to be shown, so the snapshot carries it; the event
//! log never does. On the overlay each text that shows subtitles names their language in a small
//! label, and a constructed language with a script of its own is written in it (see bars.rs and
//! scripts/).

mod bars;
mod engine;
mod language;
mod lines;
mod long_codes;
mod names;
mod scripts;
mod translations;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::Serialize;

pub use bars::{LABEL_SCALE, LATIN_LINE, LATIN_SCALE, SubtitleBar, label_room};
pub use language::{canonical_language_tag, same_language, valid_language_tag};
pub use lines::{MAX_LINE_CHARS, wrap};
pub use names::{constructed_language, constructed_languages, language_label, language_name, natural_languages};
pub use scripts::NativeScript;

use lines::Track;
pub(crate) use lines::{chars_for_box, lines_for_box};

/// Target languages a speaker may have at once.
pub const MAX_TARGETS: usize = 3;
/// Lines each language shows at most.
pub const MAX_LINES: u8 = 3;
/// Lines each language shows unless the host says otherwise.
pub const DEFAULT_LINES: u8 = 2;
/// How long a language's lines stay after they last changed, unless the host says otherwise.
pub const DEFAULT_CLEAR_AFTER_MS: u64 = 4_000;

/// How subtitles are set up and what they are doing, reported by the host.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SubtitleStatus {
    /// Subtitles are on, agreed to, and can listen; heard words become subtitles only then.
    pub enabled: bool,
    /// `off`, `needs_consent`, `needs_key`, `connecting`, `listening`, `retrying`, `stopped`, or
    /// `paused` while the hourly budget is spent.
    pub state: String,
    /// What subtitles are doing or need, for the editor; never shown on the video.
    pub message: String,
    /// The spoken language as an ISO 639 code, or empty when it is detected.
    pub language: String,
    /// The spoken language as speech to text last detected it, by the ISO 639 code it reports,
    /// while `language` is empty; empty until it reports one.
    pub detected: String,
    /// BCP 47 codes of the languages each line is translated into, in order.
    pub targets: Vec<String>,
    /// Lines each language shows, 1 to [`MAX_LINES`].
    pub lines: u8,
    /// How long a language's lines stay after they last changed.
    pub clear_after_ms: u64,
    /// Speech addressed to Sceneuron, by the wake word or push to talk, is left out while voice
    /// control is on.
    pub hide_commands: bool,
    /// Final lines go to the stream as closed captions.
    pub stream_captions: bool,
    /// The language of those captions: empty for the spoken one, or one of the targets.
    pub stream_language: String,
    /// Each text that shows subtitles on the overlay names their language in a small label.
    pub labels: bool,
    /// Targets with a script of their own, such as Quenya's Tengwar, that the overlay shows in
    /// Latin letters instead; the others are written in their script.
    pub latin_targets: Vec<String>,
    /// Under each line written in a script of its own, the overlay shows it in small Latin letters.
    pub latin_lines: bool,
}

impl Default for SubtitleStatus {
    fn default() -> Self {
        Self {
            enabled: false,
            state: "off".to_owned(),
            message: String::new(),
            language: String::new(),
            detected: String::new(),
            targets: Vec::new(),
            lines: DEFAULT_LINES,
            clear_after_ms: DEFAULT_CLEAR_AFTER_MS,
            hide_commands: true,
            stream_captions: false,
            stream_language: String::new(),
            labels: true,
            latin_targets: Vec::new(),
            latin_lines: false,
        }
    }
}

/// Subtitles as the editor and the overlay see them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SubtitlesView {
    /// The final lines in the spoken language, oldest first, wrapped at [`MAX_LINE_CHARS`].
    pub lines: Vec<String>,
    /// The final segments in the spoken language that are kept, whole and with their ids, oldest
    /// first, so that a screen reader can hear each new one once, even when it wraps or repeats
    /// the one before.
    pub finals: Vec<FinalSegment>,
    /// The words of the segment still being said; they may still change.
    pub live: String,
    /// The final lines in each target language, by its code.
    pub translations: BTreeMap<String, Vec<String>>,
    pub status: SubtitleStatus,
}

/// A final segment in the spoken language, unwrapped; `id` grows with every segment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FinalSegment {
    pub id: u64,
    pub text: String,
}

/// A final segment to translate into `targets`, with the segments before it for consistency.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranslationJob {
    /// Matches the answer to its segment, whenever it arrives.
    pub id: u64,
    pub text: String,
    /// The spoken language, or empty when it is detected.
    pub language: String,
    pub targets: Vec<String>,
    /// Up to [`CONTEXT_SEGMENTS`] earlier segments, oldest first.
    pub context: Vec<ContextLine>,
}

/// An earlier segment and the translations it got.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextLine {
    pub text: String,
    pub translations: BTreeMap<String, String>,
}

/// Earlier segments a translation job carries.
pub const CONTEXT_SEGMENTS: usize = 3;
/// Final segments kept for matching translations and for context.
const KEPT_SEGMENTS: usize = 16;

/// A final segment and what became of its translations.
#[derive(Clone, Debug)]
struct Segment {
    id: u64,
    text: String,
    translations: BTreeMap<String, String>,
    /// Targets whose translation has not arrived yet.
    waiting: BTreeSet<String>,
}

/// The engine's subtitles; kept in memory only, like raised cards.
#[derive(Clone, Debug, Default)]
pub(crate) struct SubtitleState {
    status: SubtitleStatus,
    own: Track,
    /// One track per target, by the code as the status spells it.
    targets: BTreeMap<String, Track>,
    segments: VecDeque<Segment>,
    last_id: u64,
    /// The last translation failure logged, so a failing provider is logged once.
    last_failure: Option<String>,
}
