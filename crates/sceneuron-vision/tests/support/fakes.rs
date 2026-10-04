// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Scripted perceivers for worker tests.

use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use sceneuron_vision::{DetectError, INPUT_HEIGHT, INPUT_WIDTH, InputSize, Perceiver, Perception};

/// Longest a gated analysis waits for its release. Bounded so that a failing test, whose
/// unwinding joins the worker before dropping the release sender, fails instead of hanging.
pub const GATE_WAIT: Duration = Duration::from_secs(30);

/// Returns a perception whose best face score is `score`. It can announce each analysis
/// as it starts, wait for a release signal, and then sleep for `delay`.
pub struct Fake {
    pub score: f32,
    pub delay: Duration,
    pub started: Option<Sender<()>>,
    pub gate: Option<Receiver<()>>,
}

impl Fake {
    pub fn new(score: f32) -> Fake {
        Fake {
            score,
            delay: Duration::ZERO,
            started: None,
            gate: None,
        }
    }
}

/// The input size every fake reports.
pub const FAKE_INPUT: InputSize = InputSize::Fixed {
    width: INPUT_WIDTH,
    height: INPUT_HEIGHT,
};

impl Perceiver for Fake {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn input_size(&self) -> InputSize {
        FAKE_INPUT
    }

    fn perceive(&mut self, _rgb: &[u8], _width: u32, _height: u32) -> Result<Perception, DetectError> {
        if let Some(started) = &self.started {
            started.send(()).expect("test listens for analysis starts");
        }
        if let Some(gate) = &self.gate {
            gate.recv_timeout(GATE_WAIT).expect("test releases the perceiver");
        }
        thread::sleep(self.delay);
        Ok(Perception {
            best_face: self.score,
            ..Perception::default()
        })
    }
}

/// Fails its first analysis, then succeeds.
pub struct FailsOnce(pub bool);

impl Perceiver for FailsOnce {
    fn name(&self) -> &'static str {
        "fails once"
    }

    fn input_size(&self) -> InputSize {
        FAKE_INPUT
    }

    fn perceive(&mut self, _rgb: &[u8], _width: u32, _height: u32) -> Result<Perception, DetectError> {
        if std::mem::replace(&mut self.0, true) {
            Ok(Perception::default())
        } else {
            Err(DetectError::Inference("model hiccup".into()))
        }
    }
}

/// Panics on every analysis.
pub struct Panics;

impl Perceiver for Panics {
    fn name(&self) -> &'static str {
        "panics"
    }

    fn input_size(&self) -> InputSize {
        FAKE_INPUT
    }

    fn perceive(&mut self, _rgb: &[u8], _width: u32, _height: u32) -> Result<Perception, DetectError> {
        panic!("perceiver exploded");
    }
}
