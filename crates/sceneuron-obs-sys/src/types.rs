// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Handle types, enum aliases, callback types, and the plain C structs other than
//! `obs_source_info`.

use core::ffi::{c_long, c_uint, c_void};
use core::marker::{PhantomData, PhantomPinned};

macro_rules! opaque_types {
    ($($(#[$meta:meta])* $name:ident;)+) => {
        $(
            $(#[$meta])*
            #[repr(C)]
            pub struct $name {
                _data: [u8; 0],
                _marker: PhantomData<(*mut u8, PhantomPinned)>,
            }
        )+
    };
}

opaque_types! {
    /// `obs_source_t`: an input, filter, transition, or scene.
    obs_source_t;
    /// `obs_weak_source_t`: a weak reference that does not keep a source alive.
    obs_weak_source_t;
    /// `obs_data_t`: a reference-counted settings object.
    obs_data_t;
    /// `obs_data_array_t`: a reference-counted array of settings objects.
    obs_data_array_t;
    /// `obs_properties_t`: a property list shown in a properties dialog.
    obs_properties_t;
    /// `obs_property_t`: one entry of an `obs_properties_t`.
    obs_property_t;
    /// `obs_scene_t`: the scene behind a scene source.
    obs_scene_t;
    /// `obs_sceneitem_t`: a source placed in a scene.
    obs_sceneitem_t;
    /// `obs_module_t`: a loaded OBS module.
    obs_module_t;
    /// `obs_output_t`: an output, such as the stream, that encodes and sends video and audio.
    obs_output_t;
    /// `proc_handler_t`: a named procedure table.
    proc_handler_t;
    /// `gs_effect_t`: a graphics effect.
    gs_effect_t;
    /// `obs_hotkey_t`: a registered hotkey.
    obs_hotkey_t;
    /// `audio_t` (`struct audio_output`): the audio output OBS mixes into.
    audio_t;
    /// `struct obs_mouse_event`; complete in C, opaque here because it is only passed through.
    obs_mouse_event;
    /// `struct obs_key_event`; complete in C, opaque here because it is only passed through.
    obs_key_event;
    /// `struct obs_source_audio_mix`; complete in C, opaque here because it is only passed through.
    obs_source_audio_mix;
    /// `struct audio_output_data`; complete in C, opaque here because it is only passed through.
    audio_output_data;
}

// C enums are modelled as integer aliases, not Rust enums: a value added by a newer OBS would be
// undefined behavior in a Rust enum. Every enum here is 4 bytes with non-negative values.

/// `enum obs_source_type`; see the `OBS_SOURCE_TYPE_*` constants.
pub type obs_source_type = c_uint;
/// `enum obs_icon_type`; see the `OBS_ICON_TYPE_*` constants.
pub type obs_icon_type = c_uint;
/// `enum obs_media_state`, returned by the `media_get_state` callback.
pub type obs_media_state = c_uint;
/// `enum video_format`; see the `VIDEO_FORMAT_*` constants.
pub type video_format = c_uint;
/// `enum obs_text_type`; see the `OBS_TEXT_*` constants.
pub type obs_text_type = c_uint;
/// `enum obs_text_info_type`; see the `OBS_TEXT_INFO_*` constants.
pub type obs_text_info_type = c_uint;
/// `enum obs_frontend_event`; see the `OBS_FRONTEND_EVENT_*` constants.
pub type obs_frontend_event = c_uint;

/// `obs_hotkey_id` (`size_t`).
pub type obs_hotkey_id = usize;

/// `enum obs_task_type`
pub type obs_task_type = c_uint;

/// `obs_task_t`
pub type obs_task_t = Option<unsafe extern "C" fn(param: *mut c_void)>;

/// `MAX_AV_PLANES`: the number of plane pointers in a frame.
pub const MAX_AV_PLANES: usize = 8;

/// `obs_source_enum_proc_t`
pub type obs_source_enum_proc_t =
    Option<unsafe extern "C" fn(parent: *mut obs_source_t, child: *mut obs_source_t, param: *mut c_void)>;

/// `obs_hotkey_func`
pub type obs_hotkey_func =
    Option<unsafe extern "C" fn(data: *mut c_void, id: obs_hotkey_id, hotkey: *mut obs_hotkey_t, pressed: bool)>;

/// `obs_property_clicked_t`
pub type obs_property_clicked_t = Option<
    unsafe extern "C" fn(props: *mut obs_properties_t, property: *mut obs_property_t, data: *mut c_void) -> bool,
>;

/// `obs_frontend_cb`
pub type obs_frontend_cb = Option<unsafe extern "C" fn(private_data: *mut c_void)>;

/// `obs_frontend_event_cb`
pub type obs_frontend_event_cb = Option<unsafe extern "C" fn(event: obs_frontend_event, private_data: *mut c_void)>;

/// `obs_frontend_save_cb`
pub type obs_frontend_save_cb =
    Option<unsafe extern "C" fn(save_data: *mut obs_data_t, saving: bool, private_data: *mut c_void)>;

/// `calldata_t` (`struct calldata`), the argument block of `proc_handler_call`.
///
/// `Default` matches the C `calldata_init` (all zero, heap-backed). A heap-backed block owns
/// `stack` once libobs has grown it; release it with `bfree(stack)` as `calldata_free` does.
#[repr(C)]
#[derive(Debug)]
pub struct calldata_t {
    pub stack: *mut u8,
    pub size: usize,
    pub capacity: usize,
    pub fixed: bool,
}

impl Default for calldata_t {
    fn default() -> Self {
        Self {
            stack: core::ptr::null_mut(),
            size: 0,
            capacity: 0,
            fixed: false,
        }
    }
}

/// `struct obs_audio_data`, a block of audio as an audio filter sees it: one plane of `frames`
/// float samples per channel of the OBS audio output, in its sample rate.
#[repr(C)]
#[derive(Debug)]
pub struct obs_audio_data {
    pub data: [*mut u8; MAX_AV_PLANES],
    pub frames: u32,
    pub timestamp: u64,
}

impl Default for obs_audio_data {
    fn default() -> Self {
        Self {
            data: [core::ptr::null_mut(); MAX_AV_PLANES],
            frames: 0,
            timestamp: 0,
        }
    }
}

/// `struct obs_source_frame`, an asynchronous video frame as seen by `filter_video`.
///
/// `refs` is a C `long`, 8 bytes on macOS; the layout assertions pin the tail of the struct.
#[repr(C)]
#[derive(Debug)]
pub struct obs_source_frame {
    pub data: [*mut u8; MAX_AV_PLANES],
    pub linesize: [u32; MAX_AV_PLANES],
    pub width: u32,
    pub height: u32,
    pub timestamp: u64,
    pub format: video_format,
    pub color_matrix: [f32; 16],
    pub full_range: bool,
    pub max_luminance: u16,
    pub color_range_min: [f32; 3],
    pub color_range_max: [f32; 3],
    pub flip: bool,
    pub flags: u8,
    /// `enum video_trc`, stored as `uint8_t` in C.
    pub trc: u8,
    /// Reference count owned by libobs (`volatile long`).
    pub refs: c_long,
    /// Used internally by libobs.
    pub prev_frame: bool,
}

impl Default for obs_source_frame {
    fn default() -> Self {
        Self {
            data: [core::ptr::null_mut(); MAX_AV_PLANES],
            linesize: [0; MAX_AV_PLANES],
            width: 0,
            height: 0,
            timestamp: 0,
            format: 0,
            color_matrix: [0.0; 16],
            full_range: false,
            max_luminance: 0,
            color_range_min: [0.0; 3],
            color_range_max: [0.0; 3],
            flip: false,
            flags: 0,
            trc: 0,
            refs: 0,
            prev_frame: false,
        }
    }
}
