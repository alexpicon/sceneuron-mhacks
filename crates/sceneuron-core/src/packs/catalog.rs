// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The packs themselves, laid out on a 1920x1080 canvas so they can be combined without
//! overlapping: the status banner at the top, the live description bar below it, classroom
//! signals on the left, meeting signals centered near the bottom, the subtitles bar just above
//! them, and the poker card in the lower right.
//!
//! Video calls compress the picture, so cards use large bold type on solid, distinct colors.
//! Every background is fully opaque, so a raised card covers whatever is under it, and every text
//! color has a contrast of at least 4.5 to 1 against its background.
//! Some cards have camera signals and trigger phrases, but every deck's camera, voice, and
//! speech start off; the user turns them on.

use super::Pack;
use super::live_description::live_description;
use super::parts::{card, colored, labeled, pack, phrase, signal, slot, speech};
use crate::decks::Card;
use crate::model::Element;
use crate::props::{ElementProps, EnterAnimation};
use crate::signals::{CardSignal, HandSign};

pub(super) const LIGHT: &str = "#ffffffff";
pub(super) const DARK: &str = "#111827ff";
const SLATE: &str = "#1e293bff";
const YELLOW: &str = "#fde047ff";

pub(super) fn packs() -> Vec<Pack> {
    vec![
        meeting(),
        poker(),
        classroom(),
        status(),
        subtitles(),
        live_description(),
    ]
}

fn meeting() -> Pack {
    let props = ElementProps {
        x: 380.0,
        y: 840.0,
        width: 1160.0,
        height: 180.0,
        font_size: 72.0,
        corner_radius: 36.0,
        enter: EnterAnimation::Pop,
        ..slot_props()
    };
    let signals = [
        ("wrap-up", "⏱️", "Wrap it up", "#f59e0bff", DARK),
        ("off-track", "🧭", "We're off track", "#c2410cff", LIGHT),
        ("question", "✋", "I have a question", "#2563ebff", LIGHT),
        ("agree", "👍", "Agree", "#15803dff", LIGHT),
        ("disagree", "👎", "Disagree", "#dc2626ff", LIGHT),
        ("cant-hear", "🔇", "Can't hear you", "#7c3aedff", LIGHT),
        ("offline", "🗂️", "Let's take this offline", "#0f766eff", LIGHT),
        ("next-topic", "⏭️", "Next topic", "#db2777ff", LIGHT),
    ];
    let mut cards: Vec<Card> = signals
        .iter()
        .map(|(id, emoji, name, background, color)| colored(id, name, &format!("{emoji} {name}"), background, color))
        .collect();
    // An open palm stays free because hands open while talking. The signs used here count as
    // fingers too whenever the camera sees the thumb: a fist as 0, pointing as 1 or 2, and a
    // peace sign as 2 or 3, one more with the thumb out; and the camera adds up both hands. So
    // while Planning poker's hand signals are on as well, the context decides where such a pose
    // goes: to Planning poker while a vote is open there, and to this deck otherwise.
    let signs = [
        ("wrap-up", HandSign::Fist),
        ("question", HandSign::Pointing),
        ("agree", HandSign::ThumbsUp),
        ("disagree", HandSign::ThumbsDown),
        ("next-topic", HandSign::Peace),
    ];
    for (id, sign) in signs {
        signal(&mut cards, id, CardSignal::Sign(sign));
    }
    let phrases = [
        ("offline", "let's take this offline"),
        ("off-track", "we're getting off track"),
        ("question", "I have a question"),
        ("next-topic", "next topic"),
        ("cant-hear", "I can't hear you"),
    ];
    for (id, words) in phrases {
        phrase(&mut cards, id, words);
    }
    pack(
        "meeting",
        "Meeting signals",
        "Signal a video call without unmuting: wrap it up, we're off track, a question, agree or \
         disagree, and more. Each card hides after 15 seconds.",
        slot("meeting-card", "Meeting card", props),
        15_000,
        cards,
    )
}

fn poker() -> Pack {
    let props = ElementProps {
        x: 1560.0,
        y: 600.0,
        width: 300.0,
        height: 420.0,
        font_size: 130.0,
        color: DARK.to_owned(),
        background: "#f8fafcff".to_owned(),
        border_color: "#3b82f6ff".to_owned(),
        border_width: 8.0,
        corner_radius: 28.0,
        enter: EnterAnimation::Pop,
        ..slot_props()
    };
    // Values up to 8 can be shown with fingers, 8 with two hands.
    let values = [
        ("p0", "0", Some(0)),
        ("p1", "1", Some(1)),
        ("p2", "2", Some(2)),
        ("p3", "3", Some(3)),
        ("p5", "5", Some(5)),
        ("p8", "8", Some(8)),
        ("p13", "13", None),
        ("p21", "21", None),
        ("unsure", "?", None),
        ("break", "☕", None),
    ];
    let mut cards: Vec<Card> = values
        .iter()
        .map(|(id, value, fingers)| Card {
            signal: fingers.map(CardSignal::Fingers),
            ..card(id, value, value)
        })
        .collect();
    speech(&mut cards, "unsure", "I'm not sure");
    speech(&mut cards, "break", "Coffee break");
    pack(
        "poker",
        "Planning poker",
        "Show your estimate in sprint planning: 0, 1, 2, 3, 5, 8, 13, 21, unsure, or time for a \
         break. The card stays up until you lower it.",
        slot("poker-card", "Poker card", props),
        0,
        cards,
    )
}

fn classroom() -> Pack {
    let props = ElementProps {
        x: 60.0,
        y: 400.0,
        width: 760.0,
        height: 220.0,
        font_size: 64.0,
        corner_radius: 32.0,
        enter: EnterAnimation::SlideUp,
        ..slot_props()
    };
    let signals = [
        ("hand", "✋ Hand raised", "#facc15ff", DARK),
        ("question", "❓ Question", "#2563ebff", LIGHT),
        ("repeat", "🔁 Please repeat", "#7c3aedff", LIGHT),
        ("slower", "🐢 Slower, please", "#0f766eff", LIGHT),
        ("got-it", "✅ Got it", "#15803dff", LIGHT),
    ];
    let answers = [
        ("answer-a", "A", "#dc2626ff"),
        ("answer-b", "B", "#1d4ed8ff"),
        ("answer-c", "C", "#b45309ff"),
        ("answer-d", "D", "#166534ff"),
    ];
    let mut cards = labeled(&signals);
    signal(&mut cards, "hand", CardSignal::HandRaised);
    signal(&mut cards, "got-it", CardSignal::Sign(HandSign::ThumbsUp));
    phrase(&mut cards, "question", "I have a question");
    cards.extend(answers.iter().map(|(id, letter, background)| {
        let mut card = colored(id, letter, letter, background, LIGHT);
        card.props.font_size = Some(150.0);
        card
    }));
    pack(
        "classroom",
        "Classroom",
        "Signals for a class or workshop: hand raised, a question, please repeat, slower, got it, \
         and answers A to D. Each card hides after 30 seconds.",
        slot("classroom-card", "Classroom card", props),
        30_000,
        cards,
    )
}

fn status() -> Pack {
    let props = ElementProps {
        x: 360.0,
        y: 40.0,
        width: 1200.0,
        height: 130.0,
        font_size: 60.0,
        corner_radius: 65.0,
        enter: EnterAnimation::Fade,
        ..slot_props()
    };
    let mut cards = labeled(&[
        ("muted", "🔇 I'm on mute", "#dc2626ff", LIGHT),
        ("brb", "☕ Back in 5", "#b45309ff", LIGHT),
        ("lunch", "🍽️ At lunch", "#c2410cff", LIGHT),
        ("focus", "🎧 Focus time", "#4f46e5ff", LIGHT),
        ("presenting", "🖥️ Presenting", "#2563ebff", LIGHT),
        ("on-call", "📞 On another call", "#7c3aedff", LIGHT),
    ]);
    phrase(&mut cards, "brb", "I'll be right back");
    pack(
        "status",
        "Status",
        "A banner at the top that tells the call where you are: on mute, back in 5, at lunch, focus \
         time, presenting, or on another call. It stays up until you lower it.",
        slot("status-card", "Status banner", props),
        0,
        cards,
    )
}

/// Two text bars inside the title-safe area, between the classroom cards and the meeting card:
/// the spoken language in white, then the first target language in yellow. They show live
/// subtitles, so the pack has no deck, and each hides between sentences.
fn subtitles() -> Pack {
    let bar = |id: &str, name: &str, y: f64, text: &str, color: &str| {
        let props = ElementProps {
            x: 380.0,
            y,
            width: 1160.0,
            height: 100.0,
            font_size: 40.0,
            corner_radius: 16.0,
            enter: EnterAnimation::None,
            visible: true,
            text: text.to_owned(),
            color: color.to_owned(),
            background: DARK.to_owned(),
            ..ElementProps::default()
        };
        slot(id, name, props)
    };
    let elements: Vec<Element> = vec![
        bar("subtitles-spoken", "Subtitles", 626.0, "{{subtitles}}", LIGHT),
        bar(
            "subtitles-translated",
            "Translated subtitles",
            732.0,
            "{{subtitles:1}}",
            YELLOW,
        ),
    ];
    Pack {
        id: "subtitles".to_owned(),
        name: "Subtitles".to_owned(),
        description: "A caption bar for live subtitles: what you say, then its first translation below \
                      it. Each line hides between sentences. Turn subtitles on in the AI tab."
            .to_owned(),
        elements,
        decks: Vec::new(),
    }
}

/// What every card slot shares: hidden until a card is raised, empty text, light bold type on
/// a dark fill.
fn slot_props() -> ElementProps {
    ElementProps {
        visible: false,
        text: String::new(),
        color: LIGHT.to_owned(),
        background: SLATE.to_owned(),
        ..ElementProps::default()
    }
}
