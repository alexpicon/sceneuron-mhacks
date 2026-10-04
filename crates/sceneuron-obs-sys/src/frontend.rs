// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The OBS frontend API function table.

use core::ffi::{c_char, c_void};

use crate::table::function_table;
use crate::types::{obs_frontend_cb, obs_frontend_event_cb, obs_frontend_save_cb, obs_output_t, obs_source_t};

function_table! {
    /// The obs-frontend-api functions Sceneuron calls. Each field holds the C function of the same
    /// name, declared exactly as in the OBS 32.2.2 headers. Only the OBS application exports these;
    /// headless libobs hosts do not.
    pub struct Frontend {
        obs_frontend_add_tools_menu_item:
            unsafe extern "C" fn(name: *const c_char, callback: obs_frontend_cb, private_data: *mut c_void),
        obs_frontend_open_source_properties: unsafe extern "C" fn(source: *mut obs_source_t),
        obs_frontend_add_event_callback:
            unsafe extern "C" fn(callback: obs_frontend_event_cb, private_data: *mut c_void),
        obs_frontend_remove_event_callback:
            unsafe extern "C" fn(callback: obs_frontend_event_cb, private_data: *mut c_void),
        obs_frontend_add_save_callback: unsafe extern "C" fn(callback: obs_frontend_save_cb, private_data: *mut c_void),
        obs_frontend_remove_save_callback:
            unsafe extern "C" fn(callback: obs_frontend_save_cb, private_data: *mut c_void),
        obs_frontend_get_current_scene: unsafe extern "C" fn() -> *mut obs_source_t,
        // Returns a new reference to the streaming output, released with `obs_output_release`.
        obs_frontend_get_streaming_output: unsafe extern "C" fn() -> *mut obs_output_t,
        obs_frontend_streaming_active: unsafe extern "C" fn() -> bool,
        obs_frontend_recording_active: unsafe extern "C" fn() -> bool,
    }
}
