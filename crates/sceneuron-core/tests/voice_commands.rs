// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the local commands beyond cards: every reaction by its name, an alias, or "play";
//! themes; showing and hiding elements; looks; subtitles on, off, and into a language; the
//! ambiguities that do nothing and say why; and what goes to the AI instead.

mod common;

use common::text_element;
use common::voice::{ask, decks, held, raise, reason, said};
use sceneuron_core::{
    Canvas, Flood, Intent, Project, Theme, VoiceAction, VoiceMode, default_project, interpret, pack_for_canvas,
};

/// The default project, with Be right back, Name label, and Camera frame, and the meeting deck
/// with its voice on.
fn project() -> Project {
    let meeting = pack_for_canvas("meeting", &Canvas::default()).unwrap();
    let mut project = default_project().add_pack(&meeting).unwrap();
    project.decks.iter_mut().for_each(|deck| deck.voice = true);
    project
}

fn addressed(text: &str, project: &Project) -> Intent {
    interpret(text, true, VoiceMode::PushToTalk, "Sceneuron", project)
}

fn act(action: VoiceAction) -> Intent {
    Intent::Act(action)
}

fn react(reaction: Flood) -> Intent {
    act(VoiceAction::React { reaction })
}

fn theme(theme: Theme) -> Intent {
    act(VoiceAction::Theme { theme })
}

fn show(element: &str, visible: bool) -> Intent {
    act(VoiceAction::ShowElement {
        element: element.to_owned(),
        visible,
    })
}

fn into(language: &str) -> Intent {
    act(VoiceAction::SubtitleLanguage {
        language: language.to_owned(),
    })
}

#[test]
fn every_reaction_plays_by_its_name_an_alias_or_play() {
    let meeting = decks(&["meeting"]);
    for (words, flood) in [
        ("Sceneuron, email", Flood::Email),
        ("Sceneuron, this meeting could have been an email", Flood::Email),
        ("Sceneuron, mind blown", Flood::MindBlown),
        ("Sceneuron, that blew my mind", Flood::MindBlown),
        ("Sceneuron, play ship it", Flood::ShipIt),
        ("Sceneuron, play the ship it reaction", Flood::ShipIt),
        ("Sceneuron, coffee break", Flood::Coffee),
        ("Sceneuron, tongue", Flood::Tongue),
        ("Sceneuron, stick your tongue out", Flood::Tongue),
    ] {
        assert_eq!(said(words, &meeting), react(flood), "{words}");
    }
    assert_eq!(held("mind blown please", &meeting), react(Flood::MindBlown));
    assert_eq!(held("send an email to Dana", &meeting), ask("send an email to Dana"));
}

#[test]
fn a_reaction_named_like_a_card_plays_only_with_play() {
    let both = decks(&["meeting", "poker"]);
    assert_eq!(
        reason(held("coffee break", &both)),
        "\"coffee\" could be the Coffee break reaction or Planning poker: ☕; say \"play coffee break\" for the \
         reaction"
    );
    assert_eq!(held("play coffee break", &both), react(Flood::Coffee));
    assert_eq!(held("vote coffee", &both), raise("poker", "break"));
}

#[test]
fn themes_switch_by_name_and_turn_off() {
    let project = project();
    for (words, chosen) in [
        ("hacker theme", Theme::Hacker),
        ("use the 80s theme", Theme::Eighties),
        ("switch to the Windows 95 theme", Theme::Windows95),
        ("classic mac theme", Theme::ClassicMac),
        ("theme vaporwave", Theme::Vaporwave),
        ("change the theme to retro", Theme::Eighties),
        ("theme off", Theme::None),
        ("no theme", Theme::None),
        ("plain theme", Theme::None),
    ] {
        assert_eq!(addressed(words, &project), theme(chosen), "{words}");
    }
    assert_eq!(
        addressed("pirate theme", &project),
        ask("pirate theme"),
        "no such theme"
    );
}

#[test]
fn elements_show_and_hide_by_name_or_id_but_card_slots_are_left_to_cards() {
    let mut project = project();
    assert_eq!(addressed("show be right back", &project), show("brb", true));
    assert_eq!(
        addressed("Sceneuron, hide the name label", &project),
        show("name-label", false)
    );
    assert_eq!(
        addressed("bring back the camera frame", &project),
        show("cam-frame", true)
    );
    assert_eq!(addressed("hide brb", &project), show("brb", false));
    assert_eq!(
        addressed("hide the meeting card", &project),
        Intent::Lower {
            decks: vec!["meeting".to_owned()]
        },
        "hiding a slot lowers its deck"
    );
    assert_eq!(
        addressed("show the meeting card", &project),
        ask("show the meeting card")
    );
    project.elements.push(text_element("topic", "Next topic"));
    assert_eq!(
        reason(addressed("show next topic", &project)),
        "\"show next topic\" could be the element Next topic or Meeting signals: Next topic"
    );
    project.elements.push(text_element("brb-2", "Be right back"));
    assert_eq!(
        reason(addressed("show be right back", &project)),
        "\"show be right back\" could be the elements Be right back or Be right back"
    );
}

#[test]
fn a_look_and_subtitles_are_asked_for_by_name() {
    let project = project();
    for words in [
        "look now",
        "take a look",
        "Sceneuron, look at the camera",
        "have a look please",
    ] {
        assert_eq!(addressed(words, &project), act(VoiceAction::Look), "{words}");
    }
    for (words, enabled) in [
        ("subtitles on", true),
        ("turn on subtitles", true),
        ("captions on", true),
        ("turn subtitles off", false),
        ("stop subtitles", false),
        ("turn off captions", false),
    ] {
        assert_eq!(
            addressed(words, &project),
            act(VoiceAction::Subtitles { enabled }),
            "{words}"
        );
    }
}

#[test]
fn subtitles_go_into_a_language_named_as_the_editor_names_it() {
    let project = project();
    for (words, code) in [
        ("subtitles in Spanish", "es"),
        ("translate into Japanese", "ja"),
        ("translate subtitles into French", "fr"),
        ("Klingon subtitles", "tlh"),
        ("subtitles in simplified Chinese", "zh-Hans"),
        ("subtitles in Chinese Traditional", "zh-Hant"),
        ("subtitles into Quenya", "qya"),
        ("subtitles in Maori", "mi"),
        ("subtitles in Spanish Latin America", "es-419"),
    ] {
        assert_eq!(addressed(words, &project), into(code), "{words}");
    }
    assert_eq!(
        reason(addressed("subtitles in Chinese", &project)),
        "\"chinese\" could be Chinese (Simplified) or Chinese (Traditional)"
    );
    assert_eq!(
        addressed("subtitles in Martian", &project),
        ask("subtitles in Martian"),
        "a language the list lacks goes to the AI"
    );
}

#[test]
fn commands_come_before_cards_and_anything_else_goes_to_the_ai() {
    let project = project();
    assert_eq!(addressed("pause", &project), Intent::Pause);
    assert_eq!(addressed("wrap it up", &project), raise("meeting", "wrap-up"));
    let request = "make the name label say back in five";
    assert_eq!(addressed(request, &project), ask(request));
    assert_eq!(
        said("Sceneuron, make it rain", &decks(&["meeting"])),
        ask("Sceneuron, make it rain")
    );
    assert!(matches!(
        said("make it rain", &decks(&["meeting"])),
        Intent::Ignore { addressed: false, .. }
    ));
}
