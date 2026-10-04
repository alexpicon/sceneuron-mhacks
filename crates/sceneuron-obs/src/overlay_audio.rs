// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Spoken cards in OBS's audio mix.
//!
//! The overlay's private browser source plays spoken cards with obs-browser's "Control audio
//! via OBS" (`reroute_audio`) on, so the audio goes to OBS instead of the computer's speakers.
//! A private source is in no scene, so OBS never mixes its audio; the overlay source outputs
//! every packet the browser produces as its own instead, and appears in the audio mixer.

use std::ffi::c_void;

use sceneuron_obs_sys as sys;

use crate::ffi::guard;

/// Starts passing the audio of `child` on as the audio of `parent`.
///
/// # Safety
/// Both sources must be live, and [`stop`] must be called with the same pair before `parent` is
/// destroyed or `child` is released.
pub unsafe fn start(parent: *mut sys::obs_source_t, child: *mut sys::obs_source_t) {
    // SAFETY: per the function contract; `parent` outlives the callback registration.
    unsafe { (sys::obs().obs_source_add_audio_capture_callback)(child, Some(forward), parent.cast()) };
}

/// Stops passing audio on. When it returns, the callback is not running and will not run again.
///
/// # Safety
/// Both sources must be live and passed to [`start`] before.
pub unsafe fn stop(parent: *mut sys::obs_source_t, child: *mut sys::obs_source_t) {
    // SAFETY: per the function contract; libobs removes the callback under the child's audio
    // callback lock, which the callback runs under.
    unsafe { (sys::obs().obs_source_remove_audio_capture_callback)(child, Some(forward), parent.cast()) };
}

/// Outputs one packet of the browser's audio as the overlay's.
unsafe extern "C" fn forward(
    param: *mut c_void,
    _source: *mut sys::obs_source_t,
    audio: *const sys::audio_data,
    muted: bool,
) {
    guard((), || {
        // SAFETY: libobs passes a packet that is valid for the duration of the callback.
        let Some(audio) = (unsafe { audio.as_ref() }) else {
            return;
        };
        let mut info = sys::obs_audio_info::default();
        // SAFETY: `info` is a valid, writable obs_audio_info.
        if muted || !unsafe { (sys::obs().obs_get_audio_info)(&mut info) } {
            return;
        }
        let packet = packet(audio, &info);
        // SAFETY: `param` is the live parent registered in `start`, which `stop` removes before the
        // parent goes away; the packet's planes stay valid during the call.
        unsafe { (sys::obs().obs_source_output_audio)(param.cast(), &packet) };
    })
}

/// The packet the overlay outputs for one the browser produced: the same planes, frame count,
/// and time, described in OBS's own format, which capture callbacks receive audio in.
pub fn packet(audio: &sys::audio_data, info: &sys::obs_audio_info) -> sys::obs_source_audio {
    sys::obs_source_audio {
        data: audio.data.map(<*mut u8>::cast_const),
        frames: audio.frames,
        speakers: info.speakers,
        format: sys::AUDIO_FORMAT_FLOAT_PLANAR,
        samples_per_sec: info.samples_per_sec,
        timestamp: audio.timestamp,
    }
}
