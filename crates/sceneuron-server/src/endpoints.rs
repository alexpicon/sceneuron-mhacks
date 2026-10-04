// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The routes: each path, the methods it answers, and the access it needs.

use crate::access::Access;
use crate::assets::{self, Asset, EDITOR_PAGE, OVERLAY_PAGE};

const GET: &[&str] = &["GET"];
const GET_PUT: &[&str] = &["GET", "PUT"];
const GET_POST: &[&str] = &["GET", "POST"];
const POST: &[&str] = &["POST"];

#[derive(Clone, Copy, Debug)]
pub(crate) enum Endpoint {
    Root,
    File(&'static Asset),
    State,
    Project,
    Override,
    Release,
    Effect,
    /// Plays a reaction preset over the whole overlay.
    Flood,
    Automation,
    Simulate,
    Sources,
    Attach,
    /// A JPEG of what camera analysis sees, for the editor only.
    CameraPreview,
    CameraAnalysis,
    AudioSources,
    AttachVoice,
    AddOverlay,
    /// "Set up OBS for me": the overlay, camera analysis, the voice filter, and a paired Stream
    /// Deck, where missing.
    Setup,
    OverlayUrl,
    Hello,
    AiSettings,
    AiPlan,
    AiAnalyze,
    Events,
    Packs,
    Card,
    ControlElement,
    ControlState,
    ControlEvents,
    Pairing,
    Pair,
    Revoke,
    Room,
    Voice,
    VoiceTalk,
    /// What can be said to Sceneuron now.
    VoiceGuide,
    /// Typed words carried out as a voice command.
    VoiceCommand,
    /// The audio of a spoken card, at `/api/speech/<id>`.
    Speech,
    Subtitles,
    SubtitlesSimulate,
}

impl Endpoint {
    pub(crate) fn find(path: &str) -> Option<Self> {
        let endpoint = match path {
            "/" => Self::Root,
            "/editor" => Self::File(&EDITOR_PAGE),
            "/overlay" => Self::File(&OVERLAY_PAGE),
            "/api/state" => Self::State,
            "/api/project" => Self::Project,
            "/api/override" => Self::Override,
            "/api/release" => Self::Release,
            "/api/effect" => Self::Effect,
            "/api/flood" => Self::Flood,
            "/api/automation" => Self::Automation,
            "/api/simulate" => Self::Simulate,
            "/api/sources" => Self::Sources,
            "/api/sources/attach" => Self::Attach,
            "/api/camera/preview" => Self::CameraPreview,
            "/api/camera/analysis" => Self::CameraAnalysis,
            "/api/audio-sources" => Self::AudioSources,
            "/api/audio-sources/attach" => Self::AttachVoice,
            "/api/overlay" => Self::AddOverlay,
            "/api/setup" => Self::Setup,
            "/api/overlay-url" => Self::OverlayUrl,
            "/api/hello" => Self::Hello,
            "/api/ai" => Self::AiSettings,
            "/api/ai/plan" => Self::AiPlan,
            "/api/ai/analyze" => Self::AiAnalyze,
            "/api/events" => Self::Events,
            "/api/packs" => Self::Packs,
            "/api/card" => Self::Card,
            "/api/control/element" => Self::ControlElement,
            "/api/control/state" => Self::ControlState,
            "/api/control/events" => Self::ControlEvents,
            "/api/control/pairing" => Self::Pairing,
            "/api/control/pair" => Self::Pair,
            "/api/control/revoke" => Self::Revoke,
            "/api/room" => Self::Room,
            "/api/voice" => Self::Voice,
            "/api/voice/talk" => Self::VoiceTalk,
            "/api/voice/guide" => Self::VoiceGuide,
            "/api/voice/command" => Self::VoiceCommand,
            "/api/subtitles" => Self::Subtitles,
            "/api/subtitles/simulate" => Self::SubtitlesSimulate,
            _ if speech_id(path).is_some() => Self::Speech,
            _ => return path.strip_prefix("/assets/").and_then(assets::find).map(Self::File),
        };
        Some(endpoint)
    }

    pub(crate) fn methods(self) -> &'static [&'static str] {
        match self {
            Self::Project | Self::AiSettings => GET_PUT,
            Self::Room | Self::Pairing => GET_POST,
            Self::Override
            | Self::Release
            | Self::Effect
            | Self::Flood
            | Self::Automation
            | Self::Simulate
            | Self::Attach
            | Self::AttachVoice
            | Self::AddOverlay
            | Self::Setup
            | Self::AiPlan
            | Self::AiAnalyze
            | Self::Card
            | Self::ControlElement
            | Self::Pair
            | Self::Revoke
            | Self::VoiceTalk
            | Self::VoiceCommand
            | Self::SubtitlesSimulate => POST,
            Self::Root
            | Self::File(_)
            | Self::State
            | Self::Sources
            | Self::CameraPreview
            | Self::CameraAnalysis
            | Self::AudioSources
            | Self::OverlayUrl
            | Self::Hello
            | Self::Events
            | Self::Packs
            | Self::ControlState
            | Self::ControlEvents
            | Self::Voice
            | Self::VoiceGuide
            | Self::Speech
            | Self::Subtitles => GET,
        }
    }

    /// Whether a request with `access` may use this endpoint. Each token other than the
    /// editor's lists its endpoints here explicitly, so a new endpoint is refused to them
    /// until it is added.
    pub(crate) fn allows(self, access: Access) -> bool {
        if matches!(self, Self::Root | Self::File(_)) {
            // Pages and assets need no token and ignore any.
            return true;
        }
        match access {
            Access::Full => true,
            // A page asks for a hello before it sends a token, and pairing is how a control
            // surface gets one.
            Access::None => matches!(self, Self::Hello | Self::Pair),
            // The overlay page plays spoken cards, and subtitles are meant to be shown.
            Access::Overlay => matches!(self, Self::State | Self::Events | Self::Speech | Self::Subtitles),
            // The room endpoint lets a control token reveal and start the next round, and checks
            // the action itself. A control surface may hold push-to-talk, but never read what
            // was heard.
            Access::Control { .. } => matches!(
                self,
                Self::Card
                    | Self::ControlElement
                    | Self::ControlState
                    | Self::ControlEvents
                    | Self::Automation
                    | Self::Release
                    | Self::Effect
                    | Self::Flood
                    | Self::Room
                    | Self::VoiceTalk
            ),
        }
    }
}

/// The id in `/api/speech/<id>`: 64 lowercase hexadecimal digits.
pub(crate) fn speech_id(path: &str) -> Option<&str> {
    let id = path.strip_prefix("/api/speech/")?;
    let hex = |byte: u8| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte);
    (id.len() == 64 && id.bytes().all(hex)).then_some(id)
}
