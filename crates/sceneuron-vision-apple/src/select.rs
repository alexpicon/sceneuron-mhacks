// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Decides which Vision results to keep and converts them to Sceneuron's types. Vision's
//! normalized coordinates have their origin at the bottom left; Sceneuron's at the top left.

use sceneuron_vision::{FaceBox, Label, Point, TextLine};

/// Faces at or above this confidence are listed in [`sceneuron_vision::Perception::faces`].
pub const MIN_FACE_CONFIDENCE: f32 = 0.5;
/// The most faces whose lips are measured in one frame, the largest first: enough for the people
/// on a call around one camera, and a bound on the landmark work.
pub const MAX_MEASURED_MOUTHS: usize = 4;
/// Hand and body joints below this confidence are treated as not located.
pub const MIN_JOINT_CONFIDENCE: f32 = 0.3;
/// Hands and bodies below this confidence are ignored.
pub const MIN_POSE_CONFIDENCE: f32 = 0.3;
/// Animals below this confidence are ignored.
pub const MIN_ANIMAL_CONFIDENCE: f32 = 0.5;
/// Image labels below this confidence are ignored. Vision gives specific objects low
/// confidences: a clearly visible helmet in the test portrait scores 0.23.
pub const MIN_LABEL_CONFIDENCE: f32 = 0.2;
/// Most image labels kept per frame.
pub const MAX_LABELS: usize = 3;
/// Text lines below this confidence are ignored.
pub const MIN_TEXT_CONFIDENCE: f32 = 0.5;
/// Most text lines kept per frame.
pub const MAX_TEXT_LINES: usize = 4;
/// Longest text line kept, in characters; longer lines are cut.
pub const MAX_TEXT_CHARS: usize = 48;

/// Image labels too broad to tell a vision-language model anything; they are dropped,
/// as are catch-all labels ending in "other". Sorted for binary search.
pub const GENERIC_LABELS: &[&str] = &[
    "adult",
    "animal",
    "appliance",
    "art",
    "arthropods",
    "building",
    "canine",
    "celebration",
    "ceremony",
    "clothing",
    "consumer electronics",
    "container",
    "conveyance",
    "cookware",
    "daytime",
    "decoration",
    "document",
    "domicile",
    "drink",
    "entertainer",
    "feline",
    "food",
    "footwear",
    "furniture",
    "games",
    "headgear",
    "housewares",
    "interior room",
    "land",
    "light",
    "liquid",
    "machine",
    "mammal",
    "material",
    "media",
    "music",
    "musical instrument",
    "office supplies",
    "outdoor",
    "people",
    "performance",
    "plant",
    "printed page",
    "recreation",
    "road",
    "screenshot",
    "sky",
    "sport",
    "sports equipment",
    "structure",
    "tableware",
    "textile",
    "tool",
    "utensil",
    "vegetation",
    "vehicle",
    "water",
    "water body",
    "waterways",
    "wood natural",
    "wood processed",
];

/// A rectangle in Vision's normalized coordinates, origin at the bottom left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisionRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// One recognized line of text before selection.
#[derive(Debug, Clone, PartialEq)]
pub struct TextCandidate {
    pub text: String,
    pub confidence: f32,
    /// Height of the line's box as a fraction of the frame height; taller is more prominent.
    pub height: f32,
}

/// Converts a Vision point to Sceneuron's top-left convention.
pub fn point(x: f64, y: f64) -> Point {
    Point {
        x: x as f32,
        y: (1.0 - y) as f32,
    }
}

/// Converts a face landmark point to Sceneuron's frame coordinates. Vision gives landmark points
/// normalized to their face's bounding box, origin at the box's bottom left.
pub fn landmark_point(face: VisionRect, x: f64, y: f64) -> Point {
    point(face.x + x * face.width, face.y + y * face.height)
}

/// The index of the face with the largest box among `faces`, given as boxes and confidences, that
/// reach [`MIN_FACE_CONFIDENCE`]; `None` when there is none.
pub fn largest_face(faces: impl IntoIterator<Item = (VisionRect, f32)>) -> Option<usize> {
    let area = |rect: &VisionRect| {
        let area = rect.width * rect.height;
        if area.is_finite() { area } else { 0.0 }
    };
    faces
        .into_iter()
        .enumerate()
        .filter(|(_, (_, confidence))| *confidence >= MIN_FACE_CONFIDENCE)
        .max_by(|(_, (a, _)), (_, (b, _))| area(a).total_cmp(&area(b)))
        .map(|(index, _)| index)
}

/// The indices of the faces among `faces`, given as boxes and confidences, whose lips are
/// measured: those that reach [`MIN_FACE_CONFIDENCE`], the largest first, at most
/// [`MAX_MEASURED_MOUTHS`].
pub fn measured_faces(faces: impl IntoIterator<Item = (VisionRect, f32)>) -> Vec<usize> {
    let area = |rect: &VisionRect| {
        let area = rect.width * rect.height;
        if area.is_finite() { area } else { 0.0 }
    };
    let mut confident: Vec<(usize, f64)> = faces
        .into_iter()
        .enumerate()
        .filter(|(_, (_, confidence))| *confidence >= MIN_FACE_CONFIDENCE)
        .map(|(index, (rect, _))| (index, area(&rect)))
        .collect();
    confident.sort_by(|a, b| b.1.total_cmp(&a.1));
    confident
        .into_iter()
        .take(MAX_MEASURED_MOUTHS)
        .map(|(index, _)| index)
        .collect()
}

/// The widest of the mouth openings measured in one frame, so that anyone's open mouth counts;
/// `None` when no lips were measured.
pub fn widest_opening(openings: impl IntoIterator<Item = Option<f32>>) -> Option<f32> {
    openings
        .into_iter()
        .flatten()
        .filter(|opening| opening.is_finite())
        .max_by(f32::total_cmp)
}

/// Converts a Vision face rectangle to a [`FaceBox`] clamped to the frame. `None` when it
/// is not finite or has no area inside the frame.
pub fn face_box(rect: VisionRect, score: f32) -> Option<FaceBox> {
    let coords = [rect.x, rect.y, rect.width, rect.height];
    if !coords.iter().all(|value| value.is_finite()) || !score.is_finite() {
        return None;
    }
    let clamp = |value: f64| value.clamp(0.0, 1.0) as f32;
    let (x1, x2) = (clamp(rect.x), clamp(rect.x + rect.width));
    let (y1, y2) = (clamp(1.0 - rect.y - rect.height), clamp(1.0 - rect.y));
    (x2 > x1 && y2 > y1).then_some(FaceBox {
        x1,
        y1,
        x2,
        y2,
        score: score.clamp(0.0, 1.0),
    })
}

/// Turns a Vision identifier such as `"Coffee_Mug"` into a label name such as `"coffee mug"`.
pub fn label_name(identifier: &str) -> String {
    identifier.replace('_', " ").trim().to_lowercase()
}

/// Keeps the most confident specific image labels: at least [`MIN_LABEL_CONFIDENCE`], not
/// in the generic list, at most [`MAX_LABELS`], most confident first.
pub fn labels(candidates: impl IntoIterator<Item = (String, f32)>) -> Vec<Label> {
    let specific = candidates
        .into_iter()
        .filter(|(_, confidence)| *confidence >= MIN_LABEL_CONFIDENCE)
        .map(|(identifier, confidence)| (label_name(&identifier), confidence))
        .filter(|(name, _)| is_specific(name));
    strongest(specific, MAX_LABELS)
}

/// Keeps animals at or above [`MIN_ANIMAL_CONFIDENCE`], one entry per kind, most confident
/// first.
pub fn animals(candidates: impl IntoIterator<Item = (String, f32)>) -> Vec<Label> {
    let confident = candidates
        .into_iter()
        .filter(|(_, confidence)| *confidence >= MIN_ANIMAL_CONFIDENCE)
        .map(|(identifier, confidence)| (label_name(&identifier), confidence))
        .filter(|(name, _)| !name.is_empty());
    strongest(confident, usize::MAX)
}

fn is_specific(name: &str) -> bool {
    !name.is_empty() && !name.ends_with(" other") && GENERIC_LABELS.binary_search(&name).is_err()
}

fn strongest(candidates: impl Iterator<Item = (String, f32)>, limit: usize) -> Vec<Label> {
    let mut kept: Vec<Label> = Vec::new();
    for (name, confidence) in candidates.filter(|(_, confidence)| confidence.is_finite()) {
        match kept.iter_mut().find(|label| label.name == name) {
            Some(label) => label.confidence = label.confidence.max(confidence),
            None => kept.push(Label { name, confidence }),
        }
    }
    kept.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
    kept.truncate(limit);
    kept
}

/// Keeps the most prominent confident lines: at least [`MIN_TEXT_CONFIDENCE`], trimmed,
/// cut to [`MAX_TEXT_CHARS`], at most [`MAX_TEXT_LINES`], tallest first.
pub fn text_lines(candidates: impl IntoIterator<Item = TextCandidate>) -> Vec<TextLine> {
    let mut kept: Vec<TextCandidate> = candidates
        .into_iter()
        .filter(|line| line.confidence >= MIN_TEXT_CONFIDENCE)
        .map(|line| TextCandidate {
            text: line
                .text
                .trim()
                .chars()
                .take(MAX_TEXT_CHARS)
                .collect::<String>()
                .trim_end()
                .to_owned(),
            ..line
        })
        .filter(|line| !line.text.is_empty())
        .collect();
    kept.sort_by(|a, b| {
        b.height
            .total_cmp(&a.height)
            .then(b.confidence.total_cmp(&a.confidence))
    });
    kept.truncate(MAX_TEXT_LINES);
    kept.into_iter()
        .map(|line| TextLine {
            text: line.text,
            confidence: line.confidence,
        })
        .collect()
}
