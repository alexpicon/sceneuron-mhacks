// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What the editor's camera preview shows: a picture of what camera analysis sees, or why there
//! is none, and what on-device analysis found.

use serde::Serialize;

/// The longest side, in pixels, of a picture [`Host::camera_preview`](crate::Host::camera_preview)
/// returns.
pub const PREVIEW_MAX_SIDE: u32 = 640;

/// A JPEG of what camera analysis sees now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewImage {
    pub jpeg: Vec<u8>,
    /// A test picture drawn by a host without a camera, not a camera frame.
    pub simulated: bool,
}

/// Why the camera preview has no picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewGap {
    /// Sceneuron is not running inside OBS, so there is no camera.
    NotInObs,
    /// No source has the camera analysis filter.
    NoSource,
    /// The preview just started and the first frame has not arrived yet.
    Waiting,
    /// The analyzed source has sent no frames for a while.
    Stale,
}

/// One face that on-device analysis found, as fractions of the frame from its top left.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct FaceArea {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    /// Face probability, `0.0..=1.0`.
    pub score: f32,
}

/// What on-device analysis found in the latest analyzed frame.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct CameraAnalysis {
    /// Short descriptions, such as `face in view (0.98)`, `right hand raised, thumbs up`, or
    /// `3 fingers up`.
    pub seen: Vec<String>,
    pub faces: Vec<FaceArea>,
    /// How long ago the analyzed frame was captured; `None` when nothing was analyzed lately.
    pub age_ms: Option<u64>,
}
