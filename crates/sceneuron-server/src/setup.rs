// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! "Set up OBS for me": what one click puts in place, each part only when it is missing: the
//! Sceneuron Overlay in the current scene, camera analysis on a camera, the Sceneuron Voice
//! filter on Mic/Aux, and a paired Stream Deck. The host does the work in OBS and the server
//! pairs the Stream Deck; the answer says what each did and what it skipped.

use serde::Serialize;

use crate::host::SourceInfo;

/// The source types of OBS's camera devices, likeliest to show the user first: Video Capture
/// Device and its older version, and last the Capture Card Device, which more often carries a
/// game or another computer.
const CAMERA_KINDS: [&str; 3] = ["macos-avcapture", "av_capture_input", "macos-avcapture-fast"];

/// What setting up OBS did with each part Sceneuron needs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SetupReport {
    /// The Sceneuron Overlay in the current scene.
    pub overlay: SetupStep,
    /// Camera analysis on a camera device.
    pub camera: SetupStep,
    /// The Sceneuron Voice filter on a microphone.
    pub microphone: SetupStep,
}

/// What "Set up OBS for me" answers: the host's report, and pairing the Stream Deck.
#[derive(Debug, Serialize)]
pub(crate) struct SetupAnswer {
    #[serde(flatten)]
    pub(crate) obs: SetupReport,
    /// The Stream Deck on this computer: added when a pairing link went to Sceneuron's plugin in
    /// the Stream Deck app, present when a control surface is paired already.
    pub(crate) stream_deck: SetupStep,
}

/// What happened to one part.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SetupStep {
    pub outcome: SetupOutcome,
    /// The source the part is on: the overlay, the camera, or the microphone, or `Stream Deck`;
    /// empty when skipped.
    pub source: String,
    /// Why the part was skipped; empty otherwise.
    pub reason: String,
}

/// Whether a part was added, already there, or could not be added.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SetupOutcome {
    /// Added now.
    Added,
    /// Already in place, so it was left as it is.
    Present,
    /// Not added; the reason says why.
    Skipped,
}

impl SetupStep {
    /// The part was added now, on `source`.
    pub fn added(source: &str) -> SetupStep {
        Self::new(SetupOutcome::Added, source, "")
    }

    /// The part was already on `source`.
    pub fn present(source: &str) -> SetupStep {
        Self::new(SetupOutcome::Present, source, "")
    }

    /// The part could not be added, for `reason`.
    pub fn skipped(reason: &str) -> SetupStep {
        Self::new(SetupOutcome::Skipped, "", reason)
    }

    fn new(outcome: SetupOutcome, source: &str, reason: &str) -> SetupStep {
        SetupStep {
            outcome,
            source: source.to_owned(),
            reason: reason.to_owned(),
        }
    }
}

/// The source camera analysis should watch when none is analyzed: the first source of the
/// likeliest camera type in `CAMERA_KINDS`. Any other source, such as a media file or a network
/// stream, may show no one, and analysis would then call the user away, so with no camera device
/// there is none; the user picks the source to analyze instead.
pub fn camera_to_analyze(sources: &[SourceInfo]) -> Option<&SourceInfo> {
    CAMERA_KINDS
        .iter()
        .find_map(|kind| sources.iter().find(|source| source.kind == *kind))
}
