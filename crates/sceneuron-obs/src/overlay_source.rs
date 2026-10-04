// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Registration and libobs callbacks of the "Sceneuron Overlay" source.

use std::ffi::{c_char, c_void};
use std::ptr;

use sceneuron_obs_sys as sys;

use crate::ffi::guard;
use crate::overlay::Overlay;
use crate::properties::Properties;

pub const OVERLAY_ID: &std::ffi::CStr = c"sceneuron_overlay";

pub fn register() {
    // SAFETY: an all-zero obs_source_info is valid: null pointers and absent callbacks.
    let mut info: sys::obs_source_info = unsafe { std::mem::zeroed() };
    info.id = OVERLAY_ID.as_ptr();
    info.type_ = sys::OBS_SOURCE_TYPE_INPUT;
    // Audio carries spoken cards from the private browser source; see `overlay_audio`.
    info.output_flags = sys::OBS_SOURCE_VIDEO | sys::OBS_SOURCE_AUDIO | sys::OBS_SOURCE_CUSTOM_DRAW;
    info.icon_type = sys::OBS_ICON_TYPE_BROWSER;
    info.get_name = Some(get_name);
    info.create = Some(create);
    info.destroy = Some(destroy);
    info.update = Some(update);
    info.get_width = Some(get_width);
    info.get_height = Some(get_height);
    info.get_defaults = Some(get_defaults);
    info.get_properties = Some(get_properties);
    info.video_tick = Some(video_tick);
    info.video_render = Some(video_render);
    info.enum_active_sources = Some(enum_child);
    info.enum_all_sources = Some(enum_child);
    // SAFETY: libobs copies `info` during the call; the id is a 'static C string.
    unsafe { (sys::obs().obs_register_source_s)(&info, std::mem::size_of::<sys::obs_source_info>()) };
}

/// Borrows the overlay state libobs hands back to a callback.
///
/// # Safety
/// `data` must be null or the pointer returned by `create` and not yet destroyed.
unsafe fn overlay<'a>(data: *mut c_void) -> Option<&'a Overlay> {
    // SAFETY: per the function contract; the state is only ever shared, never mutably borrowed.
    unsafe { data.cast::<Overlay>().as_ref() }
}

unsafe extern "C" fn get_name(_type_data: *mut c_void) -> *const c_char {
    c"Sceneuron Overlay".as_ptr()
}

unsafe extern "C" fn create(settings: *mut sys::obs_data_t, source: *mut sys::obs_source_t) -> *mut c_void {
    guard(ptr::null_mut(), || Overlay::create(settings, source).cast::<c_void>())
}

unsafe extern "C" fn destroy(data: *mut c_void) {
    guard((), || {
        if !data.is_null() {
            // SAFETY: `data` came from Overlay::create and libobs destroys it exactly once.
            unsafe { Overlay::destroy(data.cast::<Overlay>()) };
        }
    })
}

unsafe extern "C" fn update(data: *mut c_void, settings: *mut sys::obs_data_t) {
    guard((), || {
        // SAFETY: libobs passes back the pointer from `create`.
        if let Some(overlay) = unsafe { overlay(data) } {
            overlay.read_size(settings);
            overlay.refresh_child(true);
            overlay.ensure_child();
        }
    })
}

unsafe extern "C" fn get_width(data: *mut c_void) -> u32 {
    // SAFETY: libobs passes back the pointer from `create`.
    guard(0, || unsafe { overlay(data) }.map_or(0, Overlay::width))
}

unsafe extern "C" fn get_height(data: *mut c_void) -> u32 {
    // SAFETY: libobs passes back the pointer from `create`.
    guard(0, || unsafe { overlay(data) }.map_or(0, Overlay::height))
}

unsafe extern "C" fn get_defaults(settings: *mut sys::obs_data_t) {
    guard((), || {
        // SAFETY: libobs passes a live settings object; keys are valid C strings.
        unsafe {
            (sys::obs().obs_data_set_default_int)(settings, c"width".as_ptr(), 1920);
            (sys::obs().obs_data_set_default_int)(settings, c"height".as_ptr(), 1080);
        }
    })
}

unsafe extern "C" fn get_properties(_data: *mut c_void) -> *mut sys::obs_properties_t {
    guard(ptr::null_mut(), || {
        let props = Properties::new();
        props.int("width", "Width", 16, 8192);
        props.int("height", "Height", 16, 8192);
        props.info(
            "about",
            "Shows the Sceneuron layout. Edit graphics and camera rules in the Sceneuron editor.",
        );
        props.open_editor_button();
        props.into_raw()
    })
}

unsafe extern "C" fn video_tick(data: *mut c_void, _seconds: f32) {
    guard((), || {
        // SAFETY: libobs passes back the pointer from `create`.
        if let Some(overlay) = unsafe { overlay(data) } {
            overlay.refresh_child(false);
        }
    })
}

unsafe extern "C" fn video_render(data: *mut c_void, _effect: *mut sys::gs_effect_t) {
    guard((), || {
        // SAFETY: libobs passes back the pointer from `create`; rendering happens on the graphics thread.
        if let Some(child) = unsafe { overlay(data) }.and_then(Overlay::child) {
            // SAFETY: live child source rendered inside the parent's render callback.
            unsafe { (sys::obs().obs_source_video_render)(child.as_ptr()) };
        }
    })
}

unsafe extern "C" fn enum_child(data: *mut c_void, callback: sys::obs_source_enum_proc_t, param: *mut c_void) {
    guard((), || {
        // SAFETY: libobs passes back the pointer from `create`.
        let Some(overlay) = (unsafe { overlay(data) }) else {
            return;
        };
        if let (Some(callback), Some(child)) = (callback, overlay.child()) {
            // SAFETY: libobs supplied the callback and parameter for this enumeration.
            unsafe { callback(overlay.source, child.as_ptr(), param) };
        }
    })
}
