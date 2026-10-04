// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Compile-time layout checks against the OBS 32.2.2 headers on 64-bit targets.
//!
//! The numbers were measured by compiling the real headers on macOS, where `long` is 8 bytes.

use core::ffi::c_void;
use core::mem::offset_of;

use crate::source_info::obs_source_info;
use crate::types::{
    calldata_t, obs_audio_data, obs_frontend_event, obs_hotkey_id, obs_icon_type, obs_media_state, obs_source_frame,
    obs_source_type, obs_text_info_type, obs_text_type, video_format,
};

const _: () = {
    // `sizeof(struct obs_source_info)` in OBS 32.2.2; libobs rejects a larger registration size.
    const OBS_SOURCE_INFO_C_SIZE: usize = 424;
    assert!(offset_of!(obs_source_info, filter_video) == 128);
    assert!(offset_of!(obs_source_info, type_data) == 216);
    assert!(offset_of!(obs_source_info, enum_all_sources) == 240);
    assert!(offset_of!(obs_source_info, get_properties2) == 272);
    assert!(offset_of!(obs_source_info, icon_type) == 288);
    assert!(offset_of!(obs_source_info, media_play_pause) == 296);
    assert!(offset_of!(obs_source_info, media_get_state) == 360);
    assert!(offset_of!(obs_source_info, version) == 368);
    assert!(size_of::<obs_source_info>() == 376);
    assert!(size_of::<obs_source_info>() <= OBS_SOURCE_INFO_C_SIZE);
    assert!(size_of::<Option<unsafe extern "C" fn()>>() == size_of::<*mut c_void>());
};

const _: () = {
    assert!(offset_of!(obs_source_frame, linesize) == 64);
    assert!(offset_of!(obs_source_frame, width) == 96);
    assert!(offset_of!(obs_source_frame, timestamp) == 104);
    assert!(offset_of!(obs_source_frame, format) == 112);
    assert!(offset_of!(obs_source_frame, full_range) == 180);
    assert!(offset_of!(obs_source_frame, flip) == 208);
    assert!(offset_of!(obs_source_frame, refs) == 216);
    assert!(size_of::<obs_source_frame>() == 232);
};

const _: () = {
    assert!(offset_of!(obs_audio_data, frames) == 64);
    assert!(offset_of!(obs_audio_data, timestamp) == 72);
    assert!(size_of::<obs_audio_data>() == 80);
};

const _: () = {
    assert!(offset_of!(calldata_t, fixed) == 24);
    assert!(size_of::<calldata_t>() == 32);
    assert!(size_of::<obs_hotkey_id>() == 8);
    assert!(size_of::<video_format>() == 4);
    assert!(size_of::<obs_source_type>() == 4);
    assert!(size_of::<obs_icon_type>() == 4);
    assert!(size_of::<obs_media_state>() == 4);
    assert!(size_of::<obs_text_type>() == 4);
    assert!(size_of::<obs_text_info_type>() == 4);
    assert!(size_of::<obs_frontend_event>() == 4);
};
