// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Local commands beyond cards, read from the tables in [`commands`](super::commands): a look,
//! subtitles on or off or into a language, a theme, showing or hiding an element, and playing a
//! reaction with "play". They come before cards, except that "show" or "hide" with words that
//! name a card as well as an element does nothing and says so.

use super::commands::{
    HIDE, LOOK, NEXT_ROUND, PLAY, REVEAL, SHOW, SUBTITLES_INTO, SUBTITLES_OFF, SUBTITLES_ON, THEME, THEME_OFF, bare,
    bare_words, said_one_of, slots, theme_named,
};
use super::matcher::{Intent, Miss};
use super::reacting::top_reaction;
use super::scoring::{MIN_SCORE, names, top_card};
use super::{RoomStep, VoiceAction};
use crate::decks::Deck;
use crate::model::{Element, Project};
use crate::subtitles::{constructed_languages, natural_languages};
use crate::theme::Theme;

/// The local command the words ask for; `None` when they ask for none of these.
pub(super) fn local_command(asked: &[String], project: &Project, decks: &[&Deck]) -> Option<Result<Intent, Miss>> {
    let said = bare(asked);
    let act = |action: VoiceAction| Some(Ok(Intent::Act(action)));
    if said_one_of(&said, &LOOK) {
        return act(VoiceAction::Look);
    }
    for (forms, step) in [(&REVEAL[..], RoomStep::Reveal), (&NEXT_ROUND[..], RoomStep::Next)] {
        if said_one_of(&said, forms) {
            return act(VoiceAction::Room { step });
        }
    }
    for (forms, enabled) in [(&SUBTITLES_ON, true), (&SUBTITLES_OFF, false)] {
        if said_one_of(&said, forms) {
            return act(VoiceAction::Subtitles { enabled });
        }
    }
    for named in slots(&said, &SUBTITLES_INTO) {
        match language_named(named) {
            Ok(Some(language)) => return act(VoiceAction::SubtitleLanguage { language }),
            Ok(None) => {}
            Err(why) => return Some(Err(Miss::refused(why))),
        }
    }
    if said_one_of(&said, &THEME_OFF) {
        return act(VoiceAction::Theme { theme: Theme::None });
    }
    if let Some(theme) = slots(&said, &THEME).into_iter().find_map(theme_named) {
        return act(VoiceAction::Theme { theme });
    }
    if let Some(shown) = element_command(asked, &said, project, decks) {
        return Some(shown);
    }
    let played = slots(&said, &PLAY).into_iter().find_map(top_reaction);
    played.map(|(_, reaction)| Ok(Intent::Act(VoiceAction::React { reaction })))
}

/// Showing or hiding the element the words name, by its name or id; deck slots are left to their
/// cards.
fn element_command(
    asked: &[String],
    said: &[String],
    project: &Project,
    decks: &[&Deck],
) -> Option<Result<Intent, Miss>> {
    for (forms, visible) in [(&SHOW[..], true), (&HIDE[..], false)] {
        for named in slots(said, forms) {
            let found = elements_named(named, project);
            let element = match found.as_slice() {
                [] => continue,
                [element] => element,
                several => {
                    let all: Vec<&str> = several.iter().map(|element| element.name.as_str()).collect();
                    let reason = format!("{:?} could be the elements {}", asked.join(" "), all.join(" or "));
                    return Some(Err(Miss::refused(reason)));
                }
            };
            if let Some((_, deck, card)) = top_card(asked, decks).filter(|(score, ..)| *score >= MIN_SCORE) {
                let card = names(&[(deck, card)]);
                let reason = format!("{:?} could be the element {} or {card}", asked.join(" "), element.name);
                return Some(Err(Miss::refused(reason)));
            }
            let element = element.id.clone();
            return Some(Ok(Intent::Act(VoiceAction::ShowElement { element, visible })));
        }
    }
    None
}

fn elements_named<'p>(named: &[String], project: &'p Project) -> Vec<&'p Element> {
    let slot = |element: &Element| project.decks.iter().any(|deck| deck.element == element.id);
    project
        .elements
        .iter()
        .filter(|element| !slot(element))
        .filter(|element| {
            let name = bare_words(&element.name);
            (!name.is_empty() && name == named) || bare_words(&element.id) == named
        })
        .collect()
}

/// The code of the language the words name, by its whole name, with what is in parentheses
/// first or not ("simplified Chinese"), or by its name without them when only one language has
/// it; why not when several do.
fn language_named(named: &[String]) -> Result<Option<String>, String> {
    let said = fold(&named.join(" "));
    let languages = || natural_languages().iter().chain(constructed_languages());
    let spoken = |name: &str| fold(&bare_words(name).join(" "));
    let exact = languages().find(|(_, name)| {
        let (base, extra) = name.split_once(" (").unwrap_or((name, ""));
        spoken(name) == said || (!extra.is_empty() && spoken(&format!("{extra} {base}")) == said)
    });
    if let Some((code, _)) = exact {
        return Ok(Some((*code).to_owned()));
    }
    let partial: Vec<&(&str, &str)> = languages()
        .filter(|(_, name)| name.split_once(" (").is_some_and(|(base, _)| spoken(base) == said))
        .collect();
    match partial.as_slice() {
        [] => Ok(None),
        [(code, _)] => Ok(Some((*code).to_owned())),
        several => {
            let all: Vec<&str> = several.iter().map(|(_, name)| *name).collect();
            Err(format!("{:?} could be {}", named.join(" "), all.join(" or ")))
        }
    }
}

/// Lowercase text with the accents of the listed language names dropped, as speech to text
/// often writes them.
fn fold(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'ā' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'ē' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'ī' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ō' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'ū' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            other => other,
        })
        .collect()
}
