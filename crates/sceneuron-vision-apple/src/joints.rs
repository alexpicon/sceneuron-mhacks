// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Reads hand and body pose results: the joints of each hand, and each body described with
//! Sceneuron's pose geometry.

use objc2::available;
use objc2_foundation::{NSDictionary, NSString};
use objc2_vision::{
    VNChirality, VNDetectHumanBodyPoseRequest, VNDetectHumanHandPoseRequest, VNHumanHandPoseObservation, VNObservation,
    VNRecognizedPoint,
};
use sceneuron_vision::{Body, BodyJoints, Chirality, HandJoints, Point, body_pose};

use crate::select::{self, MIN_JOINT_CONFIDENCE, MIN_POSE_CONFIDENCE};

type Points = NSDictionary<NSString, VNRecognizedPoint>;

/// The joints and the chirality of every confident hand.
pub(crate) fn hands(request: &VNDetectHumanHandPoseRequest) -> Vec<(HandJoints, Chirality)> {
    // SAFETY: reading the results of a performed request, typed as hand observations.
    let Some(results) = (unsafe { request.results() }) else {
        return Vec::new();
    };
    let names = HandJointNames::get();
    let mut hands = Vec::new();
    for observation in results.iter().filter(|observation| confident(observation)) {
        // SAFETY: a group name constant exported by Vision.
        let points = unsafe {
            observation
                .recognizedPointsForJointsGroupName_error(objc2_vision::VNHumanHandPoseObservationJointsGroupNameAll)
        };
        let Ok(points) = points else { continue };
        let finger = |chain: [&NSString; 4]| chain.map(|name| joint(&points, name));
        let joints = HandJoints {
            wrist: joint(&points, names.wrist),
            thumb: finger(names.thumb),
            index: finger(names.index),
            middle: finger(names.middle),
            ring: finger(names.ring),
            little: finger(names.little),
        };
        hands.push((joints, chirality(&observation)));
    }
    hands
}

/// Every confident body.
pub(crate) fn bodies(request: &VNDetectHumanBodyPoseRequest) -> Vec<Body> {
    use objc2_vision::{
        VNHumanBodyPoseObservationJointNameLeftShoulder as LEFT_SHOULDER,
        VNHumanBodyPoseObservationJointNameLeftWrist as LEFT_WRIST, VNHumanBodyPoseObservationJointNameNose as NOSE,
        VNHumanBodyPoseObservationJointNameRightShoulder as RIGHT_SHOULDER,
        VNHumanBodyPoseObservationJointNameRightWrist as RIGHT_WRIST,
        VNHumanBodyPoseObservationJointsGroupNameAll as ALL,
    };
    // SAFETY: reading the results of a performed request, typed as body observations.
    let Some(results) = (unsafe { request.results() }) else {
        return Vec::new();
    };
    let mut bodies = Vec::new();
    for observation in results.iter().filter(|observation| confident(observation)) {
        // SAFETY: a group name constant exported by Vision.
        let Ok(points) = (unsafe { observation.recognizedPointsForJointsGroupName_error(ALL) }) else {
            continue;
        };
        // SAFETY: the joint names are immutable NSString constants exported by Vision.
        let joints = unsafe {
            BodyJoints {
                nose: joint(&points, NOSE),
                left_shoulder: joint(&points, LEFT_SHOULDER),
                right_shoulder: joint(&points, RIGHT_SHOULDER),
                left_wrist: joint(&points, LEFT_WRIST),
                right_wrist: joint(&points, RIGHT_WRIST),
            }
        };
        bodies.push(body_pose(&joints));
    }
    bodies
}

fn confident(observation: &VNObservation) -> bool {
    // SAFETY: a plain property read on an observation.
    unsafe { observation.confidence() >= MIN_POSE_CONFIDENCE }
}

fn chirality(observation: &VNHumanHandPoseObservation) -> Chirality {
    if !available!(macos = 12.0) {
        return Chirality::Unknown;
    }
    // SAFETY: `chirality` is a plain property, present from macOS 12 as checked above.
    match unsafe { observation.chirality() } {
        VNChirality::Left => Chirality::Left,
        VNChirality::Right => Chirality::Right,
        _ => Chirality::Unknown,
    }
}

/// A located joint in Sceneuron's coordinates, or `None` below [`MIN_JOINT_CONFIDENCE`].
fn joint(points: &Points, name: &NSString) -> Option<Point> {
    let point = points.objectForKey(name)?;
    // SAFETY: plain property reads on a recognized point.
    let (confidence, location) = unsafe { (point.confidence(), point.location()) };
    (confidence >= MIN_JOINT_CONFIDENCE).then(|| select::point(location.x, location.y))
}

struct HandJointNames {
    wrist: &'static NSString,
    thumb: [&'static NSString; 4],
    index: [&'static NSString; 4],
    middle: [&'static NSString; 4],
    ring: [&'static NSString; 4],
    little: [&'static NSString; 4],
}

impl HandJointNames {
    fn get() -> HandJointNames {
        use objc2_vision::*;
        // SAFETY: the joint names are immutable NSString constants exported by Vision.
        unsafe {
            HandJointNames {
                wrist: VNHumanHandPoseObservationJointNameWrist,
                thumb: [
                    VNHumanHandPoseObservationJointNameThumbCMC,
                    VNHumanHandPoseObservationJointNameThumbMP,
                    VNHumanHandPoseObservationJointNameThumbIP,
                    VNHumanHandPoseObservationJointNameThumbTip,
                ],
                index: [
                    VNHumanHandPoseObservationJointNameIndexMCP,
                    VNHumanHandPoseObservationJointNameIndexPIP,
                    VNHumanHandPoseObservationJointNameIndexDIP,
                    VNHumanHandPoseObservationJointNameIndexTip,
                ],
                middle: [
                    VNHumanHandPoseObservationJointNameMiddleMCP,
                    VNHumanHandPoseObservationJointNameMiddlePIP,
                    VNHumanHandPoseObservationJointNameMiddleDIP,
                    VNHumanHandPoseObservationJointNameMiddleTip,
                ],
                ring: [
                    VNHumanHandPoseObservationJointNameRingMCP,
                    VNHumanHandPoseObservationJointNameRingPIP,
                    VNHumanHandPoseObservationJointNameRingDIP,
                    VNHumanHandPoseObservationJointNameRingTip,
                ],
                little: [
                    VNHumanHandPoseObservationJointNameLittleMCP,
                    VNHumanHandPoseObservationJointNameLittlePIP,
                    VNHumanHandPoseObservationJointNameLittleDIP,
                    VNHumanHandPoseObservationJointNameLittleTip,
                ],
            }
        }
    }
}
