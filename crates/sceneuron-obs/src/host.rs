// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The OBS side of the editor's camera, microphone, and overlay actions, setting up OBS in one
//! click, reaching Sceneuron's Stream Deck plugin, and where a new control token is kept.
//!
//! Camera analysis follows one source: presence comes from one person in front of one camera,
//! and frames from two cameras would contradict each other. Analyzing a new source therefore
//! removes the analysis filter from every other source. The microphone works the same way (see
//! `audio_host`).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use sceneuron_ai::encode_jpeg;
use sceneuron_obs_sys as sys;
use sceneuron_server::{
    AudioSourceInfo, CameraAnalysis, Host, Hub, PREVIEW_MAX_SIDE, PreviewGap, PreviewImage, SetupReport, SetupStep,
    SourceInfo, camera_to_analyze,
};

use crate::audio_host;
use crate::config::SharedConfig;
use crate::ffi::{self, SourceRef, c_string};
use crate::filter::FILTER_ID;
use crate::open_url;
use crate::overlay_source::OVERLAY_ID;
use crate::preview::PreviewTap;
use crate::source_filters::{
    add_filter, for_each_live_source, for_each_source, has_filter, identity, is_type, remove_filters_except,
    source_by_uuid,
};
use crate::stream_deck;
use crate::ui;

/// Why "Set up OBS for me" analyzes no source: it starts analysis only on a camera device.
const NO_CAMERA: &str = "OBS has no camera device; add your camera as a Video Capture Device, or pick the source to \
                         analyze under Camera source below";

pub struct ObsHost {
    /// The plugin's settings, where a new control token is saved.
    pub config: Arc<SharedConfig>,
    /// The id [`SharedConfig::next_server`] gave the server this host serves.
    pub server: u64,
    /// The clock preview frames are stamped with.
    pub hub: Arc<Hub>,
    pub preview: Arc<PreviewTap>,
    /// The voice filter that feeds the voice input, by its id (see `voice`).
    pub voice_owner: Arc<AtomicUsize>,
}

impl Host for ObsHost {
    fn list_sources(&self) -> Vec<SourceInfo> {
        video_sources()
    }

    fn attach_filter(&self, uuid: &str) -> Result<(), String> {
        let uuid = uuid.to_owned();
        ui::run(move || attach_filter(&uuid)).unwrap_or_else(|| Err(NO_UI.to_owned()))
    }

    fn list_audio_sources(&self) -> Option<Vec<AudioSourceInfo>> {
        Some(audio_host::list(self.voice_owner.load(Ordering::SeqCst)))
    }

    fn attach_voice_filter(&self, uuid: &str) -> Result<(), String> {
        let uuid = uuid.to_owned();
        ui::run(move || audio_host::attach(&uuid)).unwrap_or_else(|| Err(NO_UI.to_owned()))
    }

    fn camera_preview(&self) -> Result<PreviewImage, PreviewGap> {
        let now = self.hub.now_ms();
        self.preview.request(now);
        let frame = self.preview.frame(now)?;
        // The tap holds only frames it built itself, which always encode.
        let jpeg = encode_jpeg(&frame, PREVIEW_MAX_SIDE).map_err(|_| PreviewGap::Stale)?;
        Ok(PreviewImage { jpeg, simulated: false })
    }

    fn camera_analysis(&self) -> CameraAnalysis {
        let now = self.hub.now_ms();
        self.preview.request(now);
        self.preview.analysis(now)
    }

    fn add_overlay(&self) -> Result<String, String> {
        ui::run(add_overlay).unwrap_or_else(|| Err(NO_UI.to_owned()))
    }

    fn set_up(&self) -> Result<SetupReport, String> {
        ui::run(set_up).ok_or_else(|| NO_UI.to_owned())
    }

    fn stream_deck_plugin_installed(&self) -> bool {
        stream_deck::plugin_installed()
    }

    fn open_stream_deck_link(&self, link: &str) -> Result<(), String> {
        open_url::open_stream_deck_link(link)
    }

    fn save_control_token(&self, token: &str) -> Result<(), String> {
        self.config.set_control_token(self.server, token)
    }
}

const NO_UI: &str = "the OBS window is not available";

fn attach_filter(uuid: &str) -> Result<(), String> {
    let source = source_by_uuid(uuid)?;
    let info = describe(source.as_ptr());
    if info.analyzed {
        return Ok(());
    }
    // SAFETY: valid source pointer.
    let flags = unsafe { (sys::obs().obs_source_get_output_flags)(source.as_ptr()) };
    if flags & sys::OBS_SOURCE_ASYNC == 0 {
        return Err(format!("{} does not provide camera frames", info.name));
    }
    add_filter(source.as_ptr(), FILTER_ID, c"Sceneuron Camera Analysis")?;
    remove_filters_except(source.as_ptr(), FILTER_ID);
    ffi::info(&format!("analyzing source {:?}", info.name));
    Ok(())
}

/// Video sources that can be analyzed: inputs with asynchronous video, such as cameras and media.
fn video_sources() -> Vec<SourceInfo> {
    let mut sources = Vec::new();
    for_each_live_source(|source| {
        // SAFETY: `source` is valid during enumeration.
        let flags = unsafe { (sys::obs().obs_source_get_output_flags)(source) };
        let async_video = sys::OBS_SOURCE_VIDEO | sys::OBS_SOURCE_ASYNC;
        if flags & async_video == async_video {
            sources.push(describe(source));
        }
    });
    sources
}

/// "Set up OBS for me" on the UI thread: each part is added only where it is missing.
fn set_up() -> SetupReport {
    let skipped = |error: String| SetupStep::skipped(&error);
    let overlay = match overlay_in_current_scene() {
        Ok(Some(name)) => SetupStep::present(&name),
        Ok(None) => add_overlay().map_or_else(skipped, |name| SetupStep::added(&name)),
        Err(error) => skipped(error),
    };
    let sources = video_sources();
    let camera = match sources.iter().find(|source| source.analyzed) {
        Some(source) => SetupStep::present(&source.name),
        None => match camera_to_analyze(&sources) {
            Some(source) => attach_filter(&source.uuid).map_or_else(skipped, |()| SetupStep::added(&source.name)),
            None => SetupStep::skipped(NO_CAMERA),
        },
    };
    SetupReport {
        overlay,
        camera,
        microphone: audio_host::set_up(),
    }
}

/// The current scene's source, which keeps the scene alive, and the scene.
fn current_scene() -> Result<(SourceRef, *mut sys::obs_scene_t), String> {
    let frontend = sys::frontend().ok_or(NO_UI)?;
    // SAFETY: returns an owned reference to the current scene source or null.
    let scene_source = unsafe { SourceRef::from_owned((frontend.obs_frontend_get_current_scene)()) }
        .ok_or("there is no current scene")?;
    // SAFETY: valid source; the scene pointer is borrowed from it.
    let scene = unsafe { (sys::obs().obs_scene_from_source)(scene_source.as_ptr()) };
    if scene.is_null() {
        return Err("the current scene could not be opened".to_owned());
    }
    Ok((scene_source, scene))
}

/// The name of a Sceneuron overlay in the current scene, if one is there: placed in it or in one
/// of its groups, even hidden, or shown on the program through a scene nested in it.
fn overlay_in_current_scene() -> Result<Option<String>, String> {
    let (_scene_source, scene) = current_scene()?;
    // Collected first and checked after, so no scene is locked while OBS holds its source list.
    let mut overlays = Vec::new();
    for_each_live_source(|source| {
        if is_type(source, OVERLAY_ID)
            // SAFETY: `source` is valid during enumeration; get_ref returns an owned reference.
            && let Some(overlay) = unsafe { SourceRef::from_owned((sys::obs().obs_source_get_ref)(source)) }
        {
            overlays.push(overlay);
        }
    });
    Ok(overlays.into_iter().find_map(|overlay| {
        let name = identity(overlay.as_ptr()).1;
        let c_name = c_string(&name);
        // SAFETY: the scene lives while its source is held; the item returned is only compared
        // with null, never used.
        let placed = !unsafe { (sys::obs().obs_scene_find_source_recursive)(scene, c_name.as_ptr()) }.is_null();
        // SAFETY: a live source, held by `overlay`.
        let shown = unsafe { (sys::obs().obs_source_active)(overlay.as_ptr()) };
        (placed || shown).then_some(name)
    }))
}

fn add_overlay() -> Result<String, String> {
    let (_scene_source, scene) = current_scene()?;
    let name = unique_overlay_name();
    let c_name = c_string(&name);
    // SAFETY: registered source id and valid name; returns an owned reference or null.
    let overlay = unsafe {
        SourceRef::from_owned((sys::obs().obs_source_create)(
            OVERLAY_ID.as_ptr(),
            c_name.as_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        ))
    }
    .ok_or("could not create the overlay source")?;
    // SAFETY: live scene and source; the scene keeps its own reference.
    unsafe { (sys::obs().obs_scene_add)(scene, overlay.as_ptr()) };
    Ok(name)
}

fn describe(source: *mut sys::obs_source_t) -> SourceInfo {
    let (uuid, name, kind) = identity(source);
    SourceInfo {
        uuid,
        name,
        kind,
        analyzed: has_filter(source, FILTER_ID),
    }
}

fn unique_overlay_name() -> String {
    let mut taken = Vec::new();
    // Removed sources count too: one keeps its name until OBS releases it.
    for_each_source(|source| taken.push(identity(source).1));
    let base = "Sceneuron Overlay";
    (1..)
        .map(|n| if n == 1 { base.to_owned() } else { format!("{base} {n}") })
        .find(|name| !taken.contains(name))
        .unwrap_or_else(|| base.to_owned())
}
