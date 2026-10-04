// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Camera frames: JPEG encoding for the model and tiny thumbnails for change detection.

use std::sync::Arc;

use jpeg_encoder::{ColorType, Encoder};

use crate::provider::AiError;

const JPEG_QUALITY: u8 = 80;
/// Thumbnail grid: 16:9 like most cameras, coarse enough to ignore sensor noise.
pub const THUMBNAIL_WIDTH: usize = 32;
pub const THUMBNAIL_HEIGHT: usize = 18;
/// A cell counts as changed when its brightness moves by more than this, out of 255.
const CELL_CHANGE: u8 = 20;

/// One camera image in packed 8-bit RGB, three bytes per pixel, rows top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub rgb: Arc<[u8]>,
    pub width: u32,
    pub height: u32,
    /// When the host captured the frame, on the host's millisecond clock.
    pub captured_ms: u64,
}

impl Frame {
    fn is_valid(&self) -> bool {
        let pixels = (self.width as usize).checked_mul(self.height as usize);
        let bytes = pixels.and_then(|pixels| pixels.checked_mul(3));
        self.width > 0 && self.height > 0 && bytes == Some(self.rgb.len())
    }
}

/// Encodes the frame as a JPEG whose longer side is at most `max_width` pixels.
///
/// Larger frames are shrunk by area averaging, which keeps the aspect ratio and avoids the
/// aliasing of nearest-pixel sampling; smaller frames are never enlarged.
pub fn encode_jpeg(frame: &Frame, max_width: u32) -> Result<Vec<u8>, AiError> {
    if !frame.is_valid() {
        let message = format!(
            "the camera frame is {}x{} but carries {} bytes",
            frame.width,
            frame.height,
            frame.rgb.len()
        );
        return Err(AiError::Invalid(message));
    }
    let (width, height) = (frame.width as usize, frame.height as usize);
    let longer = width.max(height);
    let limit = (max_width.max(1) as usize).min(u16::MAX as usize);
    let (out_width, out_height) = if longer > limit {
        let scale = limit as f64 / longer as f64;
        let side = |length: usize| ((length as f64 * scale).round() as usize).clamp(1, limit);
        (side(width), side(height))
    } else {
        (width, height)
    };
    let pixels = if (out_width, out_height) == (width, height) {
        frame.rgb.to_vec()
    } else {
        area_downscale(&frame.rgb, 3, (width, height), (out_width, out_height))
    };
    let mut jpeg = Vec::new();
    let encoder = Encoder::new(&mut jpeg, JPEG_QUALITY);
    let size =
        |length: usize| u16::try_from(length).map_err(|_| AiError::Invalid("the camera frame is too large".into()));
    encoder
        .encode(&pixels, size(out_width)?, size(out_height)?, ColorType::Rgb)
        .map_err(|error| AiError::Invalid(format!("cannot encode the camera frame: {error}")))?;
    Ok(jpeg)
}

/// A 32x18 grayscale summary of a frame, for deciding whether the picture changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thumbnail {
    cells: [u8; THUMBNAIL_WIDTH * THUMBNAIL_HEIGHT],
}

impl Thumbnail {
    /// Averages the frame's brightness over each cell. An invalid frame gives a black thumbnail.
    pub fn of(frame: &Frame) -> Thumbnail {
        let mut cells = [0u8; THUMBNAIL_WIDTH * THUMBNAIL_HEIGHT];
        if frame.is_valid() {
            let (pixels, _) = frame.rgb.as_chunks::<3>();
            let luma: Vec<u8> = pixels
                .iter()
                .map(|[r, g, b]| ((77 * u32::from(*r) + 150 * u32::from(*g) + 29 * u32::from(*b)) >> 8) as u8)
                .collect();
            let size = (frame.width as usize, frame.height as usize);
            let small = area_downscale(&luma, 1, size, (THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT));
            cells.copy_from_slice(&small);
        }
        Thumbnail { cells }
    }

    /// A thumbnail filled with one brightness, for tests and placeholders.
    pub fn uniform(luma: u8) -> Thumbnail {
        Thumbnail {
            cells: [luma; THUMBNAIL_WIDTH * THUMBNAIL_HEIGHT],
        }
    }

    /// Fraction of cells (0 to 1) whose brightness changed by more than 20 out of 255.
    pub fn difference(&self, other: &Thumbnail) -> f64 {
        let changed = self
            .cells
            .iter()
            .zip(&other.cells)
            .filter(|(a, b)| a.abs_diff(**b) > CELL_CHANGE)
            .count();
        changed as f64 / self.cells.len() as f64
    }
}

/// The source pixels one output pixel covers along an axis, with their area weights.
struct Span {
    start: usize,
    weights: Vec<f32>,
}

/// Splits `source` pixels into `target` equal intervals; each weight is the overlap of a source
/// pixel with the interval, so the weights of every span add up to 1.
fn spans(source: usize, target: usize) -> Vec<Span> {
    let scale = source as f64 / target as f64;
    (0..target)
        .map(|index| {
            let begin = index as f64 * scale;
            let end = (index + 1) as f64 * scale;
            let first = (begin.floor() as usize).min(source - 1);
            let last = (end.ceil() as usize).clamp(first + 1, source);
            let weights = (first..last)
                .map(|pixel| {
                    let overlap = end.min(pixel as f64 + 1.0) - begin.max(pixel as f64);
                    (overlap.max(0.0) / scale) as f32
                })
                .collect();
            Span { start: first, weights }
        })
        .collect()
}

/// Resizes interleaved 8-bit pixels by area averaging, one axis at a time.
fn area_downscale(pixels: &[u8], channels: usize, from: (usize, usize), to: (usize, usize)) -> Vec<u8> {
    let ((width, height), (out_width, out_height)) = (from, to);
    let columns = spans(width, out_width);
    let rows = spans(height, out_height);
    let mut horizontal = vec![0f32; height * out_width * channels];
    for y in 0..height {
        let source = &pixels[y * width * channels..(y + 1) * width * channels];
        let target = &mut horizontal[y * out_width * channels..(y + 1) * out_width * channels];
        for (column, span) in columns.iter().enumerate() {
            for (offset, weight) in span.weights.iter().enumerate() {
                let pixel = (span.start + offset) * channels;
                for channel in 0..channels {
                    target[column * channels + channel] += f32::from(source[pixel + channel]) * weight;
                }
            }
        }
    }
    let stride = out_width * channels;
    let mut out = vec![0u8; out_height * stride];
    for (row, span) in rows.iter().enumerate() {
        let target = &mut out[row * stride..(row + 1) * stride];
        for (index, value) in target.iter_mut().enumerate() {
            let sum: f32 = span
                .weights
                .iter()
                .enumerate()
                .map(|(offset, weight)| horizontal[(span.start + offset) * stride + index] * weight)
                .sum();
            *value = sum.round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}
