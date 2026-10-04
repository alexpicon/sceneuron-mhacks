// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The libobs function table.

use core::ffi::{c_char, c_int, c_longlong, c_void};

use crate::audio::{obs_audio_info, obs_source_audio, obs_source_audio_capture_t};
use crate::source_info::obs_source_info;
use crate::table::function_table;
use crate::types::{
    audio_t, calldata_t, obs_data_array_t, obs_data_t, obs_hotkey_func, obs_hotkey_id, obs_module_t, obs_output_t,
    obs_properties_t, obs_property_clicked_t, obs_property_t, obs_scene_t, obs_sceneitem_t, obs_source_enum_proc_t,
    obs_source_t, obs_source_type, obs_task_t, obs_task_type, obs_text_info_type, obs_text_type, obs_weak_source_t,
    proc_handler_t,
};

function_table! {
    /// The libobs functions Sceneuron calls. Each field holds the C function of the same name,
    /// declared exactly as in the OBS 32.2.2 headers.
    pub struct Libobs {
        // util/base.h, util/bmem.h
        blog: unsafe extern "C" fn(log_level: c_int, format: *const c_char, ...),
        bfree: unsafe extern "C" fn(ptr: *mut c_void),

        // obs.h: core, modules, and source registration
        obs_get_version_string: unsafe extern "C" fn() -> *const c_char,
        obs_register_source_s: unsafe extern "C" fn(info: *const obs_source_info, size: usize),
        obs_module_get_config_path: unsafe extern "C" fn(module: *mut obs_module_t, file: *const c_char) -> *mut c_char,

        // obs.h: sources
        obs_source_create: unsafe extern "C" fn(
            id: *const c_char,
            name: *const c_char,
            settings: *mut obs_data_t,
            hotkey_data: *mut obs_data_t,
        ) -> *mut obs_source_t,
        obs_source_create_private: unsafe extern "C" fn(
            id: *const c_char,
            name: *const c_char,
            settings: *mut obs_data_t,
        ) -> *mut obs_source_t,
        obs_source_release: unsafe extern "C" fn(source: *mut obs_source_t),
        obs_source_get_ref: unsafe extern "C" fn(source: *mut obs_source_t) -> *mut obs_source_t,
        obs_source_get_weak_source: unsafe extern "C" fn(source: *mut obs_source_t) -> *mut obs_weak_source_t,
        obs_weak_source_get_source: unsafe extern "C" fn(weak: *mut obs_weak_source_t) -> *mut obs_source_t,
        obs_weak_source_release: unsafe extern "C" fn(weak: *mut obs_weak_source_t),
        obs_source_add_active_child: unsafe extern "C" fn(parent: *mut obs_source_t, child: *mut obs_source_t) -> bool,
        obs_source_remove_active_child: unsafe extern "C" fn(parent: *mut obs_source_t, child: *mut obs_source_t),
        obs_source_video_render: unsafe extern "C" fn(source: *mut obs_source_t),
        obs_source_update: unsafe extern "C" fn(source: *mut obs_source_t, settings: *mut obs_data_t),
        obs_source_get_name: unsafe extern "C" fn(source: *const obs_source_t) -> *const c_char,
        obs_source_get_uuid: unsafe extern "C" fn(source: *const obs_source_t) -> *const c_char,
        obs_source_get_id: unsafe extern "C" fn(source: *const obs_source_t) -> *const c_char,
        obs_source_get_unversioned_id: unsafe extern "C" fn(source: *const obs_source_t) -> *const c_char,
        obs_source_get_output_flags: unsafe extern "C" fn(source: *const obs_source_t) -> u32,
        obs_source_active: unsafe extern "C" fn(source: *const obs_source_t) -> bool,
        obs_source_removed: unsafe extern "C" fn(source: *const obs_source_t) -> bool,
        obs_source_get_type: unsafe extern "C" fn(source: *const obs_source_t) -> obs_source_type,
        obs_source_get_settings: unsafe extern "C" fn(source: *const obs_source_t) -> *mut obs_data_t,
        obs_source_filter_add: unsafe extern "C" fn(source: *mut obs_source_t, filter: *mut obs_source_t),
        obs_source_filter_remove: unsafe extern "C" fn(source: *mut obs_source_t, filter: *mut obs_source_t),
        obs_source_enum_filters:
            unsafe extern "C" fn(source: *mut obs_source_t, callback: obs_source_enum_proc_t, param: *mut c_void),
        obs_filter_get_parent: unsafe extern "C" fn(filter: *const obs_source_t) -> *mut obs_source_t,
        obs_get_source_by_uuid: unsafe extern "C" fn(uuid: *const c_char) -> *mut obs_source_t,
        obs_enum_sources: unsafe extern "C" fn(
            enum_proc: Option<unsafe extern "C" fn(param: *mut c_void, source: *mut obs_source_t) -> bool>,
            param: *mut c_void,
        ),
        obs_get_output_source: unsafe extern "C" fn(channel: u32) -> *mut obs_source_t,
        obs_obj_get_data: unsafe extern "C" fn(obj: *mut c_void) -> *mut c_void,

        // obs.h, media-io/audio-io.h: the audio output that filters receive audio in
        obs_get_audio: unsafe extern "C" fn() -> *mut audio_t,
        audio_output_get_channels: unsafe extern "C" fn(audio: *const audio_t) -> usize,
        audio_output_get_sample_rate: unsafe extern "C" fn(audio: *const audio_t) -> u32,

        // obs.h: scenes
        obs_scene_from_source: unsafe extern "C" fn(source: *const obs_source_t) -> *mut obs_scene_t,
        obs_scene_add: unsafe extern "C" fn(scene: *mut obs_scene_t, source: *mut obs_source_t) -> *mut obs_sceneitem_t,
        obs_scene_find_source_recursive:
            unsafe extern "C" fn(scene: *mut obs_scene_t, name: *const c_char) -> *mut obs_sceneitem_t,

        // obs-data.h
        obs_data_create: unsafe extern "C" fn() -> *mut obs_data_t,
        obs_data_create_from_json: unsafe extern "C" fn(json_string: *const c_char) -> *mut obs_data_t,
        obs_data_release: unsafe extern "C" fn(data: *mut obs_data_t),
        obs_data_get_json: unsafe extern "C" fn(data: *mut obs_data_t) -> *const c_char,
        obs_data_get_string: unsafe extern "C" fn(data: *mut obs_data_t, name: *const c_char) -> *const c_char,
        obs_data_set_string: unsafe extern "C" fn(data: *mut obs_data_t, name: *const c_char, val: *const c_char),
        obs_data_get_int: unsafe extern "C" fn(data: *mut obs_data_t, name: *const c_char) -> c_longlong,
        obs_data_set_int: unsafe extern "C" fn(data: *mut obs_data_t, name: *const c_char, val: c_longlong),
        obs_data_get_bool: unsafe extern "C" fn(data: *mut obs_data_t, name: *const c_char) -> bool,
        obs_data_set_bool: unsafe extern "C" fn(data: *mut obs_data_t, name: *const c_char, val: bool),
        obs_data_get_double: unsafe extern "C" fn(data: *mut obs_data_t, name: *const c_char) -> f64,
        obs_data_set_double: unsafe extern "C" fn(data: *mut obs_data_t, name: *const c_char, val: f64),
        obs_data_set_default_int: unsafe extern "C" fn(data: *mut obs_data_t, name: *const c_char, val: c_longlong),
        obs_data_set_default_bool: unsafe extern "C" fn(data: *mut obs_data_t, name: *const c_char, val: bool),
        obs_data_set_default_string:
            unsafe extern "C" fn(data: *mut obs_data_t, name: *const c_char, val: *const c_char),
        obs_data_set_default_double: unsafe extern "C" fn(data: *mut obs_data_t, name: *const c_char, val: f64),
        obs_data_get_obj: unsafe extern "C" fn(data: *mut obs_data_t, name: *const c_char) -> *mut obs_data_t,
        obs_data_set_obj: unsafe extern "C" fn(data: *mut obs_data_t, name: *const c_char, obj: *mut obs_data_t),
        obs_data_get_array: unsafe extern "C" fn(data: *mut obs_data_t, name: *const c_char) -> *mut obs_data_array_t,
        obs_data_set_array:
            unsafe extern "C" fn(data: *mut obs_data_t, name: *const c_char, array: *mut obs_data_array_t),
        obs_data_array_release: unsafe extern "C" fn(array: *mut obs_data_array_t),

        // obs-properties.h
        obs_properties_create: unsafe extern "C" fn() -> *mut obs_properties_t,
        obs_properties_add_bool: unsafe extern "C" fn(
            props: *mut obs_properties_t,
            name: *const c_char,
            description: *const c_char,
        ) -> *mut obs_property_t,
        obs_properties_add_int: unsafe extern "C" fn(
            props: *mut obs_properties_t,
            name: *const c_char,
            description: *const c_char,
            min: c_int,
            max: c_int,
            step: c_int,
        ) -> *mut obs_property_t,
        obs_properties_add_float_slider: unsafe extern "C" fn(
            props: *mut obs_properties_t,
            name: *const c_char,
            description: *const c_char,
            min: f64,
            max: f64,
            step: f64,
        ) -> *mut obs_property_t,
        obs_properties_add_text: unsafe extern "C" fn(
            props: *mut obs_properties_t,
            name: *const c_char,
            description: *const c_char,
            type_: obs_text_type,
        ) -> *mut obs_property_t,
        obs_properties_add_button2: unsafe extern "C" fn(
            props: *mut obs_properties_t,
            name: *const c_char,
            text: *const c_char,
            callback: obs_property_clicked_t,
            priv_: *mut c_void,
        ) -> *mut obs_property_t,
        obs_property_set_long_description:
            unsafe extern "C" fn(p: *mut obs_property_t, long_description: *const c_char),
        obs_property_text_set_info_type: unsafe extern "C" fn(p: *mut obs_property_t, type_: obs_text_info_type),
        obs_property_set_enabled: unsafe extern "C" fn(p: *mut obs_property_t, enabled: bool),

        // obs-hotkey.h
        obs_hotkey_register_frontend: unsafe extern "C" fn(
            name: *const c_char,
            description: *const c_char,
            func: obs_hotkey_func,
            data: *mut c_void,
        ) -> obs_hotkey_id,
        obs_hotkey_unregister: unsafe extern "C" fn(id: obs_hotkey_id),
        obs_hotkey_set_description: unsafe extern "C" fn(id: obs_hotkey_id, desc: *const c_char),
        obs_hotkey_save: unsafe extern "C" fn(id: obs_hotkey_id) -> *mut obs_data_array_t,
        obs_hotkey_load: unsafe extern "C" fn(id: obs_hotkey_id, data: *mut obs_data_array_t),

        // obs.h, callback/proc.h, callback/calldata.h
        obs_get_proc_handler: unsafe extern "C" fn() -> *mut proc_handler_t,
        proc_handler_call:
            unsafe extern "C" fn(handler: *mut proc_handler_t, name: *const c_char, params: *mut calldata_t) -> bool,
        calldata_set_data:
            unsafe extern "C" fn(data: *mut calldata_t, name: *const c_char, in_: *const c_void, new_size: usize),
        calldata_get_data:
            unsafe extern "C" fn(data: *const calldata_t, name: *const c_char, out: *mut c_void, size: usize) -> bool,
        obs_queue_task: unsafe extern "C" fn(type_: obs_task_type, task: obs_task_t, param: *mut c_void, wait: bool),
        obs_in_task_thread: unsafe extern "C" fn(type_: obs_task_type) -> bool,

        // obs.h: audio
        obs_get_audio_info: unsafe extern "C" fn(oai: *mut obs_audio_info) -> bool,
        obs_source_add_audio_capture_callback: unsafe extern "C" fn(
            source: *mut obs_source_t,
            callback: obs_source_audio_capture_t,
            param: *mut c_void,
        ),
        obs_source_remove_audio_capture_callback: unsafe extern "C" fn(
            source: *mut obs_source_t,
            callback: obs_source_audio_capture_t,
            param: *mut c_void,
        ),
        obs_source_output_audio: unsafe extern "C" fn(source: *mut obs_source_t, audio: *const obs_source_audio),

        // obs.h: outputs, for closed captions on the stream
        obs_output_active: unsafe extern "C" fn(output: *const obs_output_t) -> bool,
        obs_output_release: unsafe extern "C" fn(output: *mut obs_output_t),
        obs_output_output_caption_text2:
            unsafe extern "C" fn(output: *mut obs_output_t, text: *const c_char, display_duration: f64),
    }
}
