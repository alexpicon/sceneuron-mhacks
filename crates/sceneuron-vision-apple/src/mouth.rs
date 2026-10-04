// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Reads the lips of the faces in view from Vision's face landmarks and measures how far apart
//! they are, so that the widest open mouth counts, whoever it belongs to.

use objc2::rc::Retained;
use objc2_vision::{
    VNDetectFaceLandmarksRequest, VNDetectFaceRectanglesRequest, VNFaceLandmarkRegion2D, VNFaceObservation,
    VNImageRequestHandler, VNRequest,
};
use sceneuron_vision::{Point, mouth_opening};

use crate::observe::vision_rect;
use crate::requests::{perform, set_faces};
use crate::select::{self, VisionRect};

/// How far apart the lips are of the widest open mouth among the confident faces that `faces`
/// found, the largest few, measured by performing `landmarks` on those faces alone. `aspect` is
/// the frame's width divided by its height. `None` without such a face, when the landmarks fail,
/// or when no lips were located; the rest of the frame's analysis stands either way.
pub(crate) fn widest(
    handler: &VNImageRequestHandler,
    faces: &VNDetectFaceRectanglesRequest,
    landmarks: &VNDetectFaceLandmarksRequest,
    aspect: f32,
) -> Option<f32> {
    let measured = measured_faces(faces);
    if measured.is_empty() {
        return None;
    }
    let chosen: Vec<&VNFaceObservation> = measured.iter().map(|face| &**face).collect();
    set_faces(landmarks, &chosen);
    let request: &VNRequest = landmarks;
    perform(handler, &[request], "face landmarks").ok()?;
    // SAFETY: reading the results of a performed request, typed as face observations.
    let results = unsafe { landmarks.results() }?;
    select::widest_opening(results.iter().map(|observation| opening(&observation, aspect)))
}

/// How far apart one observed face's lips are.
fn opening(observation: &VNFaceObservation, aspect: f32) -> Option<f32> {
    // SAFETY: plain property reads on an observation and its landmarks.
    let (rect, lips) = unsafe { (vision_rect(observation.boundingBox()), observation.landmarks()?) };
    // SAFETY: plain property reads on the landmarks.
    let (inner, outer) = unsafe { (lips.innerLips()?, lips.outerLips()?) };
    mouth_opening(&points(&inner, rect), &points(&outer, rect), aspect)
}

fn measured_faces(request: &VNDetectFaceRectanglesRequest) -> Vec<Retained<VNFaceObservation>> {
    // SAFETY: reading the results of a performed request, typed as face observations.
    let Some(results) = (unsafe { request.results() }) else {
        return Vec::new();
    };
    let faces: Vec<Retained<VNFaceObservation>> = results.iter().collect();
    // SAFETY: plain property reads on observations.
    let boxes = faces
        .iter()
        .map(|face| unsafe { (vision_rect(face.boundingBox()), face.confidence()) });
    select::measured_faces(boxes)
        .into_iter()
        .map(|index| faces[index].clone())
        .collect()
}

/// The region's points in Sceneuron's frame coordinates; `face` is its face's bounding box.
fn points(region: &VNFaceLandmarkRegion2D, face: VisionRect) -> Vec<Point> {
    // SAFETY: plain property reads on a region.
    let (count, first) = unsafe { (region.pointCount(), region.normalizedPoints()) };
    if first.is_null() || count == 0 {
        return Vec::new();
    }
    // SAFETY: Vision keeps `count` points at `first` for as long as the region lives, and the
    // region outlives this borrow.
    let raw = unsafe { std::slice::from_raw_parts(first, count) };
    raw.iter()
        .map(|point| select::landmark_point(face, point.x, point.y))
        .collect()
}
