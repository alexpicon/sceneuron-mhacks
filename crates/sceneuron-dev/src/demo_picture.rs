// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The simulated camera's picture: a drawn room, labeled SIMULATED, with a band that sweeps across
//! so each picture differs from the last, and a person in view while demo presence says so.

use sceneuron_ai::Frame;
use sceneuron_server::FaceArea;

pub(crate) const WIDTH: u32 = 640;
pub(crate) const HEIGHT: u32 = 360;
/// The drawn head, by its center and radius in pixels.
const HEAD: (i32, i32, i32) = (320, 150, 52);
/// How long the band takes to cross the picture.
const SWEEP_MS: u64 = 3_000;
/// Five-by-seven glyphs of S, I, M, U, L, A, T, E, D: one row each, the leftmost pixel highest.
const LABEL: [[u8; 7]; 9] = [
    [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
    [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
    [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
    [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
    [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
    [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
    [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
    [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
    [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
];
const LABEL_SCALE: i32 = 4;
const LABEL_COLOR: [u8; 3] = [255, 214, 102];
const SKIN: [u8; 3] = [224, 172, 140];
const SHIRT: [u8; 3] = [70, 92, 160];
const BAND: [u8; 3] = [64, 80, 104];

/// The picture `elapsed_ms` after the demo started, with a person in view when `present`.
pub(crate) fn draw(present: bool, elapsed_ms: u64) -> Frame {
    let band = ((elapsed_ms % SWEEP_MS) * u64::from(WIDTH) / SWEEP_MS) as i32;
    let mut rgb = Vec::with_capacity((WIDTH * HEIGHT * 3) as usize);
    for y in 0..HEIGHT as i32 {
        for x in 0..WIDTH as i32 {
            rgb.extend_from_slice(&pixel(x, y, band, present));
        }
    }
    Frame {
        rgb: rgb.into(),
        width: WIDTH,
        height: HEIGHT,
        captured_ms: elapsed_ms,
    }
}

/// The box around the drawn head, as on-device analysis would report it.
pub(crate) fn face() -> FaceArea {
    let (x, y, radius) = HEAD;
    let margin = radius + 6;
    let horizontal = |value: i32| value as f32 / WIDTH as f32;
    let vertical = |value: i32| value as f32 / HEIGHT as f32;
    FaceArea {
        x1: horizontal(x - margin),
        y1: vertical(y - margin - 8),
        x2: horizontal(x + margin),
        y2: vertical(y + margin + 8),
        score: 0.95,
    }
}

fn pixel(x: i32, y: i32, band: i32, present: bool) -> [u8; 3] {
    if in_label(x, y) {
        return LABEL_COLOR;
    }
    if present {
        let (head_x, head_y, radius) = HEAD;
        if inside_ellipse(x - head_x, y - head_y, radius, radius + 8) {
            return SKIN;
        }
        if inside_ellipse(x - head_x, y - (HEIGHT as i32 + 20), 140, 150) {
            return SHIRT;
        }
    }
    if (x - band).abs() < 8 {
        return BAND;
    }
    // A dark wall that brightens toward the top, with a floor below 290 px.
    let shade = if y > 290 { 22 } else { 46 - (y / 10) as u8 };
    [shade, shade + 6, shade + 16]
}

fn inside_ellipse(dx: i32, dy: i32, radius_x: i32, radius_y: i32) -> bool {
    let (dx, dy) = (i64::from(dx), i64::from(dy));
    let (rx, ry) = (i64::from(radius_x), i64::from(radius_y));
    dx * dx * ry * ry + dy * dy * rx * rx <= rx * rx * ry * ry
}

/// Whether `(x, y)` lies on the SIMULATED label in the top left corner.
fn in_label(x: i32, y: i32) -> bool {
    let (left, top) = (16, 16);
    let advance = 6 * LABEL_SCALE;
    let (column, row) = ((x - left) / LABEL_SCALE, (y - top) / LABEL_SCALE);
    if x < left || y < top || row >= 7 {
        return false;
    }
    let (glyph, bit) = ((x - left) / advance, column % 6);
    let Some(rows) = LABEL.get(glyph as usize) else {
        return false;
    };
    bit < 5 && rows[row as usize] & (0b10000 >> bit) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(frame: &Frame, x: u32, y: u32) -> [u8; 3] {
        let index = ((y * frame.width + x) * 3) as usize;
        [frame.rgb[index], frame.rgb[index + 1], frame.rgb[index + 2]]
    }

    #[test]
    fn the_picture_is_labeled_and_shows_the_person_only_when_present() {
        let present = draw(true, 0);
        assert_eq!(present.rgb.len(), (WIDTH * HEIGHT * 3) as usize);
        assert_ne!(at(&present, 16, 16), LABEL_COLOR, "the S starts with a gap");
        assert_eq!(at(&present, 20, 16), LABEL_COLOR);
        assert_eq!(at(&present, 320, 150), SKIN);
        assert_ne!(at(&draw(false, 0), 320, 150), SKIN);
    }

    #[test]
    fn the_band_moves_so_each_picture_differs() {
        assert_ne!(draw(false, 0).rgb, draw(false, 500).rgb);
        assert_eq!(draw(false, 0).rgb, draw(false, SWEEP_MS).rgb);
    }

    #[test]
    fn the_face_box_surrounds_the_head() {
        let face = face();
        let (x, y, _) = HEAD;
        let (center_x, center_y) = (x as f32 / WIDTH as f32, y as f32 / HEIGHT as f32);
        assert!(face.x1 < center_x && center_x < face.x2);
        assert!(face.y1 < center_y && center_y < face.y2);
        assert!(face.x1 > 0.0 && face.y1 > 0.0 && face.x2 < 1.0 && face.y2 < 1.0);
    }
}
