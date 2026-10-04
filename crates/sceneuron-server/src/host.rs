// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What the server needs from the application that embeds it.

use serde::Serialize;

use crate::preview::{CameraAnalysis, PreviewGap, PreviewImage};
use crate::setup::SetupReport;

const NOT_IN_OBS: &str = "Sceneuron is not running inside OBS";

/// A video source the user can choose for camera analysis.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceInfo {
    pub uuid: String,
    pub name: String,
    /// The source type id, such as `av_capture_input`.
    pub kind: String,
    /// The source already has the Sceneuron analysis filter.
    pub analyzed: bool,
}

/// A source that carries audio, which the user can choose as the microphone for voice control
/// and subtitles.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AudioSourceInfo {
    pub uuid: String,
    pub name: String,
    /// The source type id, such as `coreaudio_input_capture`.
    pub kind: String,
    /// One of OBS's global audio devices from Settings > Audio, such as Mic/Aux or Desktop Audio.
    pub global: bool,
    /// The source has the Sceneuron Voice filter.
    pub voice: bool,
    /// Its Sceneuron Voice filter is the one voice control and subtitles hear. Only one filter
    /// feeds them, so at most one source is in use.
    pub in_use: bool,
}

/// Application services behind the editor's camera, microphone, and scene controls.
///
/// The server calls these from its request thread, outside every hub lock.
pub trait Host: Send + Sync {
    /// Video sources that can be analyzed.
    fn list_sources(&self) -> Vec<SourceInfo>;

    /// Starts camera analysis on the source with this UUID.
    fn attach_filter(&self, uuid: &str) -> Result<(), String>;

    /// Sources that carry audio, OBS's global audio devices included; `None` when there are
    /// none to offer because Sceneuron is not running inside OBS.
    fn list_audio_sources(&self) -> Option<Vec<AudioSourceInfo>>;

    /// Makes the source with this UUID the microphone for voice control and subtitles by
    /// adding the Sceneuron Voice filter to it.
    fn attach_voice_filter(&self, uuid: &str) -> Result<(), String>;

    /// A JPEG of what camera analysis sees now, at most
    /// [`PREVIEW_MAX_SIDE`](crate::PREVIEW_MAX_SIDE) pixels on its longer side.
    ///
    /// Asking is what keeps preview frames coming: a host copies frames for the preview only
    /// for a couple of seconds after the last request. The picture goes only to the editor and
    /// is never kept.
    fn camera_preview(&self) -> Result<PreviewImage, PreviewGap>;

    /// What on-device analysis found lately, which a host keeps only while the preview is
    /// asked for, like its frames.
    fn camera_analysis(&self) -> CameraAnalysis;

    /// Adds an overlay source to the current scene and returns its name.
    fn add_overlay(&self) -> Result<String, String>;

    /// Puts what Sceneuron needs into the application, each part only when it is missing: an
    /// overlay in the current scene, camera analysis on a camera device, and the Sceneuron Voice
    /// filter on Mic/Aux; see [`camera_to_analyze`](crate::camera_to_analyze). The report says
    /// what was added and what was skipped; an error means nothing could be tried. The server
    /// pairs the Stream Deck itself, through the two calls below.
    fn set_up(&self) -> Result<SetupReport, String>;

    /// Whether Sceneuron's plugin for the Stream Deck app is installed on this computer, so a
    /// pairing link has a plugin to reach.
    fn stream_deck_plugin_installed(&self) -> bool;

    /// Opens a [`stream_deck_pairing_link`](crate::stream_deck_pairing_link) without bringing
    /// the Stream Deck app forward. The link carries a pairing code, so it must never be logged.
    fn open_stream_deck_link(&self, link: &str) -> Result<(), String>;

    /// Keeps a new control token, so that after a restart it works and the revoked one does
    /// not. Called when a client revokes the control token, before the server switches to
    /// `token`; an error keeps the old token in use.
    fn save_control_token(&self, token: &str) -> Result<(), String>;
}

/// The host outside OBS: there are no sources, microphones, camera pictures, or Stream Deck
/// plugin, scene changes are refused, and a new control token lasts as long as the server.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullHost;

impl Host for NullHost {
    fn list_sources(&self) -> Vec<SourceInfo> {
        Vec::new()
    }

    fn attach_filter(&self, _uuid: &str) -> Result<(), String> {
        Err(NOT_IN_OBS.to_owned())
    }

    fn list_audio_sources(&self) -> Option<Vec<AudioSourceInfo>> {
        None
    }

    fn attach_voice_filter(&self, _uuid: &str) -> Result<(), String> {
        Err(NOT_IN_OBS.to_owned())
    }

    fn camera_preview(&self) -> Result<PreviewImage, PreviewGap> {
        Err(PreviewGap::NotInObs)
    }

    fn camera_analysis(&self) -> CameraAnalysis {
        CameraAnalysis::default()
    }

    fn add_overlay(&self) -> Result<String, String> {
        Err(NOT_IN_OBS.to_owned())
    }

    fn set_up(&self) -> Result<SetupReport, String> {
        Err(NOT_IN_OBS.to_owned())
    }

    fn stream_deck_plugin_installed(&self) -> bool {
        false
    }

    fn open_stream_deck_link(&self, _link: &str) -> Result<(), String> {
        Err(NOT_IN_OBS.to_owned())
    }

    fn save_control_token(&self, _token: &str) -> Result<(), String> {
        Ok(())
    }
}
