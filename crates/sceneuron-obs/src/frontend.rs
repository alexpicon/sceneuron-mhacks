// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Frontend integration: the save callback for hotkey bindings, OBS's program for the overlay, and
//! exit handling.

use std::ffi::c_void;
use std::ptr;

use sceneuron_obs_sys as sys;

use crate::ffi::guard;
use crate::{caption_output, hotkeys, on_air, overlay, plugin, settings};

pub fn connect() {
    let Some(frontend) = sys::frontend() else {
        return;
    };
    // SAFETY: callbacks match the frontend signatures and stay valid for the process lifetime.
    unsafe {
        (frontend.obs_frontend_add_save_callback)(Some(on_save), ptr::null_mut());
        (frontend.obs_frontend_add_event_callback)(Some(on_event), ptr::null_mut());
    }
}

unsafe extern "C" fn on_save(save_data: *mut sys::obs_data_t, saving: bool, _data: *mut c_void) {
    guard((), || {
        if !save_data.is_null() {
            hotkeys::save_or_load(save_data, saving);
        }
    })
}

unsafe extern "C" fn on_event(event: sys::obs_frontend_event, _data: *mut c_void) {
    guard((), || {
        if on_air::concerns(event) {
            on_air::report();
        }
        // Overlay browsers must close while OBS still pumps events (before it clears the scenes),
        // or obs-browser shuts CEF down with a browser still closing and OBS crashes or hangs.
        if event == sys::OBS_FRONTEND_EVENT_SCRIPTING_SHUTDOWN {
            overlay::release_browsers(true);
        }
        if event == sys::OBS_FRONTEND_EVENT_SCENE_COLLECTION_CLEANUP {
            overlay::release_browsers(false);
        }
        if event == sys::OBS_FRONTEND_EVENT_EXIT {
            // Stream captions look up the streaming output, which the frontend tears down now.
            caption_output::stop();
            settings::release();
            if let Some(plugin) = plugin::current() {
                // A failure reaches the log through the SaveFailed listener, and the final
                // save at unload tries again.
                let _ = plugin.hub.flush();
            }
        }
    })
}
