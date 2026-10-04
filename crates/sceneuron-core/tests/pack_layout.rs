// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises fitting packs to the project's canvas: plain scaling on 16:9 canvases, slots and
//! subtitles bars that stay inside and apart on wide, tall, and 4:3 canvases, rounding, and
//! clamping on tiny ones.

use sceneuron_core::{Canvas, ElementProps, pack_for_canvas, packs};

const SHAPES: [(u32, u32); 6] = [
    (1920, 1080),
    (1280, 720),
    (2560, 1080),
    (3440, 1440),
    (1024, 768),
    (1080, 1920),
];

/// Every element of every pack on `canvas`, by element id: each card pack's slot, in pack
/// order, then the subtitles bars.
fn slots(width: u32, height: u32) -> Vec<(String, ElementProps)> {
    let canvas = Canvas { width, height };
    packs()
        .into_iter()
        .flat_map(|pack| pack_for_canvas(&pack.id, &canvas).unwrap().elements)
        .map(|element| (element.id, element.props))
        .collect()
}

fn overlap(a: &ElementProps, b: &ElementProps) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

#[test]
fn packs_scale_plainly_on_16_9_canvases_and_round_to_whole_pixels() {
    let canvas = Canvas {
        width: 1280,
        height: 720,
    };
    let meeting = pack_for_canvas("meeting", &canvas).unwrap();
    let slot = &meeting.elements[0].props;
    assert_eq!((slot.x, slot.y, slot.width, slot.height), (253.0, 560.0, 773.0, 120.0));
    assert_eq!((slot.font_size, slot.corner_radius), (48.0, 24.0));
    let poker = pack_for_canvas("poker", &canvas).unwrap();
    let card = &poker.elements[0].props;
    assert_eq!(
        (card.font_size, card.border_width, card.corner_radius),
        (87.0, 5.0, 19.0)
    );
    let classroom = pack_for_canvas("classroom", &canvas).unwrap();
    let answer = classroom.decks[0].card("answer-a").unwrap();
    assert_eq!(answer.props.font_size, Some(100.0), "card lengths scale too");
    assert_eq!(classroom.decks[0].card("hand").unwrap().props.font_size, None);
}

#[test]
fn the_authored_canvas_keeps_packs_unchanged_and_tiny_canvases_clamp() {
    for pack in packs() {
        assert_eq!(pack_for_canvas(&pack.id, &Canvas::default()), Some(pack));
    }
    let tiny = pack_for_canvas("status", &Canvas { width: 16, height: 16 }).unwrap();
    let slot = &tiny.elements[0].props;
    assert!(slot.width >= 1.0 && slot.height >= 1.0 && slot.font_size >= 1.0);
    assert_eq!(pack_for_canvas("karaoke", &Canvas::default()), None);
}

#[test]
fn slots_stay_inside_every_canvas_and_apart() {
    for (width, height) in SHAPES {
        let slots = slots(width, height);
        for (id, slot) in &slots {
            let inside = slot.x >= 0.0
                && slot.y >= 0.0
                && slot.x + slot.width <= f64::from(width)
                && slot.y + slot.height <= f64::from(height);
            assert!(inside, "{id} on {width}x{height}: {slot:?}");
        }
        for (n, (id, slot)) in slots.iter().enumerate() {
            for (other, other_slot) in &slots[n + 1..] {
                assert!(!overlap(slot, other_slot), "{id} and {other} on {width}x{height}");
            }
        }
    }
}

#[test]
fn wide_canvases_size_slots_by_height_and_keep_them_centered_where_authored() {
    let wide = slots(2560, 1080);
    let (_, meeting) = &wide[0];
    assert_eq!(
        (meeting.width, meeting.height, meeting.font_size),
        (1160.0, 180.0, 72.0)
    );
    assert_eq!((meeting.x, meeting.y), (700.0, 840.0), "centered like on 1920x1080");
    let (_, poker) = &wide[1];
    assert_eq!(
        (poker.x, poker.y, poker.width, poker.height),
        (2130.0, 600.0, 300.0, 420.0)
    );
    let ultrawide = slots(3440, 1440);
    let (_, meeting) = &ultrawide[0];
    assert_eq!(
        (meeting.width, meeting.height, meeting.font_size),
        (1547.0, 240.0, 96.0)
    );
    assert_eq!((meeting.x, meeting.y), (947.0, 1120.0));
}

#[test]
fn every_length_of_a_card_scales_by_the_smaller_axis() {
    let wide = Canvas {
        width: 3440,
        height: 1440,
    };
    let classroom = pack_for_canvas("classroom", &wide).unwrap();
    let answer = classroom.decks[0].card("answer-a").unwrap();
    assert_eq!(answer.props.font_size, Some(200.0), "150 times 1440 over 1080");
    let tall = Canvas {
        width: 1080,
        height: 1920,
    };
    let classroom = pack_for_canvas("classroom", &tall).unwrap();
    let answer = classroom.decks[0].card("answer-a").unwrap();
    assert_eq!(answer.props.font_size, Some(84.0), "150 times 1080 over 1920");
}
