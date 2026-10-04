// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises frame conversion: every pixel format round-trips a known RGB pattern, padded
//! strides and odd sizes work, and resizing averages or repeats pixels as expected.

mod support {
    pub mod frames;
}

use sceneuron_vision::{ConvertError, FrameRef, PixelFormat, to_rgb};
use support::frames::{ALL_FORMATS, Encoded, Image, encode};

/// Colors spanning the gamut, including primaries that stress 8-bit YUV rounding.
const PALETTE: [[u8; 3]; 8] = [
    [0, 0, 0],
    [255, 255, 255],
    [255, 0, 0],
    [0, 255, 0],
    [0, 0, 255],
    [128, 128, 128],
    [224, 172, 140],
    [40, 90, 200],
];

/// Largest per-channel error allowed after an 8-bit YUV round trip: rounding Y, U, and V
/// to bytes can each move a channel by up to half a step times its gain (at most 2.02).
const YUV_TOLERANCE: u8 = 2;

/// A pattern of constant 2x2 blocks, so chroma subsampling loses nothing.
fn blocks(width: usize, height: usize) -> Image {
    Image::from_fn(width, height, |x, y| PALETTE[(x / 2 + 3 * (y / 2)) % PALETTE.len()])
}

fn convert(encoded: &Encoded, out_w: u32, out_h: u32) -> Result<Vec<u8>, ConvertError> {
    let mut out = vec![0; out_w as usize * out_h as usize * 3];
    to_rgb(&encoded.frame(), out_w, out_h, &mut out).map(|()| out)
}

fn is_gray(format: PixelFormat) -> bool {
    format == PixelFormat::Y800
}

/// The color a format can represent: Y800 keeps only luma.
fn representable(color: [u8; 3], format: PixelFormat) -> [u8; 3] {
    if is_gray(format) {
        let luma = 0.299 * f64::from(color[0]) + 0.587 * f64::from(color[1]) + 0.114 * f64::from(color[2]);
        [luma.round() as u8; 3]
    } else {
        color
    }
}

fn assert_matches(image: &Image, out: &[u8], format: PixelFormat, full_range: bool) {
    let rgb = matches!(
        format,
        PixelFormat::Rgba | PixelFormat::Bgra | PixelFormat::Bgrx | PixelFormat::Bgr3
    );
    let tolerance = if rgb { 0 } else { YUV_TOLERANCE };
    for y in 0..image.height {
        for x in 0..image.width {
            let expected = representable(image.at(x, y), format);
            let offset = (y * image.width + x) * 3;
            let actual = &out[offset..offset + 3];
            let close = expected.iter().zip(actual).all(|(e, a)| e.abs_diff(*a) <= tolerance);
            assert!(
                close,
                "{format:?} full_range={full_range} pixel ({x}, {y}): expected {expected:?}, got {actual:?}"
            );
        }
    }
}

#[test]
fn obs_codes_map_to_formats() {
    let expected = [
        (1, PixelFormat::I420),
        (2, PixelFormat::Nv12),
        (3, PixelFormat::Yvyu),
        (4, PixelFormat::Yuy2),
        (5, PixelFormat::Uyvy),
        (6, PixelFormat::Rgba),
        (7, PixelFormat::Bgra),
        (8, PixelFormat::Bgrx),
        (9, PixelFormat::Y800),
        (10, PixelFormat::I444),
        (11, PixelFormat::Bgr3),
        (12, PixelFormat::I422),
        (13, PixelFormat::I420),
        (14, PixelFormat::I422),
        (15, PixelFormat::I444),
    ];
    for (code, format) in expected {
        assert_eq!(PixelFormat::from_obs(code), Some(format), "code {code}");
    }
    for code in [0, 16, 17, 20, 26, u32::MAX] {
        assert_eq!(PixelFormat::from_obs(code), None, "code {code}");
    }
}

#[test]
fn every_format_round_trips_at_native_size() {
    let image = blocks(16, 12);
    for format in ALL_FORMATS {
        for full_range in [false, true] {
            for pad in [0, 7, 64] {
                let encoded = encode(&image, format, full_range, pad);
                let out = convert(&encoded, 16, 12).unwrap();
                assert_matches(&image, &out, format, full_range);
            }
        }
    }
}

#[test]
fn odd_sizes_round_trip() {
    for (width, height) in [(7, 5), (1, 1), (3, 8), (8, 3)] {
        let image = blocks(width, height);
        for format in ALL_FORMATS {
            let encoded = encode(&image, format, false, 3);
            let out = convert(&encoded, width as u32, height as u32).unwrap();
            assert_matches(&image, &out, format, false);
        }
    }
}

#[test]
fn downscaling_preserves_uniform_blocks() {
    let source = Image::from_fn(64, 48, |x, y| PALETTE[(x / 8 + 3 * (y / 8)) % PALETTE.len()]);
    let expected = Image::from_fn(16, 12, |x, y| PALETTE[(x / 2 + 3 * (y / 2)) % PALETTE.len()]);
    for format in ALL_FORMATS {
        let out = convert(&encode(&source, format, false, 5), 16, 12).unwrap();
        assert_matches(&expected, &out, format, false);
    }
}

#[test]
fn downscaling_averages_the_covered_area() {
    let stripes = Image::from_fn(8, 4, |x, _| if x % 2 == 0 { [0, 0, 0] } else { [255, 255, 255] });
    let out = convert(&encode(&stripes, PixelFormat::Rgba, true, 0), 4, 2).unwrap();
    assert!(out.iter().all(|&value| value == 128), "{out:?}");
}

#[test]
fn upscaling_repeats_the_nearest_pixel() {
    let source = Image::from_fn(2, 2, |x, y| PALETTE[2 + x + 2 * y]);
    let expected = Image::from_fn(6, 4, |x, y| source.at(x / 3, y / 2));
    let out = convert(&encode(&source, PixelFormat::Bgr3, true, 1), 6, 4).unwrap();
    assert_matches(&expected, &out, PixelFormat::Bgr3, true);
}

/// Converts a single Y800 pixel and returns its red channel.
fn gray(level: u8, full_range: bool) -> u8 {
    let plane = [level];
    let frame = FrameRef {
        format: PixelFormat::Y800,
        width: 1,
        height: 1,
        planes: [&plane, &[], &[], &[]],
        strides: [1, 0, 0, 0],
        full_range,
    };
    let mut out = [0; 3];
    to_rgb(&frame, 1, 1, &mut out).unwrap();
    out[0]
}

#[test]
fn range_flag_selects_luma_scaling() {
    for (level, limited, full) in [
        (16, 0, 16),
        (235, 255, 235),
        (126, 128, 126),
        (0, 0, 0),
        (255, 255, 255),
    ] {
        assert_eq!(gray(level, false), limited, "limited {level}");
        assert_eq!(gray(level, true), full, "full {level}");
    }
}
