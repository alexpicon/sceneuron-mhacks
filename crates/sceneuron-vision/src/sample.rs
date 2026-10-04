// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The values exchanged with the background worker: samples in, events out.

use std::fmt;

use crate::perception::Perception;

/// One RGB24 frame to analyze.
#[derive(Clone, PartialEq, Eq)]
pub struct Sample {
    pub rgb: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Caller clock time of the frame, passed through to [`WorkerEvent::Detected`].
    pub captured_ms: u64,
}

impl fmt::Debug for Sample {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sample")
            .field("rgb_len", &self.rgb.len())
            .field("width", &self.width)
            .field("height", &self.height)
            .field("captured_ms", &self.captured_ms)
            .finish()
    }
}

/// What the worker reports, from its own thread; only a failure to start that thread is
/// reported from [`VisionWorker::spawn`](crate::VisionWorker::spawn) itself.
#[derive(Debug, Clone, PartialEq)]
pub enum WorkerEvent {
    /// The perceiver was created and samples are accepted. `perceiver` is its
    /// [`name`](crate::Perceiver::name).
    Ready { perceiver: &'static str },
    /// Creating the perceiver failed, analyzing a sample failed, or the perceiver panicked.
    /// Only a failed analysis leaves the worker accepting samples.
    Failed(String),
    /// A sample was analyzed. `latency_ms` runs from acceptance to the result.
    Detected {
        perception: Perception,
        captured_ms: u64,
        latency_ms: f64,
    },
}
