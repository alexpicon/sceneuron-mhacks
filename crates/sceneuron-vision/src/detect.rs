// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The perceiver interface: what a local frame analyzer accepts and how it fails.

use std::fmt;

use crate::perception::Perception;

/// Why a perceiver could not be created or could not analyze a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectError {
    /// The model or framework could not be loaded or prepared.
    Load(String),
    /// The frame does not match the size the perceiver asked for, or its byte length does
    /// not match its dimensions.
    InputSize {
        expected_width: u32,
        expected_height: u32,
        width: u32,
        height: u32,
        len: usize,
    },
    /// Running the analysis failed or produced unexpected outputs.
    Inference(String),
}

impl fmt::Display for DetectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DetectError::Load(message) => write!(f, "could not load the vision model: {message}"),
            DetectError::InputSize {
                expected_width,
                expected_height,
                width,
                height,
                len,
            } => write!(
                f,
                "expected a {expected_width}x{expected_height} RGB frame but got {width}x{height} with {len} bytes"
            ),
            DetectError::Inference(message) => write!(f, "frame analysis failed: {message}"),
        }
    }
}

impl std::error::Error for DetectError {}

/// The frame size a perceiver analyzes, chosen from the camera's frame size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSize {
    /// Always this size; frames are stretched to it.
    Fixed { width: u32, height: u32 },
    /// The frame's own aspect ratio, scaled down to fit inside `max_width` x `max_height`.
    /// Frames that already fit keep their size.
    Fit { max_width: u32, max_height: u32 },
}

impl InputSize {
    /// The size to convert a `frame_width` x `frame_height` frame to. Never zero.
    pub fn for_frame(self, frame_width: u32, frame_height: u32) -> (u32, u32) {
        match self {
            InputSize::Fixed { width, height } => (width.max(1), height.max(1)),
            InputSize::Fit { max_width, max_height } => {
                let (max_width, max_height) = (max_width.max(1), max_height.max(1));
                if frame_width == 0 || frame_height == 0 {
                    return (max_width, max_height);
                }
                let scale = (f64::from(max_width) / f64::from(frame_width))
                    .min(f64::from(max_height) / f64::from(frame_height))
                    .min(1.0);
                let scaled = |side: u32, max: u32| ((f64::from(side) * scale).round() as u32).clamp(1, max);
                (scaled(frame_width, max_width), scaled(frame_height, max_height))
            }
        }
    }
}

/// Analyzes packed RGB24 frames on the computer and reports what it sees.
///
/// A perceiver fills only the parts of [`Perception`] it can detect and leaves the rest
/// empty. It runs on one background thread at a time, so it needs `Send` but not `Sync`.
pub trait Perceiver: Send {
    /// A short name for logs, such as `"UltraFace"`.
    fn name(&self) -> &'static str;

    /// The frame size [`Perceiver::perceive`] expects; convert frames to
    /// [`InputSize::for_frame`] first.
    fn input_size(&self) -> InputSize;

    /// Analyzes one `width` x `height` frame of packed RGB24 bytes.
    fn perceive(&mut self, rgb: &[u8], width: u32, height: u32) -> Result<Perception, DetectError>;
}

impl<P: Perceiver + ?Sized> Perceiver for Box<P> {
    fn name(&self) -> &'static str {
        (**self).name()
    }

    fn input_size(&self) -> InputSize {
        (**self).input_size()
    }

    fn perceive(&mut self, rgb: &[u8], width: u32, height: u32) -> Result<Perception, DetectError> {
        (**self).perceive(rgb, width, height)
    }
}

/// Checks that `rgb` holds exactly one non-empty `width` x `height` RGB24 frame.
pub fn check_rgb(rgb: &[u8], width: u32, height: u32) -> Result<(), DetectError> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(3));
    if width == 0 || height == 0 || expected != Some(rgb.len()) {
        return Err(DetectError::InputSize {
            expected_width: width,
            expected_height: height,
            width,
            height,
            len: rgb.len(),
        });
    }
    Ok(())
}
