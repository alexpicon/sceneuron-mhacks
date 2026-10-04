// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the packs' colors: every card, as its slot shows it, and every bar without a deck, for
//! subtitles or the live description, has a fully opaque background and full opacity, so it covers whatever is under it, and solid text
//! with a contrast of at least 4.5 to 1 against that background.

use sceneuron_core::{ElementProps, packs};

/// WCAG 2's minimum contrast ratio for text.
const MIN_CONTRAST: f64 = 4.5;

/// A `#rrggbbaa` color as red, green, blue, and alpha bytes.
fn rgba(color: &str) -> [u8; 4] {
    let hex = color.strip_prefix('#').filter(|hex| hex.len() == 8);
    let hex = hex.unwrap_or_else(|| panic!("{color} is not #rrggbbaa"));
    std::array::from_fn(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
}

/// WCAG 2's relative luminance of an sRGB color; alpha is ignored.
fn luminance([red, green, blue, _]: [u8; 4]) -> f64 {
    let linear = |channel: u8| {
        let channel = f64::from(channel) / 255.0;
        if channel <= 0.040_45 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue)
}

fn contrast(one: &str, other: &str) -> f64 {
    let (one, other) = (luminance(rgba(one)), luminance(rgba(other)));
    (one.max(other) + 0.05) / (one.min(other) + 0.05)
}

#[test]
fn the_contrast_measure_matches_known_pairs() {
    assert!((contrast("#000000ff", "#ffffffff") - 21.0).abs() < 1e-9);
    assert!((contrast("#ffffffff", "#000000ff") - 21.0).abs() < 1e-9);
    assert!((contrast("#2563ebff", "#2563ebff") - 1.0).abs() < 1e-9);
    // The grays just above and below 4.5 to 1 on white.
    assert!(contrast("#767676ff", "#ffffffff") >= MIN_CONTRAST);
    assert!(contrast("#777777ff", "#ffffffff") < MIN_CONTRAST);
}

/// Checks that `shown` draws solid text, readable against an opaque background, at full opacity.
fn assert_readable(name: &str, shown: &ElementProps) {
    let (background, color) = (&shown.background, &shown.color);
    assert_eq!(rgba(background)[3], 0xff, "{name}'s background {background} is opaque");
    assert_eq!(rgba(color)[3], 0xff, "{name}'s text color {color} is solid");
    assert_eq!(shown.opacity, 1.0, "{name} is shown at full opacity");
    let ratio = contrast(color, background);
    assert!(
        ratio >= MIN_CONTRAST,
        "{name}: {color} on {background} is {ratio:.2} to 1"
    );
}

#[test]
fn every_card_shows_solid_readable_text_on_an_opaque_background() {
    let mut cards = 0;
    for pack in packs() {
        for deck in &pack.decks {
            let slot = pack.elements.iter().find(|element| element.id == deck.element).unwrap();
            for card in &deck.cards {
                let mut shown = slot.props.clone();
                shown.apply(&card.props);
                assert_readable(&format!("{}.{}", pack.id, card.id), &shown);
                cards += 1;
            }
        }
    }
    assert_eq!(cards, 33, "every card of every pack is checked");
}

#[test]
fn every_bar_without_a_deck_shows_solid_readable_text_on_an_opaque_background() {
    let bars: Vec<_> = packs()
        .into_iter()
        .filter(|pack| pack.decks.is_empty())
        .flat_map(|pack| pack.elements)
        .collect();
    assert_eq!(
        bars.len(),
        3,
        "the spoken and the translated subtitles, and the live description"
    );
    for bar in &bars {
        assert_readable(&bar.id, &bar.props);
    }
}

#[test]
fn every_slot_is_opaque_for_cards_that_keep_its_background() {
    for pack in packs() {
        for element in &pack.elements {
            let props = &element.props;
            assert_eq!(rgba(&props.background)[3], 0xff, "{}: {}", element.id, props.background);
            assert_eq!(props.opacity, 1.0, "{} is at full opacity", element.id);
        }
    }
}
