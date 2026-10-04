// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The Apple Vision perceiver: runs the Vision requests on each frame and assembles a
//! [`Perception`].

use std::time::Instant;

use objc2::rc::autoreleasepool;
use objc2_core_foundation::CFRetained;
use objc2_core_graphics::CGColorSpace;
use objc2_vision::VNRequest;
use sceneuron_vision::{
    DetectError, FaceMemory, Hand, InputSize, Label, Perceiver, Perception, TextLine, check_rgb, covers_face, hand_pose,
};

use crate::image::{rgb_image, srgb};
use crate::requests::{Requests, Wanted, handler, perform};
use crate::{joints, mouth, observe};

/// Longest side of the frames Apple Vision analyzes; frames keep their aspect ratio, so a
/// 16:9 camera is analyzed at 640x360. Hands and text need more pixels than faces.
pub const INPUT_MAX_SIDE: u32 = 640;

/// Which analyses to run, and how often to run the expensive ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// How far apart the lips of the largest face are, from face landmarks.
    pub mouths: bool,
    pub hands: bool,
    pub bodies: bool,
    pub animals: bool,
    pub labels: bool,
    pub text: bool,
    /// Image labels and text, the most expensive analyses, run on the first frame and then
    /// on every `slow_every`-th frame; frames in between repeat their last results. Zero
    /// counts as one.
    pub slow_every: u32,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            mouths: true,
            hands: true,
            bodies: true,
            animals: true,
            labels: true,
            text: true,
            slow_every: 5,
        }
    }
}

/// How long the stages of one frame's analysis took, in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FrameTiming {
    /// Copying the frame into an image for Vision.
    pub image_ms: f64,
    /// Performing the requests.
    pub vision_ms: f64,
    /// Reading the results, the landmarks of the largest face included.
    pub read_ms: f64,
    /// Whether image labels and text ran on this frame.
    pub slow: bool,
}

/// Analyzes frames with Apple's Vision framework. Face detection is always on; the other
/// analyses run when wanted and supported by the system.
pub struct AppleVision {
    requests: Requests,
    space: CFRetained<CGColorSpace>,
    slow_every: u64,
    frames: u64,
    labels: Vec<Label>,
    text: Vec<TextLine>,
    /// The face hands are measured against for a facepalm, on the clock of `started`.
    faces: FaceMemory,
    started: Instant,
    skipped: Vec<String>,
    timing: Option<FrameTiming>,
}

// SAFETY: the Vision requests are not thread-safe, but `AppleVision` owns them exclusively
// and only uses them through `&mut self`, so moving it to another thread never lets two
// threads use them at once. Vision supports performing requests on any thread.
unsafe impl Send for AppleVision {}

impl AppleVision {
    /// Creates every supported analysis and runs each once to load its model. That takes
    /// under a second once macOS has prepared the models, but the very first run on a Mac
    /// can take tens of seconds, so pass it to a worker to keep it off the caller's thread.
    pub fn new() -> Result<AppleVision, DetectError> {
        AppleVision::with_options(Options::default())
    }

    /// Like [`AppleVision::new`] with a choice of analyses. Fails only when face detection
    /// is unavailable; other analyses that cannot be created or run are skipped and listed
    /// by [`AppleVision::skipped`].
    pub fn with_options(options: Options) -> Result<AppleVision, DetectError> {
        autoreleasepool(|_| {
            let mut requests = Requests::create(Wanted {
                mouths: options.mouths,
                hands: options.hands,
                bodies: options.bodies,
                animals: options.animals,
                labels: options.labels,
                text: options.text,
            })?;
            let space = srgb()?;
            let (width, height) = (320, 240);
            let gray = vec![128u8; width as usize * height as usize * 3];
            let image = rgb_image(&gray, width, height, &space)?;
            let skipped = requests.warm_up(&*handler(&image)?)?;
            Ok(AppleVision {
                requests,
                space,
                slow_every: u64::from(options.slow_every.max(1)),
                frames: 0,
                labels: Vec::new(),
                text: Vec::new(),
                faces: FaceMemory::default(),
                started: Instant::now(),
                skipped,
                timing: None,
            })
        })
    }

    /// The analyses in use, such as `["faces", "hands", "text"]`.
    pub fn active(&self) -> Vec<&'static str> {
        self.requests.active()
    }

    /// Whether it measures how far apart the lips are, for [`Perception::mouth_opening`].
    pub fn measures_mouths(&self) -> bool {
        self.requests.mouths.is_some()
    }

    /// Whether it locates hands and their joints, for [`Perception::hands`] and
    /// [`Perception::facepalm`].
    pub fn locates_hands(&self) -> bool {
        self.requests.hands.is_some()
    }

    /// Why wanted analyses were turned off while starting.
    pub fn skipped(&self) -> &[String] {
        &self.skipped
    }

    /// How long the last successful frame took.
    pub fn last_timing(&self) -> Option<FrameTiming> {
        self.timing
    }

    fn analyze(&mut self, rgb: &[u8], width: u32, height: u32, slow: bool) -> Result<Perception, DetectError> {
        let started = Instant::now();
        let image = rgb_image(rgb, width, height, &self.space)?;
        let handler = handler(&image)?;
        let converted = Instant::now();
        let requests = &self.requests;
        let mut batch: Vec<&VNRequest> = vec![&requests.faces];
        batch.extend(requests.hands.as_deref().map(|request| request as &VNRequest));
        batch.extend(requests.bodies.as_deref().map(|request| request as &VNRequest));
        batch.extend(requests.animals.as_deref().map(|request| request as &VNRequest));
        if slow {
            batch.extend(requests.labels.as_deref().map(|request| request as &VNRequest));
            batch.extend(requests.text.as_deref().map(|request| request as &VNRequest));
        }
        perform(&handler, &batch, "Vision analysis")?;
        let performed = Instant::now();
        let (faces, best_face) = observe::faces(&requests.faces);
        let aspect = width as f32 / height as f32;
        let mouth_opening = requests
            .mouths
            .as_deref()
            .and_then(|landmarks| mouth::widest(&handler, &requests.faces, landmarks, aspect));
        let located = requests.hands.as_deref().map(joints::hands).unwrap_or_default();
        let hand_joints: Vec<_> = located.iter().map(|(joints, _)| *joints).collect();
        let now_ms = u64::try_from(started.duration_since(self.started).as_millis()).unwrap_or(u64::MAX);
        let face = self.faces.anchor(&faces, &hand_joints, now_ms);
        let perception = Perception {
            faces,
            best_face,
            mouth_opening,
            hands: located
                .iter()
                .map(|(joints, chirality)| Hand {
                    over_face: face.is_some_and(|face| covers_face(joints, &face)),
                    ..hand_pose(joints, *chirality, aspect)
                })
                .collect(),
            bodies: requests.bodies.as_deref().map(joints::bodies).unwrap_or_default(),
            animals: requests.animals.as_deref().map(observe::animals).unwrap_or_default(),
            labels: match requests.labels.as_deref() {
                Some(request) if slow => observe::labels(request),
                _ => std::mem::take(&mut self.labels),
            },
            text: match requests.text.as_deref() {
                Some(request) if slow => observe::text(request),
                _ => std::mem::take(&mut self.text),
            },
        };
        self.labels.clone_from(&perception.labels);
        self.text.clone_from(&perception.text);
        self.timing = Some(FrameTiming {
            image_ms: millis(started, converted),
            vision_ms: millis(converted, performed),
            read_ms: millis(performed, Instant::now()),
            slow,
        });
        Ok(perception)
    }
}

impl Perceiver for AppleVision {
    fn name(&self) -> &'static str {
        "Apple Vision"
    }

    fn input_size(&self) -> InputSize {
        InputSize::Fit {
            max_width: INPUT_MAX_SIDE,
            max_height: INPUT_MAX_SIDE,
        }
    }

    fn perceive(&mut self, rgb: &[u8], width: u32, height: u32) -> Result<Perception, DetectError> {
        check_rgb(rgb, width, height)?;
        let slow = self.frames.is_multiple_of(self.slow_every);
        self.frames += 1;
        autoreleasepool(|_| self.analyze(rgb, width, height, slow))
    }
}

fn millis(from: Instant, to: Instant) -> f64 {
    to.duration_since(from).as_secs_f64() * 1000.0
}
