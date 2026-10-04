// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Plane geometry per pixel format and the bounds checks that make sampling safe.

use crate::frame::{ConvertError, FrameRef, PixelFormat};

/// One validated plane: every pixel the format addresses lies inside `data`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Plane<'a> {
    data: &'a [u8],
    stride: usize,
}

impl Plane<'_> {
    /// The byte at column `x` (in bytes) of row `y`.
    #[inline]
    pub(crate) fn at(&self, x: usize, y: usize) -> u8 {
        self.data[y * self.stride + x]
    }
}

/// Bytes per row and number of rows of one plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Geometry {
    row_bytes: usize,
    rows: usize,
}

/// The planes `format` uses at `width` x `height`, in plane order.
///
/// Packed 4:2:2 rows are rounded up to whole 4-byte macropixels, so an odd width needs
/// the final macropixel to be present.
fn geometry(format: PixelFormat, width: usize, height: usize) -> Option<Vec<Geometry>> {
    let half_w = width.div_ceil(2);
    let half_h = height.div_ceil(2);
    let plane = |row_bytes: Option<usize>, rows: usize| row_bytes.map(|row_bytes| Geometry { row_bytes, rows });
    let planes = match format {
        PixelFormat::I420 => vec![
            plane(Some(width), height),
            plane(Some(half_w), half_h),
            plane(Some(half_w), half_h),
        ],
        PixelFormat::Nv12 => vec![plane(Some(width), height), plane(half_w.checked_mul(2), half_h)],
        PixelFormat::I422 => vec![
            plane(Some(width), height),
            plane(Some(half_w), height),
            plane(Some(half_w), height),
        ],
        PixelFormat::I444 => vec![plane(Some(width), height); 3],
        PixelFormat::Yvyu | PixelFormat::Yuy2 | PixelFormat::Uyvy => vec![plane(half_w.checked_mul(4), height)],
        PixelFormat::Rgba | PixelFormat::Bgra | PixelFormat::Bgrx => vec![plane(width.checked_mul(4), height)],
        PixelFormat::Bgr3 => vec![plane(width.checked_mul(3), height)],
        PixelFormat::Y800 => vec![plane(Some(width), height)],
    };
    planes.into_iter().collect()
}

/// Checks every plane the frame's format uses and returns them in plane order.
pub(crate) fn validate<'a>(frame: &FrameRef<'a>) -> Result<Vec<Plane<'a>>, ConvertError> {
    if frame.width == 0 || frame.height == 0 {
        return Err(ConvertError::EmptyFrame {
            width: frame.width,
            height: frame.height,
        });
    }
    let width = usize::try_from(frame.width).map_err(|_| ConvertError::TooLarge)?;
    let height = usize::try_from(frame.height).map_err(|_| ConvertError::TooLarge)?;
    let planes = geometry(frame.format, width, height).ok_or(ConvertError::TooLarge)?;
    planes
        .into_iter()
        .enumerate()
        .map(|(index, geometry)| check(index, geometry, frame.planes[index], frame.strides[index]))
        .collect()
}

fn check(plane: usize, geometry: Geometry, data: &[u8], stride: usize) -> Result<Plane<'_>, ConvertError> {
    if stride < geometry.row_bytes {
        return Err(ConvertError::StrideTooSmall {
            plane,
            stride,
            row_bytes: geometry.row_bytes,
        });
    }
    let required = stride
        .checked_mul(geometry.rows - 1)
        .and_then(|before_last| before_last.checked_add(geometry.row_bytes))
        .ok_or(ConvertError::TooLarge)?;
    if data.len() < required {
        return Err(ConvertError::PlaneTooShort {
            plane,
            len: data.len(),
            required,
        });
    }
    Ok(Plane { data, stride })
}
