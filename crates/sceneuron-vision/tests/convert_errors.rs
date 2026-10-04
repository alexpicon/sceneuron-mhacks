// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises frame conversion errors: short planes, small strides, and bad sizes return
//! descriptive errors instead of panicking or reading out of bounds.

mod support {
    pub mod frames;
}

use sceneuron_vision::{ConvertError, PixelFormat, to_rgb};
use support::frames::{ALL_FORMATS, Encoded, Image, encode};

fn blocks(width: usize, height: usize) -> Image {
    Image::from_fn(width, height, |x, y| [(x * 40) as u8, (y * 30) as u8, 128])
}

fn convert(encoded: &Encoded, out_w: u32, out_h: u32) -> Result<Vec<u8>, ConvertError> {
    let mut out = vec![0; out_w as usize * out_h as usize * 3];
    to_rgb(&encoded.frame(), out_w, out_h, &mut out).map(|()| out)
}

#[test]
fn short_planes_are_rejected() {
    let image = blocks(9, 7);
    for format in ALL_FORMATS {
        let encoded = encode(&image, format, false, 4);
        for plane in 0..encoded.planes.len() {
            let mut short = encoded.clone();
            short.planes[plane].pop();
            let error = convert(&short, 4, 4).unwrap_err();
            assert!(
                matches!(error, ConvertError::PlaneTooShort { plane: p, .. } if p == plane),
                "{format:?} plane {plane}: {error}"
            );
            short.planes[plane].clear();
            assert!(convert(&short, 4, 4).is_err(), "{format:?} plane {plane} empty");
        }
    }
}

#[test]
fn small_strides_are_rejected() {
    let image = blocks(9, 7);
    for format in ALL_FORMATS {
        let mut encoded = encode(&image, format, false, 0);
        let last = encoded.strides.len() - 1;
        encoded.strides[last] -= 1;
        let error = convert(&encoded, 4, 4).unwrap_err();
        assert!(
            matches!(error, ConvertError::StrideTooSmall { plane, .. } if plane == last),
            "{format:?}: {error}"
        );
    }
}

#[test]
fn bad_dimensions_are_rejected() {
    let encoded = encode(&blocks(4, 4), PixelFormat::Rgba, false, 0);
    let mut frame = encoded.frame();
    let mut out = vec![0; 2 * 2 * 3];
    assert!(matches!(
        to_rgb(&frame, 0, 2, &mut out),
        Err(ConvertError::EmptyOutput { .. })
    ));
    assert!(matches!(
        to_rgb(&frame, 2, 3, &mut out),
        Err(ConvertError::OutputLength { .. })
    ));
    assert!(matches!(
        to_rgb(&frame, 2, 2, &mut out[1..]),
        Err(ConvertError::OutputLength { .. })
    ));
    frame.width = 0;
    assert!(matches!(
        to_rgb(&frame, 2, 2, &mut out),
        Err(ConvertError::EmptyFrame { .. })
    ));
    frame.width = u32::MAX;
    frame.height = u32::MAX;
    assert!(to_rgb(&frame, 2, 2, &mut out).is_err());
    frame.strides[0] = usize::MAX;
    assert!(matches!(to_rgb(&frame, 2, 2, &mut out), Err(ConvertError::TooLarge)));
}

#[test]
fn errors_describe_the_problem() {
    let error = ConvertError::PlaneTooShort {
        plane: 1,
        len: 10,
        required: 12,
    };
    assert_eq!(error.to_string(), "plane 1 holds 10 bytes but 12 are needed");
}
