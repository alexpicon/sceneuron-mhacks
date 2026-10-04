// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The ways to say each local command beyond cards, as tables: the matcher reads them, and so does
//! the guide that tells the streamer what to say, so the two cannot drift. In a form, `{}` stands
//! for what the command names: a reaction, a theme, an element, or a language. Forms and what was
//! said are compared as normalized words without "the", "a", or "an".

use super::normalize::words;
use crate::theme::Theme;

/// Pausing automation.
pub(super) const PAUSE: [&str; 4] = ["pause", "pause automation", "pause it", "stop automation"];
/// Resuming automation.
pub(super) const RESUME: [&str; 4] = ["resume", "resume automation", "unpause", "start automation"];
/// Revealing the shared room's votes.
pub(super) const REVEAL: [&str; 7] = [
    "reveal",
    "reveal votes",
    "reveal the votes",
    "reveal cards",
    "show votes",
    "show everyone's votes",
    "flip cards",
];
/// Starting the shared room's next round.
pub(super) const NEXT_ROUND: [&str; 4] = ["next round", "new round", "start next round", "start a new round"];
/// Asking the vision model to look now; a trailing "now" is dropped as a filler.
pub(super) const LOOK: [&str; 5] = ["look", "take a look", "have a look", "look at the camera", "look again"];
pub(super) const SUBTITLES_ON: [&str; 6] = [
    "subtitles on",
    "turn on subtitles",
    "turn subtitles on",
    "start subtitles",
    "captions on",
    "turn on captions",
];
pub(super) const SUBTITLES_OFF: [&str; 6] = [
    "subtitles off",
    "turn off subtitles",
    "turn subtitles off",
    "stop subtitles",
    "captions off",
    "turn off captions",
];
/// Translating subtitles into a language first.
pub(super) const SUBTITLES_INTO: [&str; 5] = [
    "subtitles in {}",
    "subtitles into {}",
    "translate subtitles into {}",
    "translate into {}",
    "{} subtitles",
];
/// Switching to a theme.
pub(super) const THEME: [&str; 5] = [
    "{} theme",
    "use the {} theme",
    "switch to the {} theme",
    "theme {}",
    "change the theme to {}",
];
/// Turning the theme off.
pub(super) const THEME_OFF: [&str; 4] = ["theme off", "no theme", "turn off the theme", "turn the theme off"];
/// Each theme with its name in the editor and what it may be called, most natural first.
pub(super) const THEMES: [(Theme, &str, &[&str]); 7] = [
    (Theme::None, "None", &["plain", "default", "none", "normal"]),
    (
        Theme::Graphite,
        "Graphite",
        &["graphite", "amber", "graphite and amber"],
    ),
    (Theme::Hacker, "Hacker", &["hacker", "hackers", "matrix"]),
    (
        Theme::Eighties,
        "80s computer",
        &["80s", "80s computer", "eighties", "retro"],
    ),
    (Theme::Windows95, "Windows 95", &["windows 95", "windows"]),
    (Theme::ClassicMac, "Classic Mac", &["classic mac", "mac", "macintosh"]),
    (
        Theme::Vaporwave,
        "Vaporwave",
        &["vaporwave", "vapor wave", "vapour wave"],
    ),
];
/// Showing an element.
pub(super) const SHOW: [&str; 3] = ["show {}", "unhide {}", "bring back {}"];
/// Hiding an element; "hide" lowers a deck first when it names one.
pub(super) const HIDE: [&str; 1] = ["hide {}"];
/// Playing a reaction; its name alone plays it too, unless a card has that name as well.
pub(super) const PLAY: [&str; 5] = [
    "play {}",
    "play the {} reaction",
    "{} reaction",
    "react with {}",
    "do {}",
];

const ARTICLES: [&str; 3] = ["the", "a", "an"];

/// Words without articles, for comparing commands.
pub(super) fn bare(words: &[String]) -> Vec<String> {
    words
        .iter()
        .filter(|word| !ARTICLES.contains(&word.as_str()))
        .cloned()
        .collect()
}

/// The bare words of `text`.
pub(super) fn bare_words(text: &str) -> Vec<String> {
    bare(&words(text))
}

/// Whether the bare words `said` are exactly one of `forms`.
pub(super) fn said_one_of(said: &[String], forms: &[&str]) -> bool {
    forms.iter().any(|form| bare_words(form) == said)
}

/// What the bare words `said` put in the slot of each of `forms` they fit, with the longest fixed
/// words first, so "play the {} reaction" comes before "play {}".
pub(super) fn slots<'a>(said: &'a [String], forms: &[&str]) -> Vec<&'a [String]> {
    let mut fitting: Vec<(usize, &[String])> = forms
        .iter()
        .filter_map(|form| {
            let (before, after) = form.split_once("{}")?;
            let (before, after) = (bare_words(before), bare_words(after));
            let fixed = before.len() + after.len();
            let fits = said.len() > fixed && said.starts_with(&before) && said.ends_with(&after);
            fits.then(|| (fixed, &said[before.len()..said.len() - after.len()]))
        })
        .collect();
    fitting.sort_by_key(|(fixed, _)| std::cmp::Reverse(*fixed));
    fitting.into_iter().map(|(_, named)| named).collect()
}

/// A form with its slot filled, for the guide.
pub(super) fn fill(form: &str, name: &str) -> String {
    form.replace("{}", name)
}

/// The theme the bare words name, by any of its names.
pub(super) fn theme_named(named: &[String]) -> Option<Theme> {
    THEMES
        .iter()
        .find(|(_, _, names)| said_one_of(named, names))
        .map(|(theme, ..)| *theme)
}

/// What the editor calls a theme.
pub(super) fn theme_name(theme: Theme) -> &'static str {
    THEMES
        .iter()
        .find(|(listed, ..)| *listed == theme)
        .map_or("None", |(_, name, _)| name)
}
