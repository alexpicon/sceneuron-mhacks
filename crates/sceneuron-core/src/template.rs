// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Placeholders in overlay text that show what the camera and the vision model saw, how a shared
//! room voted, and what the speaker is saying.
//!
//! `{{seen}}` becomes the live description from on-device analysis, `{{scene}}` the latest scene
//! caption, `{{scene:age}}` that caption with how long ago its camera image was taken, `{{room}}`
//! the shared room's summary (who voted, then the revealed cards), `{{subtitles}}` the last lines
//! of live subtitles in the spoken language, `{{subtitles:<code>}}` those in a target language,
//! named by its code or by its place among the targets (`{{subtitles:1}}` for the first), and
//! `{{condition-id}}` the detail the model reported for that condition, such as the name of an
//! object held up to the camera. `{{on-air}}` becomes OBS's current scene, then "LIVE" while OBS
//! streams or records. Unknown placeholders render as nothing, so a typo never shows raw braces on
//! stream, except `{{on-air-time}}`, the time on air, which stays for the overlay to fill in and
//! count every second (see [`crate::on_air`]); a line of placeholders alone that all show nothing
//! is left out, so text never gains an empty line.

use std::collections::BTreeMap;

use crate::engine::Engine;
use crate::on_air::OnAir;
use crate::props::ElementProps;
use crate::scene::ConditionState;
use crate::subtitles::{LATIN_LINE, SubtitleBar, SubtitleState, chars_for_box, label_room, lines_for_box};

const SUBTITLES: &str = "subtitles";
const ON_AIR: &str = "on-air";
/// The overlay fills this one in, so the time counts between snapshots.
const ON_AIR_TIME: &str = "on-air-time";
const SCENE: &str = "scene";
/// The placeholder for the live description.
pub(crate) const SEEN: &str = "seen";
/// The placeholder for the scene caption with its age.
pub(crate) const SCENE_AGE: &str = "scene:age";

/// What placeholders show.
pub(crate) struct Sources<'a> {
    pub caption: &'a str,
    /// How long ago the caption's camera image was taken, such as "just now".
    pub caption_age: &'a str,
    pub seen: &'a str,
    pub room: &'a str,
    pub conditions: &'a BTreeMap<String, ConditionState>,
    pub subtitles: &'a SubtitleState,
    pub on_air: &'a OnAir,
}

impl Engine {
    /// What placeholders show now.
    pub(crate) fn sources(&self) -> Sources<'_> {
        Sources {
            caption: &self.scene.caption,
            caption_age: &self.scene.age,
            seen: &self.live.seen,
            room: &self.room_summary,
            conditions: &self.conditions,
            subtitles: &self.subtitles,
            on_air: &self.on_air,
        }
    }

    /// Fills in the placeholders of `props.text` as the overlay shows them, with subtitles wrapped
    /// to fit its box beside their label and cut to the lines its height holds, a Latin line under
    /// each included, and written in a script of their own where they have one. A text that shows
    /// subtitles or the live description hides while it is empty, as between sentences or while no
    /// camera is analyzed; for subtitles it returns what the overlay adds to them.
    pub(crate) fn render_placeholders(&self, props: &mut ElementProps) -> Option<SubtitleBar> {
        let live = placeholders(&props.text).any(|name| matches!(name, Placeholder::Subtitles(_) | Placeholder::Seen));
        let shown = shown_subtitles(&props.text);
        let look = (!shown.is_empty()).then(|| self.subtitles.look(&shown, alone(&props.text)));
        let (label, latin) = look
            .as_ref()
            .map_or(("", false), |look| (look.label.as_str(), look.latin));
        let line_size = if latin { 1.0 + LATIN_LINE } else { 1.0 };
        let fit = Fit {
            chars: chars_for_box(props.width - label_room(label, props.font_size), props.font_size),
            lines: lines_for_box(props.height, props.font_size * line_size),
            native: true,
        };
        let sources = self.sources();
        let latin = if latin {
            let lines = render(&props.text, &sources, Fit { native: false, ..fit });
            lines.lines().map(str::to_owned).collect()
        } else {
            Vec::new()
        };
        props.text = render(&props.text, &sources, fit);
        props.visible &= !(live && props.text.trim().is_empty());
        look.map(|look| SubtitleBar {
            label: look.label,
            script: look.script.map(|script| script.id().to_owned()).unwrap_or_default(),
            latin,
        })
    }
}

/// How much subtitle text a box holds: characters on a line, and lines; with `native`, subtitles
/// with a script of their own are written in it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Fit {
    pub chars: usize,
    pub lines: usize,
    pub native: bool,
}

/// What a placeholder shows, by its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Placeholder<'a> {
    Seen,
    Scene,
    SceneAge,
    Room,
    /// OBS's current scene, then LIVE while it streams or records.
    OnAir,
    /// Kept as written for the overlay to count.
    OnAirTime,
    /// Subtitles in the spoken language (`None`) or in a target language.
    Subtitles(Option<&'a str>),
    /// A condition's detail; nothing for a name that is no condition.
    Condition(&'a str),
}

impl<'a> Placeholder<'a> {
    /// The placeholder `name` asks for; space around the name and after a colon is ignored.
    fn parse(name: &'a str) -> Placeholder<'a> {
        let name = name.trim();
        let argument = |base: &str| Some(name.strip_prefix(base)?.strip_prefix(':')?.trim());
        match name {
            SEEN => Placeholder::Seen,
            ON_AIR => Placeholder::OnAir,
            ON_AIR_TIME => Placeholder::OnAirTime,
            SCENE => Placeholder::Scene,
            "room" => Placeholder::Room,
            SUBTITLES => Placeholder::Subtitles(None),
            _ if argument(SCENE) == Some("age") => Placeholder::SceneAge,
            _ => match argument(SUBTITLES) {
                Some(target) => Placeholder::Subtitles(Some(target)),
                None => Placeholder::Condition(name),
            },
        }
    }

    fn value(self, sources: &Sources, fit: Fit) -> String {
        match self {
            Placeholder::Seen => sources.seen.to_owned(),
            Placeholder::OnAir => sources.on_air.text(),
            Placeholder::OnAirTime => format!("{{{{{ON_AIR_TIME}}}}}"),
            Placeholder::Scene => sources.caption.to_owned(),
            Placeholder::SceneAge if sources.caption.is_empty() => String::new(),
            Placeholder::SceneAge => format!("{} ({})", sources.caption, sources.caption_age),
            Placeholder::Room => sources.room.to_owned(),
            Placeholder::Subtitles(target) => sources.subtitles.text(target, fit.chars, fit.lines, fit.native),
            Placeholder::Condition(id) => sources
                .conditions
                .get(id)
                .map(|state| state.detail.clone())
                .unwrap_or_default(),
        }
    }
}

/// A piece of a text: words as written, or a placeholder.
enum Piece<'a> {
    Literal(&'a str),
    Placeholder(Placeholder<'a>),
}

/// The pieces of `text`, scanned the way the editor scans it: an unclosed `{{` is written as is.
fn pieces(text: &str) -> Vec<Piece<'_>> {
    let mut pieces = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            break;
        };
        pieces.push(Piece::Literal(&rest[..start]));
        pieces.push(Piece::Placeholder(Placeholder::parse(&after[..end])));
        rest = &after[end + 2..];
    }
    pieces.push(Piece::Literal(rest));
    pieces
}

fn placeholders(text: &str) -> impl Iterator<Item = Placeholder<'_>> {
    pieces(text).into_iter().filter_map(|piece| match piece {
        Piece::Placeholder(placeholder) => Some(placeholder),
        Piece::Literal(_) => None,
    })
}

/// One line of a text as written, once rendered.
#[derive(Default)]
struct Line {
    text: String,
    placeholders: bool,
    shows_something: bool,
}

/// Replaces every `{{name}}` in `text`; subtitles are fitted to `fit`. A line of the text that
/// holds placeholders and nothing else, which all show nothing, is left out with its line break.
pub(crate) fn render(text: &str, sources: &Sources, fit: Fit) -> String {
    if !text.contains("{{") {
        return text.to_owned();
    }
    let mut lines = vec![Line::default()];
    for piece in pieces(text) {
        match piece {
            Piece::Literal(words) => {
                for (index, part) in words.split('\n').enumerate() {
                    if index > 0 {
                        lines.push(Line::default());
                    }
                    let line = lines.last_mut().expect("there is always a line");
                    line.text.push_str(part);
                    line.shows_something |= !part.trim().is_empty();
                }
            }
            Piece::Placeholder(placeholder) => {
                let value = placeholder.value(sources, fit);
                let line = lines.last_mut().expect("there is always a line");
                line.placeholders = true;
                line.shows_something |= !value.trim().is_empty();
                line.text.push_str(&value);
            }
        }
    }
    let kept: Vec<String> = lines
        .into_iter()
        .filter(|line| !line.placeholders || line.shows_something)
        .map(|line| line.text)
        .collect();
    kept.join("\n")
}

/// Whether `text` shows subtitles, in any language; such text hides while they are empty.
pub(crate) fn shows_subtitles(text: &str) -> bool {
    !shown_subtitles(text).is_empty()
}

/// The subtitles `text` shows, in order: `None` for the spoken language, or a target's name.
fn shown_subtitles(text: &str) -> Vec<Option<&str>> {
    placeholders(text)
        .filter_map(|name| match name {
            Placeholder::Subtitles(target) => Some(target),
            _ => None,
        })
        .collect()
}

/// Whether `text` is one placeholder and nothing else, around it only spaces.
fn alone(text: &str) -> bool {
    let text = text.trim();
    text.starts_with("{{") && text.ends_with("}}") && text.matches("{{").count() == 1 && text.matches("}}").count() == 1
}

/// Whether `text` uses the placeholder `name`, such as [`SEEN`], in any spacing.
pub(crate) fn shows_placeholder(text: &str, name: &str) -> bool {
    let wanted = Placeholder::parse(name);
    placeholders(text).any(|placeholder| placeholder == wanted)
}

/// Whether overlay text shows the vision model's scene caption, through `{{scene}}` or
/// `{{scene:age}}` in any spacing, so that looks ask the model for one.
pub fn shows_caption(text: &str) -> bool {
    placeholders(text).any(|name| matches!(name, Placeholder::Scene | Placeholder::SceneAge))
}

/// Whether `text` names the placeholder `name`, such as `on-air`.
pub(crate) fn shows(text: &str, name: &str) -> bool {
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            return false;
        };
        if after[..end].trim() == name {
            return true;
        }
        rest = &after[end + 2..];
    }
    false
}

/// Whether `text` shows OBS's scene and whether it is live.
pub(crate) fn shows_on_air(text: &str) -> bool {
    shows(text, ON_AIR)
}

/// Whether `text` shows the time on air, which the overlay keeps counting.
pub(crate) fn shows_clock(text: &str) -> bool {
    shows(text, ON_AIR_TIME)
}
