// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The "Sceneuron Voice" audio filter: hands a microphone's audio to the voice input while it
//! wants audio, and passes the audio on unchanged.
//!
//! `filter_audio` runs on OBS's audio thread. It reads atomic flags and, only while voice
//! control wants audio, mixes the block to mono straight into a ring allocated once: no lock,
//! allocation, or system call. Conversion and the network happen on the voice thread.

use std::ffi::{CStr, c_char, c_void};
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use sceneuron_obs_sys as sys;
use sceneuron_voice::VoiceInput;

use crate::audio_block::planes;
use crate::ffi::guard;
use crate::plugin;
use crate::properties;
use crate::voice::claim;

pub const FILTER_ID: &CStr = c"sceneuron_voice_filter";

struct VoiceFilter {
    input: Option<(VoiceInput, Arc<AtomicUsize>)>,
    channels: usize,
    sample_rate: u32,
}

pub fn register() {
    // SAFETY: an all-zero obs_source_info is valid: null pointers and absent callbacks.
    let mut info: sys::obs_source_info = unsafe { std::mem::zeroed() };
    info.id = FILTER_ID.as_ptr();
    info.type_ = sys::OBS_SOURCE_TYPE_FILTER;
    info.output_flags = sys::OBS_SOURCE_AUDIO;
    info.get_name = Some(get_name);
    info.create = Some(create);
    info.destroy = Some(destroy);
    info.filter_audio = Some(filter_audio);
    info.get_properties = Some(get_properties);
    // SAFETY: libobs copies `info` during the call; the id is a 'static C string.
    unsafe { (sys::obs().obs_register_source_s)(&info, std::mem::size_of::<sys::obs_source_info>()) };
}

unsafe extern "C" fn get_name(_type_data: *mut c_void) -> *const c_char {
    c"Sceneuron Voice".as_ptr()
}

unsafe extern "C" fn create(_settings: *mut sys::obs_data_t, _source: *mut sys::obs_source_t) -> *mut c_void {
    guard(ptr::null_mut(), || {
        let obs = sys::obs();
        // SAFETY: obs_get_audio has no preconditions; the getters only read the output it
        // returns, which lives as long as OBS's audio subsystem.
        let (channels, sample_rate) = unsafe {
            let audio = (obs.obs_get_audio)();
            if audio.is_null() {
                (0, 0)
            } else {
                (
                    (obs.audio_output_get_channels)(audio),
                    (obs.audio_output_get_sample_rate)(audio),
                )
            }
        };
        let filter = VoiceFilter {
            input: plugin::current().map(|plugin| plugin.voice.input(&plugin.ai)),
            channels,
            sample_rate,
        };
        Box::into_raw(Box::new(filter)).cast::<c_void>()
    })
}

unsafe extern "C" fn destroy(data: *mut c_void) {
    guard((), || {
        if data.is_null() {
            return;
        }
        // SAFETY: `data` came from Box::into_raw in `create` and libobs destroys it once.
        drop(unsafe { Box::from_raw(data.cast::<VoiceFilter>()) });
        if let Some(plugin) = plugin::current() {
            plugin.voice.filter_destroyed(data as usize);
        }
    })
}

unsafe extern "C" fn filter_audio(data: *mut c_void, audio: *mut sys::obs_audio_data) -> *mut sys::obs_audio_data {
    guard(audio, || {
        if data.is_null() || audio.is_null() {
            return audio;
        }
        // SAFETY: libobs passes back the pointer `create` returned, on one thread at a time.
        let filter = unsafe { &*data.cast::<VoiceFilter>() };
        let Some((input, owner)) = &filter.input else {
            return audio;
        };
        // The filter's address is its id while it lives.
        if !input.wants_audio() || filter.sample_rate == 0 || !claim(owner, data as usize) {
            return audio;
        }
        // SAFETY: libobs guarantees the block and its planes are valid during the callback.
        if let Some(planes) = unsafe { planes(&*audio, filter.channels) } {
            input.feed_planar(planes.as_slice(), filter.sample_rate);
        }
        audio
    })
}

unsafe extern "C" fn get_properties(_data: *mut c_void) -> *mut sys::obs_properties_t {
    guard(ptr::null_mut(), || {
        let props = properties::Properties::new();
        props.info(
            "about",
            "Sceneuron listens to this microphone for voice control, and only after you turn \
             voice control on and agree in the editor to send audio to ElevenLabs speech to text. \
             In push-to-talk mode audio goes only while you hold the \"Sceneuron: Push to talk\" \
             hotkey; in always-on mode it goes while voice control is on. With it go the wake word \
             and the names and trigger phrases of cards that voice can raise, as keyterms that \
             help recognition. The audio itself passes through unchanged. Sceneuron hears one \
             microphone at a time; choosing one in the editor's Camera & mic tab moves this filter \
             there.",
        );
        props.open_editor_button();
        props.into_raw()
    })
}
