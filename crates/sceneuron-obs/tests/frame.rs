// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the mapping from OBS async video frames to bounds-checked plane views.

use sceneuron::frame::{FrameError, frame_ref};
use sceneuron_obs_sys as sys;
use sceneuron_vision::PixelFormat;

/// An OBS frame whose planes point into owned buffers.
struct TestFrame {
    frame: sys::obs_source_frame,
    _planes: Vec<Vec<u8>>,
}

fn test_frame(format: u32, width: u32, height: u32, planes: &[(u32, u32)]) -> TestFrame {
    // SAFETY: obs_source_frame is plain data; all-zero is a valid empty frame.
    let mut frame: sys::obs_source_frame = unsafe { std::mem::zeroed() };
    frame.format = format;
    frame.width = width;
    frame.height = height;
    let mut buffers = Vec::new();
    for (index, (stride, rows)) in planes.iter().enumerate() {
        let mut buffer = vec![index as u8; (*stride * *rows) as usize];
        frame.data[index] = buffer.as_mut_ptr();
        frame.linesize[index] = *stride;
        buffers.push(buffer);
    }
    TestFrame {
        frame,
        _planes: buffers,
    }
}

#[test]
fn nv12_has_luma_and_half_height_chroma() {
    let test = test_frame(sys::VIDEO_FORMAT_NV12, 640, 480, &[(640, 480), (640, 240)]);
    // SAFETY: the planes hold exactly stride * rows bytes.
    let view = unsafe { frame_ref(&test.frame) }.unwrap();
    assert_eq!(view.format, PixelFormat::Nv12);
    assert_eq!((view.planes[0].len(), view.planes[1].len()), (640 * 480, 640 * 240));
    assert!(view.planes[2].is_empty());
}

#[test]
fn i420_rounds_odd_chroma_heights_up() {
    let test = test_frame(sys::VIDEO_FORMAT_I420, 5, 5, &[(8, 5), (4, 3), (4, 3)]);
    // SAFETY: the planes hold exactly stride * rows bytes.
    let view = unsafe { frame_ref(&test.frame) }.unwrap();
    assert_eq!(view.planes[1].len(), 12);
    assert_eq!(view.strides[..3], [8, 4, 4]);
}

#[test]
fn packed_formats_use_one_plane_with_padding() {
    let test = test_frame(sys::VIDEO_FORMAT_BGRA, 3, 2, &[(16, 2)]);
    // SAFETY: the plane holds exactly stride * rows bytes.
    let view = unsafe { frame_ref(&test.frame) }.unwrap();
    assert_eq!((view.planes[0].len(), view.strides[0]), (32, 16));
}

#[test]
fn rejects_unsupported_formats_and_missing_planes() {
    let unsupported = test_frame(sys::VIDEO_FORMAT_P010, 2, 2, &[(4, 2)]);
    // SAFETY: valid buffers; the format check fails before any plane is read.
    let result = unsafe { frame_ref(&unsupported.frame) }.err();
    assert_eq!(result, Some(FrameError::UnsupportedFormat(sys::VIDEO_FORMAT_P010)));
    let missing = test_frame(sys::VIDEO_FORMAT_NV12, 2, 2, &[(2, 2)]);
    // SAFETY: plane 0 is valid and the missing plane 1 is null.
    let result = unsafe { frame_ref(&missing.frame) }.err();
    assert_eq!(result, Some(FrameError::MissingPlane(1)));
}
