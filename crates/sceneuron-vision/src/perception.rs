// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What a perceiver reports about one frame. Coordinates are normalized to `0.0..=1.0` of
//! the frame with the origin at the top left.

/// One detected face.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceBox {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    /// Face probability, `0.0..=1.0`.
    pub score: f32,
}

/// A named thing seen in the frame, such as an animal or an image label.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    /// Lowercase, human-readable name, such as `"cat"` or `"coffee mug"`.
    pub name: String,
    /// `0.0..=1.0`.
    pub confidence: f32,
}

/// Which hand a [`Hand`] is, from the person's point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Chirality {
    Left,
    Right,
    #[default]
    Unknown,
}

/// A static hand shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gesture {
    /// A fist with the thumb pointing up.
    ThumbsUp,
    /// A fist with the thumb pointing down, its tip below the rest of the hand.
    ThumbsDown,
    /// All four fingers extended, as in a wave or a stop sign.
    OpenPalm,
    /// Index and middle fingers extended, the others curled (V sign).
    Peace,
    /// Only the index finger extended.
    Pointing,
    /// All fingers curled and the thumb not extended.
    Fist,
}

impl Gesture {
    /// Short English description, such as `"thumbs up"`.
    pub fn describe(self) -> &'static str {
        match self {
            Gesture::ThumbsUp => "thumbs up",
            Gesture::ThumbsDown => "thumbs down",
            Gesture::OpenPalm => "open palm",
            Gesture::Peace => "peace sign",
            Gesture::Pointing => "pointing",
            Gesture::Fist => "fist",
        }
    }
}

/// One detected hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Hand {
    pub chirality: Chirality,
    /// The hand is held up: it points up and lies wholly in the upper part of the frame
    /// (see [`RAISED_HAND_LINE`](crate::RAISED_HAND_LINE)).
    pub raised: bool,
    /// The recognized hand shape, if any.
    pub gesture: Option<Gesture>,
    /// How many fingers are extended, `0..=5`, counting the thumb when it is extended in
    /// any direction; `None` when a finger is half bent or a joint needed to tell is
    /// missing.
    pub extended: Option<u8>,
    /// The hand covers the upper half of the largest face (see
    /// [`covers_face`](crate::covers_face)): a facepalm, so it gives no card signal.
    pub over_face: bool,
}

/// One detected person's body pose. A wrist counts as up when it is above the nose, or
/// above its shoulder when the nose is not visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Body {
    /// The person's left wrist is up.
    pub left_arm_up: bool,
    /// The person's right wrist is up.
    pub right_arm_up: bool,
    /// Both wrists are up.
    pub arms_up: bool,
}

/// One line of recognized text.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLine {
    /// Trimmed text.
    pub text: String,
    /// `0.0..=1.0`.
    pub confidence: f32,
}

/// Everything a perceiver found in one frame. Parts a perceiver cannot detect stay empty.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Perception {
    /// Faces above the detection threshold, highest score first.
    pub faces: Vec<FaceBox>,
    /// The highest face probability anywhere in the frame, even below the threshold.
    /// Presence is decided from this score.
    pub best_face: f32,
    /// How far apart the lips of the widest open mouth in view are, as a fraction of its width (see
    /// [`mouth_opening`](crate::mouth_opening)); `None` when no face's lips were located.
    pub mouth_opening: Option<f32>,
    pub hands: Vec<Hand>,
    pub bodies: Vec<Body>,
    /// Animals, most confident first.
    pub animals: Vec<Label>,
    /// Labels describing the whole image, most confident first.
    pub labels: Vec<Label>,
    /// Lines of text, most prominent first.
    pub text: Vec<TextLine>,
}

impl Perception {
    /// A hand covers the upper half of the largest face (see [`Hand::over_face`]), measured
    /// against the face as it was just before the hand hid it (see
    /// [`FaceMemory`](crate::FaceMemory)).
    pub fn facepalm(&self) -> bool {
        self.hands.iter().any(|hand| hand.over_face)
    }
}
