// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The AI director inside OBS: settings in `ai.json`, results into the hub, hints from local
//! perception, and camera frames for the vision model.
//!
//! Frames reach the director only while scene understanding is on, at most about once a second,
//! converted at the size the streamer chose in the AI settings.

use std::sync::{Arc, Mutex, PoisonError};

use sceneuron_ai::{Assistant, Credentials, Frame, HubSink, default_connector, env_file_path, follow_hub};
use sceneuron_obs_sys as sys;
use sceneuron_server::Hub;
use sceneuron_vision::{InputSize, Perception, facts, novelty, to_rgb};

use crate::config::Paths;
use crate::ffi;
use crate::frame::frame_ref;
use crate::vision::VisionBridge;

/// Starts the assistant and its director, connected to the hub and to local perception.
///
/// Optional keys come from `.env` in the module config directory, or the file
/// `SCENEURON_ENV_FILE` names, and from the environment; see `sceneuron_ai::Credentials`.
pub fn start(paths: &Paths, hub: &Arc<Hub>, vision: &VisionBridge) -> Arc<Assistant> {
    let (credentials, mut warnings) = Credentials::load(Some(&env_file_path(&paths.env)));
    let sink = Arc::new(HubSink::new(Arc::clone(hub)));
    let (assistant, warning) = Assistant::new(paths.ai.clone(), default_connector(), sink, credentials);
    warnings.extend(warning);
    for warning in warnings {
        ffi::warn(&warning);
        hub.update(|engine, now| engine.note("system", &warning, now));
    }
    let assistant = Arc::new(assistant);
    follow_hub(hub, &assistant);
    let director = Arc::downgrade(&assistant);
    let clock = Arc::clone(hub);
    let previous: Mutex<Option<Perception>> = Mutex::new(None);
    vision.on_perception(move |perception| {
        let Some(assistant) = director.upgrade() else {
            return;
        };
        let mut previous = previous.lock().unwrap_or_else(PoisonError::into_inner);
        let novel = previous.as_ref().is_some_and(|before| novelty(before, perception));
        assistant.director().hint(facts(perception), novel, clock.now_ms());
        *previous = Some(perception.clone());
    });
    assistant
}

/// Hands the director a copy of `frame` when it wants one; called from the video thread.
pub fn offer_frame(assistant: &Assistant, frame: &sys::obs_source_frame, now_ms: u64) {
    if !assistant.director().wants_frame(now_ms) {
        return;
    }
    let side = assistant.settings_view().image_width;
    let size = InputSize::Fit {
        max_width: side,
        max_height: side,
    };
    let (width, height) = size.for_frame(frame.width, frame.height);
    let mut rgb = vec![0u8; width as usize * height as usize * 3];
    // SAFETY: the frame's planes stay valid for the duration of the filter callback.
    let converted = unsafe { frame_ref(frame) }
        .ok()
        .and_then(|view| to_rgb(&view, width, height, &mut rgb).ok());
    if converted.is_none() {
        return;
    }
    if frame.flip {
        let row = width as usize * 3;
        rgb = rgb.chunks_exact(row).rev().flatten().copied().collect();
    }
    assistant.director().offer_frame(Frame {
        rgb: rgb.into(),
        width,
        height,
        captured_ms: now_ms,
    });
}
