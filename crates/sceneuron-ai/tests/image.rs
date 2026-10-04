// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises camera frame handling: JPEG encoding with area-average downscaling that keeps the
//! aspect ratio, and the grayscale thumbnails used to detect a changed picture.

mod common;

use std::sync::Arc;

use common::{frame_from, gray_frame};
use sceneuron_ai::{AiError, Frame, THUMBNAIL_HEIGHT, THUMBNAIL_WIDTH, Thumbnail, encode_jpeg};

/// Decodes a JPEG into its size and RGB pixels.
fn decode(jpeg: &[u8]) -> (u16, u16, Vec<u8>) {
    let mut decoder = jpeg_decoder::Decoder::new(jpeg);
    let pixels = decoder.decode().unwrap();
    let info = decoder.info().unwrap();
    (info.width, info.height, pixels)
}

#[test]
fn small_frames_encode_at_their_own_size() {
    let frame = frame_from(64, 36, |x, y| [(x * 4) as u8, (y * 7) as u8, 128]);
    let jpeg = encode_jpeg(&frame, 768).unwrap();
    assert_eq!(&jpeg[..2], [0xFF, 0xD8]);
    assert_eq!(&jpeg[jpeg.len() - 2..], [0xFF, 0xD9]);
    let (width, height, pixels) = decode(&jpeg);
    assert_eq!((width, height), (64, 36));
    assert_eq!(pixels.len(), 64 * 36 * 3);
}

#[test]
fn large_frames_shrink_to_the_width_and_keep_the_aspect_ratio() {
    let landscape = gray_frame(1920, 1080, 90, 0);
    let (width, height, _) = decode(&encode_jpeg(&landscape, 768).unwrap());
    assert_eq!((width, height), (768, 432));
    let odd = gray_frame(1280, 720, 90, 0);
    let (width, height, _) = decode(&encode_jpeg(&odd, 1000).unwrap());
    assert_eq!((width, height), (1000, 563));
}

#[test]
fn portrait_frames_shrink_by_their_longer_side() {
    let portrait = gray_frame(1080, 1920, 90, 0);
    let (width, height, _) = decode(&encode_jpeg(&portrait, 768).unwrap());
    assert_eq!((width, height), (432, 768));
}

#[test]
fn downscaling_averages_areas_instead_of_picking_pixels() {
    // A one-pixel checkerboard: area averaging gives mid gray, pixel picking black or white.
    let checkerboard = frame_from(
        400,
        200,
        |x, y| if (x + y) % 2 == 0 { [0, 0, 0] } else { [255, 255, 255] },
    );
    let (width, height, pixels) = decode(&encode_jpeg(&checkerboard, 100).unwrap());
    assert_eq!((width, height), (100, 50));
    let center = ((25 * 100 + 50) * 3) as usize;
    for channel in &pixels[center..center + 3] {
        assert!((118..=138).contains(channel), "expected mid gray, got {channel}");
    }
    let color = frame_from(300, 300, |_, _| [200, 100, 50]);
    let (_, _, pixels) = decode(&encode_jpeg(&color, 30).unwrap());
    for (got, want) in pixels[..3].iter().zip([200u8, 100, 50]) {
        assert!(got.abs_diff(want) <= 8, "got {got}, want about {want}");
    }
}

#[test]
fn malformed_frames_are_refused() {
    let short = Frame {
        rgb: Arc::from(vec![0u8; 10]),
        width: 4,
        height: 4,
        captured_ms: 0,
    };
    assert!(matches!(encode_jpeg(&short, 768), Err(AiError::Invalid(_))));
    let empty = Frame {
        rgb: Arc::from(Vec::new()),
        width: 0,
        height: 0,
        captured_ms: 0,
    };
    assert!(matches!(encode_jpeg(&empty, 768), Err(AiError::Invalid(_))));
    assert_eq!(Thumbnail::of(&short), Thumbnail::uniform(0));
}

#[test]
fn identical_pictures_do_not_differ() {
    let a = Thumbnail::of(&gray_frame(640, 360, 100, 0));
    let b = Thumbnail::of(&gray_frame(640, 360, 100, 99));
    assert_eq!(a.difference(&b), 0.0);
    assert_eq!(a, Thumbnail::uniform(100));
}

#[test]
fn small_brightness_changes_are_ignored() {
    let a = Thumbnail::of(&gray_frame(320, 180, 100, 0));
    let slightly = Thumbnail::of(&gray_frame(320, 180, 120, 0));
    let clearly = Thumbnail::of(&gray_frame(320, 180, 121, 0));
    assert_eq!(a.difference(&slightly), 0.0);
    assert_eq!(a.difference(&clearly), 1.0);
}

#[test]
fn difference_is_the_fraction_of_changed_cells() {
    let width = THUMBNAIL_WIDTH as u32 * 10;
    let height = THUMBNAIL_HEIGHT as u32 * 10;
    let dark = Thumbnail::of(&gray_frame(width, height, 20, 0));
    // The left quarter turns white.
    let quarter = frame_from(width, height, |x, _| if x < width / 4 { [255; 3] } else { [20; 3] });
    assert_eq!(dark.difference(&Thumbnail::of(&quarter)), 0.25);
    let all = Thumbnail::of(&gray_frame(width, height, 255, 0));
    assert_eq!(dark.difference(&all), 1.0);
}

#[test]
fn thumbnails_work_for_frames_smaller_than_the_grid() {
    let tiny = gray_frame(4, 3, 200, 0);
    assert_eq!(Thumbnail::of(&tiny), Thumbnail::uniform(200));
}
