// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Creates the Vision requests this Mac supports and performs them on one image, turning
//! Objective-C errors and exceptions into [`DetectError`]s.

use std::ffi::CStr;
use std::panic::AssertUnwindSafe;

use objc2::AllocAnyThread;
use objc2::exception::{self, Exception};
use objc2::rc::Retained;
use objc2::runtime::AnyClass;
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_core_graphics::CGImage;
use objc2_foundation::{NSArray, NSDictionary};
use objc2_vision::{
    VNClassifyImageRequest, VNDetectFaceLandmarksRequest, VNDetectFaceRectanglesRequest, VNDetectHumanBodyPoseRequest,
    VNDetectHumanHandPoseRequest, VNFaceObservation, VNFaceObservationAccepting, VNImageRequestHandler,
    VNRecognizeAnimalsRequest, VNRecognizeTextRequest, VNRequest, VNRequestTextRecognitionLevel,
};
use sceneuron_vision::DetectError;

/// Most hands analyzed per frame.
const MAX_HANDS: usize = 2;

/// The requests in use; `None` for kinds that are turned off or not supported here.
pub(crate) struct Requests {
    pub(crate) faces: Retained<VNDetectFaceRectanglesRequest>,
    /// Face landmarks, performed on the largest face only, for its lips.
    pub(crate) mouths: Option<Retained<VNDetectFaceLandmarksRequest>>,
    pub(crate) hands: Option<Retained<VNDetectHumanHandPoseRequest>>,
    pub(crate) bodies: Option<Retained<VNDetectHumanBodyPoseRequest>>,
    pub(crate) animals: Option<Retained<VNRecognizeAnimalsRequest>>,
    pub(crate) labels: Option<Retained<VNClassifyImageRequest>>,
    pub(crate) text: Option<Retained<VNRecognizeTextRequest>>,
}

/// Which kinds of request to create.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Wanted {
    pub(crate) mouths: bool,
    pub(crate) hands: bool,
    pub(crate) bodies: bool,
    pub(crate) animals: bool,
    pub(crate) labels: bool,
    pub(crate) text: bool,
}

impl Requests {
    /// Creates the wanted requests whose classes exist on this system. Face detection is
    /// required; every other kind is skipped when its class is missing.
    pub(crate) fn create(wanted: Wanted) -> Result<Requests, DetectError> {
        let faces = create(c"VNDetectFaceRectanglesRequest", || {
            // SAFETY: `new` returns a fully initialized request with default settings.
            unsafe { VNDetectFaceRectanglesRequest::new() }
        })?
        .ok_or_else(|| DetectError::Load("Vision has no face detection on this system".into()))?;
        let mouths = optional(wanted.mouths, c"VNDetectFaceLandmarksRequest", || {
            // SAFETY: `new` returns a fully initialized request with default settings.
            unsafe { VNDetectFaceLandmarksRequest::new() }
        })?;
        let hands = optional(wanted.hands, c"VNDetectHumanHandPoseRequest", || {
            // SAFETY: `new` returns an initialized request, and any hand count is valid.
            unsafe {
                let request = VNDetectHumanHandPoseRequest::new();
                request.setMaximumHandCount(MAX_HANDS);
                request
            }
        })?;
        let bodies = optional(wanted.bodies, c"VNDetectHumanBodyPoseRequest", || {
            // SAFETY: `new` returns a fully initialized request with default settings.
            unsafe { VNDetectHumanBodyPoseRequest::new() }
        })?;
        let animals = optional(wanted.animals, c"VNRecognizeAnimalsRequest", || {
            // SAFETY: `new` returns a fully initialized request with default settings.
            unsafe { VNRecognizeAnimalsRequest::new() }
        })?;
        let labels = optional(wanted.labels, c"VNClassifyImageRequest", || {
            // SAFETY: `new` returns a fully initialized request with default settings.
            unsafe { VNClassifyImageRequest::new() }
        })?;
        let text = optional(wanted.text, c"VNRecognizeTextRequest", || {
            let request = VNRecognizeTextRequest::new();
            request.setRecognitionLevel(VNRequestTextRecognitionLevel::Fast);
            request.setUsesLanguageCorrection(false);
            request
        })?;
        Ok(Requests {
            faces,
            mouths,
            hands,
            bodies,
            animals,
            labels,
            text,
        })
    }

    /// Performs every request once on `handler`'s image, which loads their models. Face
    /// detection must succeed; other requests that fail are turned off, and the returned
    /// messages say why.
    pub(crate) fn warm_up(&mut self, handler: &VNImageRequestHandler) -> Result<Vec<String>, DetectError> {
        perform(handler, &[&self.faces], "face detection").map_err(|error| DetectError::Load(error.to_string()))?;
        let mut skipped = Vec::new();
        let mut keep = |request: &VNRequest, what: &str| match perform(handler, &[request], what) {
            Ok(()) => true,
            Err(error) => {
                skipped.push(error.to_string());
                false
            }
        };
        self.mouths = self.mouths.take().filter(|request| {
            // Landmarks run on a given face; a made-up one in the middle loads their model.
            let middle = CGRect::new(CGPoint::new(0.25, 0.25), CGSize::new(0.5, 0.5));
            // SAFETY: creating an observation from a normalized bounding box has no preconditions.
            let face = unsafe { VNFaceObservation::observationWithBoundingBox(middle) };
            set_faces(request, &[&face]);
            keep(request, "face landmarks")
        });
        self.hands = self.hands.take().filter(|request| keep(request, "hand pose"));
        self.bodies = self.bodies.take().filter(|request| keep(request, "body pose"));
        self.animals = self
            .animals
            .take()
            .filter(|request| keep(request, "animal recognition"));
        self.labels = self
            .labels
            .take()
            .filter(|request| keep(request, "image classification"));
        self.text = self.text.take().filter(|request| keep(request, "text recognition"));
        Ok(skipped)
    }

    /// Names of the request kinds in use, for logs.
    pub(crate) fn active(&self) -> Vec<&'static str> {
        let optional = [
            ("mouths", self.mouths.is_some()),
            ("hands", self.hands.is_some()),
            ("bodies", self.bodies.is_some()),
            ("animals", self.animals.is_some()),
            ("labels", self.labels.is_some()),
            ("text", self.text.is_some()),
        ];
        std::iter::once("faces")
            .chain(optional.into_iter().filter(|(_, on)| *on).map(|(name, _)| name))
            .collect()
    }
}

fn create<T>(class: &CStr, make: impl FnOnce() -> Retained<T>) -> Result<Option<Retained<T>>, DetectError> {
    if AnyClass::get(class).is_none() {
        return Ok(None);
    }
    let name = class.to_string_lossy();
    exception::catch(AssertUnwindSafe(make))
        .map(Some)
        .map_err(|error| DetectError::Load(format!("creating {name} raised {}", describe(error))))
}

fn optional<T>(
    wanted: bool,
    class: &CStr,
    make: impl FnOnce() -> Retained<T>,
) -> Result<Option<Retained<T>>, DetectError> {
    if wanted { create(class, make) } else { Ok(None) }
}

/// Makes `request` find landmarks on `face` alone instead of detecting faces itself.
pub(crate) fn set_faces(request: &VNDetectFaceLandmarksRequest, faces: &[&VNFaceObservation]) {
    let faces = NSArray::from_slice(faces);
    // SAFETY: the array holds face observations, the type the request accepts, and is copied.
    unsafe { request.setInputFaceObservations(Some(&faces)) };
}

/// A handler that performs requests on `image`.
pub(crate) fn handler(image: &CGImage) -> Result<Retained<VNImageRequestHandler>, DetectError> {
    let options = NSDictionary::new();
    exception::catch(AssertUnwindSafe(|| {
        // SAFETY: the options dictionary is empty, so its value types are trivially right.
        unsafe { VNImageRequestHandler::initWithCGImage_options(VNImageRequestHandler::alloc(), image, &options) }
    }))
    .map_err(|error| DetectError::Inference(format!("creating the image handler raised {}", describe(error))))
}

/// Performs `requests` together on the handler's image; Vision shares the image work
/// between them. Their results are then read from each request.
pub(crate) fn perform(handler: &VNImageRequestHandler, requests: &[&VNRequest], what: &str) -> Result<(), DetectError> {
    let requests = NSArray::from_slice(requests);
    match exception::catch(AssertUnwindSafe(|| handler.performRequests_error(&requests))) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(DetectError::Inference(format!(
            "{what} failed: {}",
            error.localizedDescription()
        ))),
        Err(error) => Err(DetectError::Inference(format!("{what} raised {}", describe(error)))),
    }
}

fn describe(exception: Option<Retained<Exception>>) -> String {
    exception.map_or_else(|| "an unknown exception".to_owned(), |exception| format!("{exception}"))
}
