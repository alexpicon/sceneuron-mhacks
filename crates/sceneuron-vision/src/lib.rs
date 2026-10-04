// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Camera analysis for Sceneuron: converts raw OBS frames to RGB, analyzes them with a
//! local [`Perceiver`] (the bundled UltraFace face model when Apple Vision is unavailable),
//! and runs the analysis on a background worker so video threads never wait for it. The
//! result types, the pose, mouth, and facepalm geometry, and the hints for a vision-language
//! model are shared by every perceiver.

mod color;
mod convert;
mod detect;
mod facepalm;
mod facts;
mod frame;
mod hand_shape;
mod layout;
mod mouth;
mod nms;
mod perception;
mod pose;
mod sample;
mod ultraface;
mod worker;

pub use convert::to_rgb;
pub use detect::{DetectError, InputSize, Perceiver, check_rgb};
pub use facepalm::{
    FACE_MEMORY_MS, FACEPALM_COVER, FACEPALM_MAX_HAND_AREA, FACEPALM_MIDDLE_COVER, FACEPALM_MIN_JOINTS, FaceCover,
    FaceMemory, covers_face, face_cover,
};
pub use facts::{FACEPALM_FACT, MIN_NOVEL_TEXT_CHARS, NOVEL_LABEL_CONFIDENCE, SAME_TEXT_SIMILARITY, facts, novelty};
pub use frame::{ConvertError, FrameRef, PixelFormat};
pub use mouth::{MOUTH_CLOSED_FACT, MOUTH_OPEN, MOUTH_OPEN_FACT, mouth_fact, mouth_opening};
pub use perception::{Body, Chirality, FaceBox, Gesture, Hand, Label, Perception, TextLine};
pub use pose::{BodyJoints, HandJoints, Point, RAISED_HAND_LINE, RAISED_HAND_MAX_TILT_DEGREES, body_pose, hand_pose};
pub use sample::{Sample, WorkerEvent};
pub use ultraface::UltraFace;
pub use worker::VisionWorker;

/// Frame width [`UltraFace`] analyzes; convert frames to this size first.
pub const INPUT_WIDTH: u32 = 320;
/// Frame height [`UltraFace`] analyzes; convert frames to this size first.
pub const INPUT_HEIGHT: u32 = 240;
