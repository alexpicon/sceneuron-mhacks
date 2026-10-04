// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The source definition passed to `obs_register_source_s`.

use core::ffi::{c_char, c_int, c_void};

use crate::types::{
    audio_output_data, gs_effect_t, obs_audio_data, obs_data_t, obs_icon_type, obs_key_event, obs_media_state,
    obs_mouse_event, obs_properties_t, obs_source_audio_mix, obs_source_enum_proc_t, obs_source_frame, obs_source_t,
    obs_source_type,
};

/// The prefix of `struct obs_source_info` up to and including `version`.
///
/// Register it with `obs_register_source_s(&info, size_of::<obs_source_info>())`. libobs copies
/// `size` bytes into a zero-initialized struct of its own full size and checks optional callbacks
/// against `size`, so every later C field (`unversioned_id`, `missing_files`,
/// `video_get_color_space`, `filter_add`, the dark and light icons) reads as zero or NULL. The
/// prefix must still reach `version`: libobs reads `info->version` straight from the caller's
/// struct for its duplicate check before it looks at `size`, so a shorter struct would let it read
/// past the end of this one. A zero `version` keeps the id unversioned. libobs rejects a size
/// larger than its own struct, so the prefix stays valid for newer OBS releases that append fields.
///
/// `Default` is all zero, the C idiom `struct obs_source_info info = {0}`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct obs_source_info {
    pub id: *const c_char,
    /// `type` in C.
    pub type_: obs_source_type,
    pub output_flags: u32,
    pub get_name: Option<unsafe extern "C" fn(type_data: *mut c_void) -> *const c_char>,
    pub create: Option<unsafe extern "C" fn(settings: *mut obs_data_t, source: *mut obs_source_t) -> *mut c_void>,
    pub destroy: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub get_width: Option<unsafe extern "C" fn(data: *mut c_void) -> u32>,
    pub get_height: Option<unsafe extern "C" fn(data: *mut c_void) -> u32>,
    pub get_defaults: Option<unsafe extern "C" fn(settings: *mut obs_data_t)>,
    pub get_properties: Option<unsafe extern "C" fn(data: *mut c_void) -> *mut obs_properties_t>,
    pub update: Option<unsafe extern "C" fn(data: *mut c_void, settings: *mut obs_data_t)>,
    pub activate: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub deactivate: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub show: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub hide: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub video_tick: Option<unsafe extern "C" fn(data: *mut c_void, seconds: f32)>,
    pub video_render: Option<unsafe extern "C" fn(data: *mut c_void, effect: *mut gs_effect_t)>,
    pub filter_video:
        Option<unsafe extern "C" fn(data: *mut c_void, frame: *mut obs_source_frame) -> *mut obs_source_frame>,
    pub filter_audio:
        Option<unsafe extern "C" fn(data: *mut c_void, audio: *mut obs_audio_data) -> *mut obs_audio_data>,
    pub enum_active_sources:
        Option<unsafe extern "C" fn(data: *mut c_void, enum_callback: obs_source_enum_proc_t, param: *mut c_void)>,
    pub save: Option<unsafe extern "C" fn(data: *mut c_void, settings: *mut obs_data_t)>,
    pub load: Option<unsafe extern "C" fn(data: *mut c_void, settings: *mut obs_data_t)>,
    pub mouse_click: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            event: *const obs_mouse_event,
            type_: i32,
            mouse_up: bool,
            click_count: u32,
        ),
    >,
    pub mouse_move: Option<unsafe extern "C" fn(data: *mut c_void, event: *const obs_mouse_event, mouse_leave: bool)>,
    pub mouse_wheel:
        Option<unsafe extern "C" fn(data: *mut c_void, event: *const obs_mouse_event, x_delta: c_int, y_delta: c_int)>,
    pub focus: Option<unsafe extern "C" fn(data: *mut c_void, focus: bool)>,
    pub key_click: Option<unsafe extern "C" fn(data: *mut c_void, event: *const obs_key_event, key_up: bool)>,
    pub filter_remove: Option<unsafe extern "C" fn(data: *mut c_void, source: *mut obs_source_t)>,
    pub type_data: *mut c_void,
    pub free_type_data: Option<unsafe extern "C" fn(type_data: *mut c_void)>,
    pub audio_render: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            ts_out: *mut u64,
            audio_output: *mut obs_source_audio_mix,
            mixers: u32,
            channels: usize,
            sample_rate: usize,
        ) -> bool,
    >,
    pub enum_all_sources:
        Option<unsafe extern "C" fn(data: *mut c_void, enum_callback: obs_source_enum_proc_t, param: *mut c_void)>,
    pub transition_start: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub transition_stop: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub get_defaults2: Option<unsafe extern "C" fn(type_data: *mut c_void, settings: *mut obs_data_t)>,
    pub get_properties2:
        Option<unsafe extern "C" fn(data: *mut c_void, type_data: *mut c_void) -> *mut obs_properties_t>,
    pub audio_mix: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            ts_out: *mut u64,
            audio_output: *mut audio_output_data,
            channels: usize,
            sample_rate: usize,
        ) -> bool,
    >,
    pub icon_type: obs_icon_type,
    pub media_play_pause: Option<unsafe extern "C" fn(data: *mut c_void, pause: bool)>,
    pub media_restart: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub media_stop: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub media_next: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub media_previous: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub media_get_duration: Option<unsafe extern "C" fn(data: *mut c_void) -> i64>,
    pub media_get_time: Option<unsafe extern "C" fn(data: *mut c_void) -> i64>,
    pub media_set_time: Option<unsafe extern "C" fn(data: *mut c_void, milliseconds: i64)>,
    pub media_get_state: Option<unsafe extern "C" fn(data: *mut c_void) -> obs_media_state>,
    /// Non-zero registers the id as `<id>_v<version>`.
    pub version: u32,
}

impl Default for obs_source_info {
    fn default() -> Self {
        Self {
            id: core::ptr::null(),
            type_: 0,
            output_flags: 0,
            get_name: None,
            create: None,
            destroy: None,
            get_width: None,
            get_height: None,
            get_defaults: None,
            get_properties: None,
            update: None,
            activate: None,
            deactivate: None,
            show: None,
            hide: None,
            video_tick: None,
            video_render: None,
            filter_video: None,
            filter_audio: None,
            enum_active_sources: None,
            save: None,
            load: None,
            mouse_click: None,
            mouse_move: None,
            mouse_wheel: None,
            focus: None,
            key_click: None,
            filter_remove: None,
            type_data: core::ptr::null_mut(),
            free_type_data: None,
            audio_render: None,
            enum_all_sources: None,
            transition_start: None,
            transition_stop: None,
            get_defaults2: None,
            get_properties2: None,
            audio_mix: None,
            icon_type: 0,
            media_play_pause: None,
            media_restart: None,
            media_stop: None,
            media_next: None,
            media_previous: None,
            media_get_duration: None,
            media_get_time: None,
            media_set_time: None,
            media_get_state: None,
            version: 0,
        }
    }
}
