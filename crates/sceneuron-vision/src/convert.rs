// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Converts any supported frame to packed RGB24 while stretching it to a target size.
//!
//! Each output pixel averages a grid of source samples spread evenly over the source
//! area it covers (a box filter), so downscaling does not alias. The grid is capped at
//! [`MAX_TAPS`] samples per axis to bound the cost for large frames; when upscaling the
//! grid is a single sample, which is nearest-neighbor sampling. Channels are averaged
//! before the YUV to RGB conversion, which is affine and so commutes with averaging
//! up to clamping.

use crate::color::Encoding;
use crate::frame::{ConvertError, FrameRef, PixelFormat};
use crate::layout::{Plane, validate};

/// Most samples taken per output pixel along each axis.
const MAX_TAPS: u32 = 4;

/// Converts and resizes (stretch, no letterbox) to packed RGB24 `out_w` x `out_h`.
///
/// `out` must be exactly `out_w * out_h * 3` bytes. Plane sizes are validated against
/// the strides and plane heights first, so malformed frames return an error rather than
/// reading out of bounds.
pub fn to_rgb(frame: &FrameRef, out_w: u32, out_h: u32, out: &mut [u8]) -> Result<(), ConvertError> {
    if out_w == 0 || out_h == 0 {
        return Err(ConvertError::EmptyOutput {
            width: out_w,
            height: out_h,
        });
    }
    let expected = (out_w as usize)
        .checked_mul(out_h as usize)
        .and_then(|pixels| pixels.checked_mul(3))
        .ok_or(ConvertError::TooLarge)?;
    if out.len() != expected {
        return Err(ConvertError::OutputLength {
            expected,
            actual: out.len(),
        });
    }
    let planes = validate(frame)?;
    let grid = Grid {
        columns: Axis::new(frame.width, out_w),
        rows: Axis::new(frame.height, out_h),
    };
    let yuv = Encoding::Yuv {
        full_range: frame.full_range,
    };
    let plane = |index: usize| planes[index];
    match frame.format {
        PixelFormat::I420 => grid.fill(&Planar::new(&planes, 1, 1), yuv, out),
        PixelFormat::I422 => grid.fill(&Planar::new(&planes, 1, 0), yuv, out),
        PixelFormat::I444 => grid.fill(&Planar::new(&planes, 0, 0), yuv, out),
        PixelFormat::Nv12 => grid.fill(&SemiPlanar(plane(0), plane(1)), yuv, out),
        PixelFormat::Yuy2 => grid.fill(&Packed422::new(plane(0), [0, 2], 1, 3), yuv, out),
        PixelFormat::Yvyu => grid.fill(&Packed422::new(plane(0), [0, 2], 3, 1), yuv, out),
        PixelFormat::Uyvy => grid.fill(&Packed422::new(plane(0), [1, 3], 0, 2), yuv, out),
        PixelFormat::Y800 => grid.fill(&Luma(plane(0)), yuv, out),
        PixelFormat::Rgba => grid.fill(&PackedRgb::new(plane(0), 4, [0, 1, 2]), Encoding::Rgb, out),
        PixelFormat::Bgra | PixelFormat::Bgrx => grid.fill(&PackedRgb::new(plane(0), 4, [2, 1, 0]), Encoding::Rgb, out),
        PixelFormat::Bgr3 => grid.fill(&PackedRgb::new(plane(0), 3, [2, 1, 0]), Encoding::Rgb, out),
    }
    Ok(())
}

/// Source sample positions along one axis: `taps` positions per output index.
struct Axis {
    taps: usize,
    positions: Vec<usize>,
}

impl Axis {
    fn new(source: u32, output: u32) -> Axis {
        let taps = source.div_ceil(output).clamp(1, MAX_TAPS);
        let last = u128::from(source - 1);
        let denominator = 2 * u128::from(taps) * u128::from(output);
        let positions = (0..u128::from(output))
            .flat_map(|index| (0..u128::from(taps)).map(move |tap| (index, tap)))
            .map(|(index, tap)| {
                // Sample centers sit at (index + (2 * tap + 1) / (2 * taps)) * source / output.
                let numerator = (2 * index * u128::from(taps) + 2 * tap + 1) * u128::from(source);
                (numerator / denominator).min(last) as usize
            })
            .collect();
        Axis {
            taps: taps as usize,
            positions,
        }
    }

    fn at(&self, index: usize) -> &[usize] {
        &self.positions[index * self.taps..(index + 1) * self.taps]
    }
}

struct Grid {
    columns: Axis,
    rows: Axis,
}

impl Grid {
    fn fill<S: Sampler>(&self, source: &S, encoding: Encoding, out: &mut [u8]) {
        let count = (self.columns.taps * self.rows.taps) as f32;
        let out_w = self.columns.positions.len() / self.columns.taps;
        let (pixels, _) = out.as_chunks_mut::<3>();
        for (row_index, row) in pixels.chunks_exact_mut(out_w).enumerate() {
            let ys = self.rows.at(row_index);
            for (column_index, pixel) in row.iter_mut().enumerate() {
                let xs = self.columns.at(column_index);
                let mut sum = [0u32; 3];
                for &y in ys {
                    for &x in xs {
                        let [a, b, c] = source.sample(x, y);
                        sum[0] += u32::from(a);
                        sum[1] += u32::from(b);
                        sum[2] += u32::from(c);
                    }
                }
                *pixel = encoding.to_rgb(sum.map(|total| total as f32 / count));
            }
        }
    }
}

/// Reads the three channels of the pixel at `(x, y)`, which must lie inside the frame.
trait Sampler {
    fn sample(&self, x: usize, y: usize) -> [u8; 3];
}

/// Three separate Y, U, V planes; chroma coordinates are shifted right by the subsampling.
struct Planar<'a> {
    y: Plane<'a>,
    u: Plane<'a>,
    v: Plane<'a>,
    x_shift: u32,
    y_shift: u32,
}

impl<'a> Planar<'a> {
    fn new(planes: &[Plane<'a>], x_shift: u32, y_shift: u32) -> Planar<'a> {
        Planar {
            y: planes[0],
            u: planes[1],
            v: planes[2],
            x_shift,
            y_shift,
        }
    }
}

impl Sampler for Planar<'_> {
    #[inline]
    fn sample(&self, x: usize, y: usize) -> [u8; 3] {
        let (cx, cy) = (x >> self.x_shift, y >> self.y_shift);
        [self.y.at(x, y), self.u.at(cx, cy), self.v.at(cx, cy)]
    }
}

/// A luma plane plus one interleaved `U V` plane at 4:2:0.
struct SemiPlanar<'a>(Plane<'a>, Plane<'a>);

impl Sampler for SemiPlanar<'_> {
    #[inline]
    fn sample(&self, x: usize, y: usize) -> [u8; 3] {
        let (cx, cy) = (x & !1, y >> 1);
        [self.0.at(x, y), self.1.at(cx, cy), self.1.at(cx + 1, cy)]
    }
}

/// Packed 4:2:2 with 4-byte macropixels holding two luma samples and one U V pair.
struct Packed422<'a> {
    plane: Plane<'a>,
    luma: [usize; 2],
    u: usize,
    v: usize,
}

impl<'a> Packed422<'a> {
    fn new(plane: Plane<'a>, luma: [usize; 2], u: usize, v: usize) -> Packed422<'a> {
        Packed422 { plane, luma, u, v }
    }
}

impl Sampler for Packed422<'_> {
    #[inline]
    fn sample(&self, x: usize, y: usize) -> [u8; 3] {
        let base = (x >> 1) * 4;
        [
            self.plane.at(base + self.luma[x & 1], y),
            self.plane.at(base + self.u, y),
            self.plane.at(base + self.v, y),
        ]
    }
}

/// A single luma plane, reported with neutral chroma.
struct Luma<'a>(Plane<'a>);

impl Sampler for Luma<'_> {
    #[inline]
    fn sample(&self, x: usize, y: usize) -> [u8; 3] {
        [self.0.at(x, y), 128, 128]
    }
}

/// Packed 8-bit RGB variants: `bytes` per pixel and the byte offsets of R, G, B.
struct PackedRgb<'a> {
    plane: Plane<'a>,
    bytes: usize,
    offsets: [usize; 3],
}

impl<'a> PackedRgb<'a> {
    fn new(plane: Plane<'a>, bytes: usize, offsets: [usize; 3]) -> PackedRgb<'a> {
        PackedRgb { plane, bytes, offsets }
    }
}

impl Sampler for PackedRgb<'_> {
    #[inline]
    fn sample(&self, x: usize, y: usize) -> [u8; 3] {
        let base = x * self.bytes;
        self.offsets.map(|offset| self.plane.at(base + offset, y))
    }
}
