// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Ready-made features for simulated plans, and the colors a request can name.

use sceneuron_core::{Condition, Effect, Element, ElementKind, ElementProps, EnterAnimation, IdleAnimation};

use super::features::Feature;
use super::scenes::{Topic, mentions};

/// Stands for the condition id in a feature's element text.
pub(crate) const ID_MARK: &str = "@id";

/// A background color named in the streamer's words, with a readable text color on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Color {
    pub(crate) name: &'static str,
    pub(crate) background: &'static str,
    pub(crate) text: &'static str,
}

const COLORS: [Color; 10] = [
    color("pink", "#ff5c8aee", "#ffffffff"),
    color("red", "#ef4444ee", "#ffffffff"),
    color("orange", "#f97316ee", "#ffffffff"),
    color("yellow", "#ffc233ee", "#1a1a1aff"),
    color("green", "#22c55eee", "#0b1f12ff"),
    color("teal", "#14b8a6ee", "#ffffffff"),
    color("blue", "#3b82f6ee", "#ffffffff"),
    color("purple", "#9b5cffee", "#ffffffff"),
    color("black", "#141428ee", "#ffffffff"),
    color("white", "#ffffffee", "#1a1a1aff"),
];

const fn color(name: &'static str, background: &'static str, text: &'static str) -> Color {
    Color { name, background, text }
}

/// The first color named as a word in `words`, which must be lowercase.
pub(crate) fn find_color(words: &str) -> Option<Color> {
    let named = |color: &Color| {
        words
            .split(|c: char| !c.is_alphanumeric())
            .any(|word| word == color.name)
    };
    COLORS.iter().copied().find(named)
}

/// The color after `current` in the palette, for a restyle that names no color.
pub(crate) fn next_color(current: &str) -> Color {
    let index = COLORS.iter().position(|color| color.background == current);
    COLORS[index.map_or(0, |index| (index + 1) % COLORS.len())]
}

/// Stream titles for what a lowercase description says the stream is about, checked in order.
const TITLES: [(&str, &str); 5] = [
    ("keyboard", "Keyboard reviews, live"),
    ("miniature", "Painting miniatures, live"),
    ("paint", "Painting, live"),
    ("game", "Cozy games, live"),
    ("cook", "Cooking, live"),
];

/// A title for the stream's name label, when the description says what the stream is about.
pub(crate) fn title_for(words: &str) -> Option<&'static str> {
    TITLES
        .iter()
        .find(|(keyword, _)| mentions(words, keyword))
        .map(|(_, title)| *title)
}

fn condition(id: &str, name: &str, prompt: String, detail: String) -> Condition {
    Condition {
        id: id.to_owned(),
        name: name.to_owned(),
        prompt,
        detail,
        enabled: true,
        min_confidence: 0.6,
        confirm: 1,
        release: 2,
    }
}

/// A hidden text card that a rule shows; `place` is x, y, width, and height on a 1920x1080 canvas.
fn card(id: &str, name: &str, text: &str, place: [f64; 4], background: &str, color: &str) -> Element {
    let [x, y, width, height] = place;
    Element {
        id: id.to_owned(),
        name: name.to_owned(),
        kind: ElementKind::Text,
        props: ElementProps {
            x,
            y,
            width,
            height,
            visible: false,
            text: text.to_owned(),
            font_size: 52.0,
            color: color.to_owned(),
            background: background.to_owned(),
            corner_radius: 18.0,
            enter: EnterAnimation::SlideUp,
            ..ElementProps::default()
        },
    }
}

pub(crate) fn holding(thing: &str) -> Feature {
    Feature {
        topic: Topic::Holding,
        condition: condition(
            "showing-product",
            "Showing a product",
            format!("The person holds a {thing} up toward the camera to show it to viewers."),
            format!("the {thing}'s name in one to four words"),
        ),
        element: card(
            "product-name",
            "Product name",
            "Now showing: {{@id}}",
            [60.0, 650.0, 900.0, 100.0],
            "#ff5c8aee",
            "#ffffffff",
        ),
        effect: None,
        show_rule: "Name what I hold up",
        hide_rule: "Hide the name when I put it down",
        summary: format!("name each {thing} you hold up in a lower third"),
    }
}

pub(crate) fn pet(animal: &str) -> Feature {
    let mut element = card(
        "pet-banner",
        "Pet banner",
        "{{@id}} on stream!",
        [1380.0, 200.0, 480.0, 110.0],
        "#ffc233ee",
        "#1a1a1aff",
    );
    element.props.enter = EnterAnimation::Pop;
    element.props.idle = IdleAnimation::Float;
    Feature {
        topic: Topic::Pet,
        condition: condition(
            "pet-on-camera",
            "Pet on camera",
            format!("A {animal} is clearly visible in the image."),
            "the kind of animal as one capitalized word, such as \"Cat\"".to_owned(),
        ),
        element,
        effect: Some(Effect::Bounce),
        show_rule: "Announce my pet",
        hide_rule: "Hide the pet banner",
        summary: format!("announce your {animal} when it shows up"),
    }
}

pub(crate) fn wave() -> Feature {
    Feature {
        topic: Topic::Wave,
        condition: condition(
            "waving",
            "Waving",
            "The person waves a hand at the camera.".to_owned(),
            String::new(),
        ),
        element: card(
            "wave-hello",
            "Wave hello",
            "Hi chat!",
            [60.0, 60.0, 560.0, 110.0],
            "#22c55eee",
            "#0b1f12ff",
        ),
        effect: Some(Effect::Bounce),
        show_rule: "Say hi when I wave",
        hide_rule: "Hide the hello banner",
        summary: "greet chat with a banner when you wave".to_owned(),
    }
}

pub(crate) fn celebrate() -> Feature {
    let mut element = card(
        "hype",
        "Hype",
        "LET'S GO!",
        [660.0, 200.0, 600.0, 160.0],
        "#9b5cffee",
        "#ffffffff",
    );
    element.props.font_size = 88.0;
    element.props.enter = EnterAnimation::Pop;
    Feature {
        topic: Topic::Celebrate,
        condition: condition(
            "arms-up",
            "Arms up",
            "The person raises both arms above the head to celebrate.".to_owned(),
            String::new(),
        ),
        element,
        effect: Some(Effect::Shake),
        show_rule: "Hype on arms up",
        hide_rule: "Hide the hype",
        summary: "shake a hype card when you throw your arms up".to_owned(),
    }
}
