// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Raw camera frames: pixel formats, borrowed planes, and conversion errors.

use std::fmt;

/// Pixel layouts the converter understands, named after OBS `video_format` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    /// Planar 4:2:0: Y, then U and V at half width and half height.
    I420,
    /// Y plane, then one interleaved U V plane at half width and half height.
    Nv12,
    /// Packed 4:2:2 as `Y0 V Y1 U`.
    Yvyu,
    /// Packed 4:2:2 as `Y0 U Y1 V`.
    Yuy2,
    /// Packed 4:2:2 as `U Y0 V Y1`.
    Uyvy,
    /// Packed `R G B A`, one byte each.
    Rgba,
    /// Packed `B G R A`, one byte each.
    Bgra,
    /// Packed `B G R X`, one byte each; the fourth byte is ignored.
    Bgrx,
    /// A single 8-bit luma plane.
    Y800,
    /// Planar 4:4:4: Y, U, and V at full resolution.
    I444,
    /// Packed `B G R`, one byte each.
    Bgr3,
    /// Planar 4:2:2: Y, then U and V at half width and full height.
    I422,
}

impl PixelFormat {
    /// Maps an OBS `video_format` code to a supported format.
    ///
    /// The alpha variants (`I40A`, `I42A`, `YUVA`) map to their opaque layouts; the
    /// alpha plane is ignored. Codes for unsupported formats return `None`.
    pub fn from_obs(code: u32) -> Option<PixelFormat> {
        let format = match code {
            1 | 13 => PixelFormat::I420,
            2 => PixelFormat::Nv12,
            3 => PixelFormat::Yvyu,
            4 => PixelFormat::Yuy2,
            5 => PixelFormat::Uyvy,
            6 => PixelFormat::Rgba,
            7 => PixelFormat::Bgra,
            8 => PixelFormat::Bgrx,
            9 => PixelFormat::Y800,
            10 | 15 => PixelFormat::I444,
            11 => PixelFormat::Bgr3,
            12 | 14 => PixelFormat::I422,
            _ => return None,
        };
        Some(format)
    }
}

/// A borrowed view of one frame.
///
/// `planes[i]` holds rows of `strides[i]` bytes each. Only the planes the format uses
/// are read; the last row of a plane may be shorter than its stride.
#[derive(Clone, Copy)]
pub struct FrameRef<'a> {
    pub format: PixelFormat,
    pub width: u32,
    pub height: u32,
    pub planes: [&'a [u8]; 4],
    pub strides: [usize; 4],
    /// True for full-range (0-255) YUV, false for limited range (16-235 luma).
    pub full_range: bool,
}

impl fmt::Debug for FrameRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrameRef")
            .field("format", &self.format)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("plane_lens", &self.planes.map(<[u8]>::len))
            .field("strides", &self.strides)
            .field("full_range", &self.full_range)
            .finish()
    }
}

/// Why a frame could not be converted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConvertError {
    /// The source frame has a zero dimension.
    EmptyFrame { width: u32, height: u32 },
    /// The requested output has a zero dimension.
    EmptyOutput { width: u32, height: u32 },
    /// The output buffer is not exactly `out_w * out_h * 3` bytes.
    OutputLength { expected: usize, actual: usize },
    /// A plane's stride is shorter than one row of pixels.
    StrideTooSmall {
        plane: usize,
        stride: usize,
        row_bytes: usize,
    },
    /// A plane holds fewer bytes than its rows and stride require.
    PlaneTooShort { plane: usize, len: usize, required: usize },
    /// The frame or output size does not fit in memory arithmetic.
    TooLarge,
}

impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConvertError::EmptyFrame { width, height } => write!(f, "frame size {width}x{height} is empty"),
            ConvertError::EmptyOutput { width, height } => write!(f, "output size {width}x{height} is empty"),
            ConvertError::OutputLength { expected, actual } => {
                write!(f, "output buffer holds {actual} bytes but {expected} are needed")
            }
            ConvertError::StrideTooSmall {
                plane,
                stride,
                row_bytes,
            } => {
                write!(
                    f,
                    "plane {plane} stride {stride} is shorter than a {row_bytes}-byte row"
                )
            }
            ConvertError::PlaneTooShort { plane, len, required } => {
                write!(f, "plane {plane} holds {len} bytes but {required} are needed")
            }
            ConvertError::TooLarge => f.write_str("frame dimensions are too large"),
        }
    }
}

impl std::error::Error for ConvertError {}
