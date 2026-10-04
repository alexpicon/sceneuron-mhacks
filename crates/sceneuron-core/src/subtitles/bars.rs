// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What a text that shows subtitles adds to its words on the overlay: a small label naming their
//! language, the script they are written in when a constructed language has one, and under each
//! of their lines the same line in small Latin letters when the streamer asks for it. The label
//! sits beside the words and the Latin lines under them, so they take room from the box, which the
//! words are wrapped and cut to leave; `web/render-subtitles.js` draws them in that room.

use serde::Serialize;

use super::scripts::NativeScript;
use super::{SubtitleState, language_name};

/// The label's type, as a fraction of the words'.
pub const LABEL_SCALE: f64 = 0.4;
/// A Latin line's type under a line in a script of its own, as a fraction of the words'.
pub const LATIN_SCALE: f64 = 0.5;
/// A Latin line's height, in lines of the words: its type, and room above it for the long
/// descenders of Tengwar.
pub const LATIN_LINE: f64 = 0.65;
/// Average width of a character of the label, as a fraction of its type; wide enough for bold and
/// monospace type, so a label never needs more room than it is given.
const LABEL_CHAR_EM: f64 = 0.64;
/// The label's padding, as a fraction of its type, and the gap after it, of the words' type.
const LABEL_PADDING_EM: f64 = 1.0;
const LABEL_GAP_EM: f64 = 0.35;

/// What a text that shows subtitles adds to its words on the overlay.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SubtitleBar {
    /// The language of its subtitles in words, such as `English (detected)` or `Japanese`, for a
    /// small label beside them; empty while labels are off or the language is not known yet.
    pub label: String,
    /// The script its subtitles are written in when that is not Latin letters: `tengwar`,
    /// `piqad`, or `sitelen-pona`; empty otherwise.
    pub script: String,
    /// Each of its lines in Latin letters, for a small line under each line in `script`; empty
    /// unless the streamer asks for them.
    pub latin: Vec<String>,
}

/// How a text that shows subtitles is drawn, before its lines are filled in.
#[derive(Clone, Debug, Default)]
pub(crate) struct Look {
    pub(crate) label: String,
    /// The script of the one target the text shows and nothing else, when it is written in one.
    pub(crate) script: Option<NativeScript>,
    /// Latin lines go under its lines.
    pub(crate) latin: bool,
}

/// The room a label takes beside words in type `font_size`, its gap included; none for no label.
pub fn label_room(label: &str, font_size: f64) -> f64 {
    let chars = label.chars().count();
    if chars == 0 {
        return 0.0;
    }
    let label_em = chars as f64 * LABEL_CHAR_EM + LABEL_PADDING_EM;
    (label_em * LABEL_SCALE + LABEL_GAP_EM) * font_size
}

impl SubtitleState {
    /// How a text showing `shown`, each the spoken language (`None`) or a target by code or place,
    /// is drawn; `alone` when it is a single placeholder and nothing else. The label names each
    /// language it shows, in order; a target that is not set names nothing.
    pub(crate) fn look(&self, shown: &[Option<&str>], alone: bool) -> Look {
        let mut labels: Vec<String> = Vec::new();
        let mut codes = Vec::new();
        for placeholder in shown {
            let label = match placeholder {
                None => self.spoken_label(),
                Some(name) => match self.target(name) {
                    Some(code) => {
                        codes.push(code);
                        language_name(code).map_or_else(|| code.to_owned(), str::to_owned)
                    }
                    None => String::new(),
                },
            };
            if !label.is_empty() && !labels.contains(&label) {
                labels.push(label);
            }
        }
        let script = match (alone, codes.as_slice()) {
            (true, [code]) => self.native(code),
            _ => None,
        };
        Look {
            label: if self.status.labels {
                labels.join(" · ")
            } else {
                String::new()
            },
            script,
            latin: script.is_some() && self.status.latin_lines,
        }
    }

    /// The script the overlay writes the target `code` in, unless the streamer asked for Latin
    /// letters.
    pub(crate) fn native(&self, code: &str) -> Option<NativeScript> {
        let latin = self
            .status
            .latin_targets
            .iter()
            .any(|listed| listed.eq_ignore_ascii_case(code));
        NativeScript::of(code).filter(|_| !latin)
    }

    /// The spoken language's name: as set, or as detected with a note that it was; empty while
    /// detection has not said.
    fn spoken_label(&self) -> String {
        let status = &self.status;
        let named = |code: &str| language_name(code).map_or_else(|| code.to_owned(), str::to_owned);
        if !status.language.is_empty() {
            return named(&status.language);
        }
        if status.detected.is_empty() {
            return String::new();
        }
        let name = named(&status.detected);
        match name.strip_suffix(')') {
            Some(open) => format!("{open}, detected)"),
            None => format!("{name} (detected)"),
        }
    }
}
