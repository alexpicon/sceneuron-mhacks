// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises how the overlay passes the browser's audio on: each packet keeps its planes, frame
//! count, and time, and is described in OBS's float planar format at OBS's rate and layout.

use sceneuron::overlay_audio::packet;
use sceneuron_obs_sys as sys;

#[test]
fn a_packet_keeps_its_planes_frames_and_time_in_obs_format() {
    let mut left = vec![0.25f32; 480];
    let mut right = vec![-0.25f32; 480];
    let mut planes = [std::ptr::null_mut::<u8>(); sys::MAX_AV_PLANES];
    planes[0] = left.as_mut_ptr().cast();
    planes[1] = right.as_mut_ptr().cast();
    let audio = sys::audio_data {
        data: planes,
        frames: 480,
        timestamp: 123_456_789,
    };
    let info = sys::obs_audio_info {
        samples_per_sec: 48_000,
        speakers: sys::SPEAKERS_STEREO,
    };
    let out = packet(&audio, &info);
    assert_eq!(out.data[0], left.as_ptr().cast::<u8>());
    assert_eq!(out.data[1], right.as_ptr().cast::<u8>());
    assert!(out.data[2..].iter().all(|plane| plane.is_null()));
    assert_eq!((out.frames, out.timestamp), (480, 123_456_789));
    assert_eq!((out.samples_per_sec, out.speakers), (48_000, sys::SPEAKERS_STEREO));
    assert_eq!(out.format, sys::AUDIO_FORMAT_FLOAT_PLANAR);
}
