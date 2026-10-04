// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Views an `obs_source_frame` as a bounds-checked [`FrameRef`] for the vision crate.

use sceneuron_obs_sys as sys;
use sceneuron_vision::{FrameRef, PixelFormat};

/// Number of planes and the height divisor of each plane for a format.
fn plane_layout(format: PixelFormat) -> &'static [u32] {
    match format {
        PixelFormat::I420 => &[1, 2, 2],
        PixelFormat::Nv12 => &[1, 2],
        PixelFormat::I444 | PixelFormat::I422 => &[1, 1, 1],
        PixelFormat::Yvyu
        | PixelFormat::Yuy2
        | PixelFormat::Uyvy
        | PixelFormat::Rgba
        | PixelFormat::Bgra
        | PixelFormat::Bgrx
        | PixelFormat::Y800
        | PixelFormat::Bgr3 => &[1],
    }
}

/// Why a frame cannot be analyzed.
#[derive(Debug, PartialEq, Eq)]
pub enum FrameError {
    UnsupportedFormat(u32),
    MissingPlane(usize),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::UnsupportedFormat(code) => write!(f, "unsupported camera pixel format {code}"),
            FrameError::MissingPlane(index) => write!(f, "camera frame is missing plane {index}"),
        }
    }
}

/// Borrows the planes of an OBS frame.
///
/// # Safety
/// `frame` must describe valid planes: each used `data[i]` points to at least
/// `linesize[i] * plane_height` readable bytes for as long as the result lives.
pub unsafe fn frame_ref(frame: &sys::obs_source_frame) -> Result<FrameRef<'_>, FrameError> {
    let format = PixelFormat::from_obs(frame.format).ok_or(FrameError::UnsupportedFormat(frame.format))?;
    let mut planes: [&[u8]; 4] = [&[]; 4];
    let mut strides = [0usize; 4];
    for (index, divisor) in plane_layout(format).iter().enumerate() {
        let data = frame.data[index];
        if data.is_null() {
            return Err(FrameError::MissingPlane(index));
        }
        let rows = frame.height.div_ceil(*divisor) as usize;
        let stride = frame.linesize[index] as usize;
        // SAFETY: the caller guarantees `stride * rows` readable bytes at `data`.
        planes[index] = unsafe { std::slice::from_raw_parts(data, stride * rows) };
        strides[index] = stride;
    }
    Ok(FrameRef {
        format,
        width: frame.width,
        height: frame.height,
        planes,
        strides,
        full_range: frame.full_range,
    })
}
