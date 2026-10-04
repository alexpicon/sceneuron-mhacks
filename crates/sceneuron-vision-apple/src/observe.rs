// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Reads face, animal, label, and text results from performed requests.

use objc2_core_foundation::CGRect;
use objc2_vision::{
    VNClassificationObservation, VNClassifyImageRequest, VNDetectFaceRectanglesRequest, VNRecognizeAnimalsRequest,
    VNRecognizeTextRequest,
};
use sceneuron_vision::{FaceBox, Label, TextLine};

use crate::select::{self, MIN_FACE_CONFIDENCE, MIN_LABEL_CONFIDENCE, TextCandidate, VisionRect};

/// Faces at or above [`MIN_FACE_CONFIDENCE`], most confident first, and the best face
/// confidence overall.
pub(crate) fn faces(request: &VNDetectFaceRectanglesRequest) -> (Vec<FaceBox>, f32) {
    // SAFETY: reading the results of a performed request, typed as face observations.
    let Some(results) = (unsafe { request.results() }) else {
        return (Vec::new(), 0.0);
    };
    let mut best = 0.0f32;
    let mut faces = Vec::new();
    for observation in results.iter() {
        // SAFETY: plain property reads on an observation.
        let (confidence, rect) = unsafe { (observation.confidence(), observation.boundingBox()) };
        if confidence.is_finite() {
            best = best.max(confidence.min(1.0));
        }
        if confidence >= MIN_FACE_CONFIDENCE
            && let Some(face) = select::face_box(vision_rect(rect), confidence)
        {
            faces.push(face);
        }
    }
    faces.sort_by(|a, b| b.score.total_cmp(&a.score));
    (faces, best)
}

/// The animals recognized, one entry per kind.
pub(crate) fn animals(request: &VNRecognizeAnimalsRequest) -> Vec<Label> {
    // SAFETY: reading the results of a performed request, typed as object observations.
    let Some(results) = (unsafe { request.results() }) else {
        return Vec::new();
    };
    let candidates = results.iter().flat_map(|observation| {
        // SAFETY: a plain property read on an observation.
        let labels = unsafe { observation.labels() };
        labels
            .iter()
            .map(|label| (identifier(&label), confidence(&label)))
            .collect::<Vec<_>>()
    });
    select::animals(candidates)
}

/// The most confident specific image labels.
pub(crate) fn labels(request: &VNClassifyImageRequest) -> Vec<Label> {
    // SAFETY: reading the results of a performed request, typed as classifications.
    let Some(results) = (unsafe { request.results() }) else {
        return Vec::new();
    };
    // Vision scores every class it knows, so names are read only for likely candidates.
    let candidates = results.iter().filter_map(|observation| {
        let confidence = confidence(&observation);
        (confidence >= MIN_LABEL_CONFIDENCE).then(|| (identifier(&observation), confidence))
    });
    select::labels(candidates)
}

/// The most prominent confident lines of text.
pub(crate) fn text(request: &VNRecognizeTextRequest) -> Vec<TextLine> {
    let Some(results) = request.results() else {
        return Vec::new();
    };
    let candidates = results.iter().filter_map(|observation| {
        let best = observation.topCandidates(1).iter().next()?;
        // SAFETY: a plain property read on an observation.
        let rect = unsafe { observation.boundingBox() };
        Some(TextCandidate {
            text: best.string().to_string(),
            confidence: best.confidence(),
            height: rect.size.height as f32,
        })
    });
    select::text_lines(candidates)
}

fn identifier(classification: &VNClassificationObservation) -> String {
    // SAFETY: a plain property read on a classification.
    unsafe { classification.identifier() }.to_string()
}

fn confidence(classification: &VNClassificationObservation) -> f32 {
    // SAFETY: a plain property read on a classification.
    unsafe { classification.confidence() }
}

pub(crate) fn vision_rect(rect: CGRect) -> VisionRect {
    VisionRect {
        x: rect.origin.x,
        y: rect.origin.y,
        width: rect.size.width,
        height: rect.size.height,
    }
}
