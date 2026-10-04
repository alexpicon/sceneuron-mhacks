// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the perceiver interface helpers: the input size chosen for each camera frame
//! size, the RGB buffer check, and error messages.

use sceneuron_vision::{DetectError, InputSize, check_rgb};

const FIT: InputSize = InputSize::Fit {
    max_width: 640,
    max_height: 640,
};

#[test]
fn fixed_sizes_ignore_the_frame() {
    let fixed = InputSize::Fixed {
        width: 320,
        height: 240,
    };
    assert_eq!(fixed.for_frame(1920, 1080), (320, 240));
    assert_eq!(fixed.for_frame(0, 0), (320, 240));
}

#[test]
fn fit_keeps_the_aspect_ratio() {
    assert_eq!(FIT.for_frame(1920, 1080), (640, 360));
    assert_eq!(FIT.for_frame(1280, 720), (640, 360));
    assert_eq!(FIT.for_frame(1440, 1080), (640, 480));
    assert_eq!(FIT.for_frame(1080, 1920), (360, 640), "portrait cameras");
    assert_eq!(FIT.for_frame(2560, 1080), (640, 270));
}

#[test]
fn fit_never_scales_up() {
    assert_eq!(FIT.for_frame(320, 240), (320, 240));
    assert_eq!(FIT.for_frame(640, 640), (640, 640));
}

#[test]
fn fit_stays_inside_the_box_and_never_reaches_zero() {
    assert_eq!(FIT.for_frame(100_000, 1), (640, 1));
    assert_eq!(FIT.for_frame(1, 100_000), (1, 640));
    assert_eq!(FIT.for_frame(u32::MAX, u32::MAX), (640, 640));
    assert_eq!(FIT.for_frame(0, 1080), (640, 640));
    let empty_box = InputSize::Fit {
        max_width: 0,
        max_height: 0,
    };
    assert_eq!(empty_box.for_frame(1920, 1080), (1, 1));
}

#[test]
fn rgb_buffers_must_match_their_size() {
    assert_eq!(check_rgb(&[0; 12], 2, 2), Ok(()));
    for (len, width, height) in [(11, 2, 2), (13, 2, 2), (0, 0, 0), (0, 0, 5), (3, 1, 0)] {
        let error = check_rgb(&vec![0; len], width, height).unwrap_err();
        assert_eq!(
            error,
            DetectError::InputSize {
                expected_width: width,
                expected_height: height,
                width,
                height,
                len,
            }
        );
    }
    assert!(check_rgb(&[], u32::MAX, u32::MAX).is_err());
}

#[test]
fn errors_describe_the_failure() {
    let size = check_rgb(&[0; 5], 2, 2).unwrap_err().to_string();
    assert_eq!(size, "expected a 2x2 RGB frame but got 2x2 with 5 bytes");
    assert_eq!(
        DetectError::Load("no Neural Engine".into()).to_string(),
        "could not load the vision model: no Neural Engine"
    );
    assert_eq!(
        DetectError::Inference("request failed".into()).to_string(),
        "frame analysis failed: request failed"
    );
}
