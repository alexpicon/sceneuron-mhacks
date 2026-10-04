// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Struct layouts on 64-bit targets, re-checked at runtime as executable documentation of the ABI
//! the plugin relies on. The numbers come from compiling the OBS 32.2.2 headers; the crate also
//! asserts the key ones at compile time.

#![cfg(target_pointer_width = "64")]

use std::mem::offset_of;

use sceneuron_obs_sys::{
    calldata_t, obs_audio_data, obs_frontend_event, obs_hotkey_id, obs_icon_type, obs_media_state, obs_source_frame,
    obs_source_info, obs_source_type, obs_text_info_type, obs_text_type, video_format,
};

/// `sizeof(struct obs_source_info)` in OBS 32.2.2.
const OBS_SOURCE_INFO_C_SIZE: usize = 424;

#[test]
fn source_info_prefix_matches_the_c_struct_through_version() {
    assert_eq!(offset_of!(obs_source_info, id), 0);
    assert_eq!(offset_of!(obs_source_info, type_), 8);
    assert_eq!(offset_of!(obs_source_info, output_flags), 12);
    assert_eq!(offset_of!(obs_source_info, get_name), 16);
    assert_eq!(offset_of!(obs_source_info, create), 24);
    assert_eq!(offset_of!(obs_source_info, video_render), 120);
    assert_eq!(offset_of!(obs_source_info, filter_video), 128);
    assert_eq!(offset_of!(obs_source_info, type_data), 216);
    assert_eq!(offset_of!(obs_source_info, audio_render), 232);
    assert_eq!(offset_of!(obs_source_info, enum_all_sources), 240);
    assert_eq!(offset_of!(obs_source_info, get_defaults2), 264);
    assert_eq!(offset_of!(obs_source_info, get_properties2), 272);
    assert_eq!(offset_of!(obs_source_info, audio_mix), 280);
    assert_eq!(offset_of!(obs_source_info, icon_type), 288);
    assert_eq!(offset_of!(obs_source_info, media_play_pause), 296);
    assert_eq!(offset_of!(obs_source_info, media_set_time), 352);
    assert_eq!(offset_of!(obs_source_info, media_get_state), 360);
    assert_eq!(offset_of!(obs_source_info, version), 368);
}

#[test]
fn source_info_prefix_size_is_what_registration_passes_to_libobs() {
    assert_eq!(size_of::<obs_source_info>(), 376);
    assert!(size_of::<obs_source_info>() <= OBS_SOURCE_INFO_C_SIZE);
}

/// `obs_register_source_s` reads `info->version` from the caller's struct before honoring `size`.
#[test]
fn source_info_prefix_covers_the_field_libobs_reads_regardless_of_size() {
    let version_end = offset_of!(obs_source_info, version) + size_of::<u32>();
    assert!(version_end <= size_of::<obs_source_info>());
}

#[test]
fn audio_data_matches_the_c_struct() {
    assert_eq!(offset_of!(obs_audio_data, data), 0);
    assert_eq!(offset_of!(obs_audio_data, frames), 64);
    assert_eq!(offset_of!(obs_audio_data, timestamp), 72);
    assert_eq!(size_of::<obs_audio_data>(), 80);
}

#[test]
fn source_frame_matches_the_c_struct() {
    assert_eq!(offset_of!(obs_source_frame, data), 0);
    assert_eq!(offset_of!(obs_source_frame, linesize), 64);
    assert_eq!(offset_of!(obs_source_frame, width), 96);
    assert_eq!(offset_of!(obs_source_frame, height), 100);
    assert_eq!(offset_of!(obs_source_frame, timestamp), 104);
    assert_eq!(offset_of!(obs_source_frame, format), 112);
    assert_eq!(offset_of!(obs_source_frame, color_matrix), 116);
    assert_eq!(offset_of!(obs_source_frame, full_range), 180);
    assert_eq!(offset_of!(obs_source_frame, max_luminance), 182);
    assert_eq!(offset_of!(obs_source_frame, color_range_min), 184);
    assert_eq!(offset_of!(obs_source_frame, color_range_max), 196);
    assert_eq!(offset_of!(obs_source_frame, flip), 208);
    assert_eq!(offset_of!(obs_source_frame, flags), 209);
    assert_eq!(offset_of!(obs_source_frame, trc), 210);
}

#[test]
fn source_frame_tail_follows_lp64_long() {
    assert_eq!(offset_of!(obs_source_frame, refs), 216);
    assert_eq!(offset_of!(obs_source_frame, prev_frame), 224);
    assert_eq!(size_of::<obs_source_frame>(), 232);
}

#[test]
fn calldata_matches_the_c_struct() {
    assert_eq!(offset_of!(calldata_t, stack), 0);
    assert_eq!(offset_of!(calldata_t, size), 8);
    assert_eq!(offset_of!(calldata_t, capacity), 16);
    assert_eq!(offset_of!(calldata_t, fixed), 24);
    assert_eq!(size_of::<calldata_t>(), 32);
}

#[test]
fn enums_and_hotkey_ids_have_their_c_sizes() {
    assert_eq!(size_of::<video_format>(), 4);
    assert_eq!(size_of::<obs_source_type>(), 4);
    assert_eq!(size_of::<obs_icon_type>(), 4);
    assert_eq!(size_of::<obs_media_state>(), 4);
    assert_eq!(size_of::<obs_text_type>(), 4);
    assert_eq!(size_of::<obs_text_info_type>(), 4);
    assert_eq!(size_of::<obs_frontend_event>(), 4);
    assert_eq!(size_of::<obs_hotkey_id>(), 8);
}
