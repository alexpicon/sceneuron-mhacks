// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Audio structs, enum aliases, and the capture callback type from media-io/audio-io.h and
//! obs.h, with their layouts pinned on 64-bit targets.

use core::ffi::{c_uint, c_void};

use crate::types::{MAX_AV_PLANES, obs_source_t};

/// `enum speaker_layout`; see the `SPEAKERS_*` constants.
pub type speaker_layout = c_uint;
/// `enum audio_format`; see the `AUDIO_FORMAT_*` constants.
pub type audio_format = c_uint;

pub const SPEAKERS_UNKNOWN: speaker_layout = 0;
pub const SPEAKERS_MONO: speaker_layout = 1;
pub const SPEAKERS_STEREO: speaker_layout = 2;
pub const SPEAKERS_7POINT1: speaker_layout = 8;

pub const AUDIO_FORMAT_UNKNOWN: audio_format = 0;
pub const AUDIO_FORMAT_FLOAT: audio_format = 4;
pub const AUDIO_FORMAT_FLOAT_PLANAR: audio_format = 8;

/// `struct audio_data`: audio in OBS's own format, one plane per channel.
#[repr(C)]
#[derive(Debug)]
pub struct audio_data {
    pub data: [*mut u8; MAX_AV_PLANES],
    pub frames: u32,
    pub timestamp: u64,
}

/// `struct obs_source_audio`, the argument of `obs_source_output_audio`.
#[repr(C)]
#[derive(Debug)]
pub struct obs_source_audio {
    pub data: [*const u8; MAX_AV_PLANES],
    pub frames: u32,
    pub speakers: speaker_layout,
    pub format: audio_format,
    pub samples_per_sec: u32,
    pub timestamp: u64,
}

/// `struct obs_audio_info`: OBS's output sample rate and speaker layout.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct obs_audio_info {
    pub samples_per_sec: u32,
    pub speakers: speaker_layout,
}

/// `obs_source_audio_capture_t`
pub type obs_source_audio_capture_t = Option<
    unsafe extern "C" fn(param: *mut c_void, source: *mut obs_source_t, audio_data: *const audio_data, muted: bool),
>;

#[cfg(target_pointer_width = "64")]
const _: () = {
    use core::mem::offset_of;
    assert!(offset_of!(audio_data, frames) == 64);
    assert!(offset_of!(audio_data, timestamp) == 72);
    assert!(size_of::<audio_data>() == 80);
    assert!(offset_of!(obs_source_audio, frames) == 64);
    assert!(offset_of!(obs_source_audio, speakers) == 68);
    assert!(offset_of!(obs_source_audio, format) == 72);
    assert!(offset_of!(obs_source_audio, samples_per_sec) == 76);
    assert!(offset_of!(obs_source_audio, timestamp) == 80);
    assert!(size_of::<obs_source_audio>() == 88);
    assert!(size_of::<obs_audio_info>() == 8);
};
