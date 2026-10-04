// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The zero `Default` values of the plain C structs, which mirror `{0}` initialization and
//! `calldata_init` in C.

use sceneuron_obs_sys::{MAX_AV_PLANES, calldata_t, obs_audio_data, obs_source_frame, obs_source_info};

#[test]
fn source_info_default_is_empty() {
    let info = obs_source_info::default();
    assert!(info.id.is_null());
    assert_eq!(info.type_, 0);
    assert_eq!(info.output_flags, 0);
    assert!(info.get_name.is_none());
    assert!(info.create.is_none());
    assert!(info.destroy.is_none());
    assert!(info.filter_video.is_none());
    assert!(info.video_render.is_none());
    assert!(info.type_data.is_null());
    assert!(info.free_type_data.is_none());
    assert!(info.get_properties2.is_none());
    assert!(info.audio_mix.is_none());
    assert_eq!(info.icon_type, 0);
    assert!(info.media_play_pause.is_none());
    assert!(info.media_get_state.is_none());
    assert_eq!(info.version, 0);
}

#[test]
fn source_frame_default_has_no_planes() {
    let frame = obs_source_frame::default();
    assert_eq!(frame.data, [std::ptr::null_mut(); MAX_AV_PLANES]);
    assert_eq!(frame.linesize, [0; MAX_AV_PLANES]);
    assert_eq!((frame.width, frame.height, frame.format), (0, 0, 0));
    assert!(!frame.full_range);
    assert_eq!(frame.refs, 0);
}

#[test]
fn audio_data_default_has_no_planes() {
    let audio = obs_audio_data::default();
    assert_eq!(audio.data, [std::ptr::null_mut(); MAX_AV_PLANES]);
    assert_eq!((audio.frames, audio.timestamp), (0, 0));
}

#[test]
fn calldata_default_matches_calldata_init() {
    let data = calldata_t::default();
    assert!(data.stack.is_null());
    assert_eq!((data.size, data.capacity), (0, 0));
    assert!(!data.fixed);
}
