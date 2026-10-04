// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! OBS's program for the overlay: the current scene's name and whether OBS streams or records,
//! read from the frontend whenever one of them may have changed, for `{{on-air}}` and
//! `{{on-air-time}}`.

use sceneuron_obs_sys as sys;

use crate::ffi::{SourceRef, from_c};
use crate::plugin;

/// Whether a frontend event may change the scene or whether OBS is on air.
pub fn concerns(event: sys::obs_frontend_event) -> bool {
    matches!(
        event,
        sys::OBS_FRONTEND_EVENT_FINISHED_LOADING
            | sys::OBS_FRONTEND_EVENT_SCENE_CHANGED
            | sys::OBS_FRONTEND_EVENT_SCENE_LIST_CHANGED
            | sys::OBS_FRONTEND_EVENT_SCENE_COLLECTION_CHANGED
            | sys::OBS_FRONTEND_EVENT_STREAMING_STARTED
            | sys::OBS_FRONTEND_EVENT_STREAMING_STOPPED
            | sys::OBS_FRONTEND_EVENT_RECORDING_STARTED
            | sys::OBS_FRONTEND_EVENT_RECORDING_STOPPED
    )
}

/// Reads the program from the frontend and gives it to the engine. Called on OBS's UI thread,
/// from frontend events.
pub fn report() {
    let (Some(frontend), Some(plugin)) = (sys::frontend(), plugin::current()) else {
        return;
    };
    // SAFETY: returns an owned reference to the current scene source, or null.
    let scene = unsafe { SourceRef::from_owned((frontend.obs_frontend_get_current_scene)()) };
    // SAFETY: the source is live while `scene` holds it; the name is copied before it is released.
    let name = scene.map_or_else(String::new, |scene| unsafe {
        from_c((sys::obs().obs_source_get_name)(scene.as_ptr()))
    });
    // SAFETY: frontend functions without arguments, called on the UI thread.
    let (streaming, recording) = unsafe {
        (
            (frontend.obs_frontend_streaming_active)(),
            (frontend.obs_frontend_recording_active)(),
        )
    };
    plugin
        .hub
        .update(|engine, now| engine.set_on_air(&name, streaming, recording, now));
}
