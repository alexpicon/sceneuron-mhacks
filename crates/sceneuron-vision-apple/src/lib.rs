// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A Sceneuron [`Perceiver`](sceneuron_vision::Perceiver) built on Apple's Vision framework,
//! which runs on the Neural Engine where available. It reports faces, how far apart the lips
//! of the largest face are and whether a hand covers it, hand gestures, body poses, animals,
//! image labels, and text.
//!
//! The [`select`] module, which decides which Vision results to keep, works on plain values, so
//! its rules are tested without running Vision.

pub mod select;

mod image;
mod joints;
mod mouth;
mod observe;
mod perceiver;
mod requests;

pub use perceiver::{AppleVision, FrameTiming, INPUT_MAX_SIDE, Options};
