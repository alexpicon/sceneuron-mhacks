// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The live description: one short line about what on-device analysis found in the camera, such
//! as "1 person; right hand raised, peace sign; mouth open", for `{{seen}}` in overlay text and for
//! the editor. The host reports each analyzed frame as a [`Sighting`], built from facts its
//! analysis already computed, so the line needs no AI and never leaves the computer.
//!
//! The line stays readable on stream: an open mouth counts as open for [`MOUTH_LINGER_MS`] after
//! it closes, so speech does not make the line flicker; a new line replaces the shown one only
//! once [`SETTLE_FRAMES`] analyzed frames in a row showed it, and at most once every
//! [`SEEN_CHANGE_MS`]; and the line clears when camera analysis stops or its frames stop coming.
//!
//! `{{scene:age}}` shows the vision model's caption with how long ago the camera image it
//! describes was taken, such as "A person waving (just now)", for the same overlay line.

mod engine;
mod sentence;

use crate::signals::HandSign;

pub use sentence::MAX_SEEN_CHARS;

pub(crate) use engine::LiveDescription;

/// The least time between two changes of the shown line: at most two and a half a second.
pub const SEEN_CHANGE_MS: u64 = 400;
/// How many analyzed frames in a row must show a new line before it replaces the shown one, so
/// one frame a detector got wrong never reaches the stream.
pub const SETTLE_FRAMES: u32 = 2;
/// How long an open mouth stays in the line after the mouth closes.
pub const MOUTH_LINGER_MS: u64 = 1_000;

/// Which hand, from the person's point of view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HandSide {
    Left,
    Right,
    #[default]
    Unknown,
}

/// One hand on-device analysis found.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SeenHand {
    pub side: HandSide,
    /// The hand is held up.
    pub raised: bool,
    /// The sign it shows, if any.
    pub sign: Option<HandSign>,
    /// How many fingers are extended, `0..=5`, when known.
    pub fingers: Option<u8>,
}

/// What on-device analysis found in one analyzed frame, as far as the live description goes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sighting {
    /// People in view: the faces or the bodies found, whichever are more.
    pub people: usize,
    /// Hands in view, apart from a hand over the face.
    pub hands: Vec<SeenHand>,
    /// A hand covers the face.
    pub facepalm: bool,
    /// The most arms one person holds up: 0, 1, or 2.
    pub arms_up: u8,
    /// The largest face's mouth is open.
    pub mouth_open: bool,
    /// Animals, most confident first, such as "cat".
    pub animals: Vec<String>,
    /// Image labels that are not animals, most confident first, such as "guitar".
    pub things: Vec<String>,
    /// Lines of text, most prominent first.
    pub text: Vec<String>,
}
