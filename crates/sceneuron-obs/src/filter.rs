// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The "Sceneuron Camera Analysis" async video filter: samples frames for the local perceiver,
//! and for the editor's camera preview while someone watches it, which its tick drops once
//! nobody does.

use std::ffi::{c_char, c_void};
use std::ptr;

use sceneuron_ai::Frame;
use sceneuron_obs_sys as sys;
use sceneuron_vision::{Sample, to_rgb};

use crate::ai;
use crate::ffi::{self, guard};
use crate::frame::frame_ref;
use crate::plugin;
use crate::preview::{PREVIEW_SIZE, keeps_aspect};
use crate::properties;

pub const FILTER_ID: &std::ffi::CStr = c"sceneuron_camera_filter";

struct CameraFilter {
    next_sample_ms: u64,
    reported_error: bool,
}

pub fn register() {
    // SAFETY: an all-zero obs_source_info is valid: null pointers and absent callbacks.
    let mut info: sys::obs_source_info = unsafe { std::mem::zeroed() };
    info.id = FILTER_ID.as_ptr();
    info.type_ = sys::OBS_SOURCE_TYPE_FILTER;
    info.output_flags = sys::OBS_SOURCE_VIDEO | sys::OBS_SOURCE_ASYNC;
    info.get_name = Some(get_name);
    info.create = Some(create);
    info.destroy = Some(destroy);
    info.filter_video = Some(filter_video);
    info.video_tick = Some(video_tick);
    info.get_properties = Some(get_properties);
    // SAFETY: libobs copies `info` during the call; the id is a 'static C string.
    unsafe { (sys::obs().obs_register_source_s)(&info, std::mem::size_of::<sys::obs_source_info>()) };
}

unsafe extern "C" fn get_name(_type_data: *mut c_void) -> *const c_char {
    c"Sceneuron Camera Analysis".as_ptr()
}

unsafe extern "C" fn create(_settings: *mut sys::obs_data_t, _source: *mut sys::obs_source_t) -> *mut c_void {
    guard(ptr::null_mut(), || {
        if let Some(plugin) = plugin::current() {
            plugin.vision.filter_created();
        }
        let filter = CameraFilter {
            next_sample_ms: 0,
            reported_error: false,
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
        drop(unsafe { Box::from_raw(data.cast::<CameraFilter>()) });
        if let Some(plugin) = plugin::current() {
            plugin.vision.filter_destroyed(data as usize);
        }
    })
}

unsafe extern "C" fn filter_video(data: *mut c_void, frame: *mut sys::obs_source_frame) -> *mut sys::obs_source_frame {
    guard(frame, || {
        if data.is_null() || frame.is_null() {
            return frame;
        }
        // SAFETY: libobs passes back the pointer `create` returned, on one thread at a time.
        let filter = unsafe { &mut *data.cast::<CameraFilter>() };
        // SAFETY: libobs guarantees the frame and its planes are valid during the callback.
        sample(filter, unsafe { &*frame });
        frame
    })
}

/// Runs on the graphics thread for every video frame, even while the camera sends none, so the
/// preview's frame is dropped soon after the editor stops asking.
unsafe extern "C" fn video_tick(_data: *mut c_void, _seconds: f32) {
    guard((), || {
        if let Some(plugin) = plugin::current() {
            plugin.vision.preview().expire(plugin.hub.now_ms());
        }
    })
}

fn sample(filter: &mut CameraFilter, frame: &sys::obs_source_frame) {
    let Some(plugin) = plugin::current() else {
        return;
    };
    // The filter's address is its id while it lives.
    if !plugin.vision.claim(std::ptr::from_mut(filter) as usize) {
        return;
    }
    let now = plugin.hub.now_ms();
    ai::offer_frame(&plugin.ai, frame, now);
    let preview = plugin.vision.preview();
    // Two atomic loads; nothing is copied for the preview unless the editor asked lately.
    let mut wants_preview = preview.wants_frame(now);
    if now >= filter.next_sample_ms
        && let Some(input) = plugin.vision.accepts()
    {
        let fps = plugin.hub.read(|engine| engine.project().detection.sample_fps);
        filter.next_sample_ms = now + (1000.0 / fps.max(0.5)) as u64;
        let (width, height) = input.for_frame(frame.width, frame.height);
        match convert(frame, width, height) {
            Ok(rgb) => {
                if wants_preview && keeps_aspect(width, height, frame.width, frame.height) {
                    preview.store(Frame {
                        rgb: rgb.as_slice().into(),
                        width,
                        height,
                        captured_ms: now,
                    });
                    wants_preview = false;
                }
                let sample = Sample {
                    rgb,
                    width,
                    height,
                    captured_ms: now,
                };
                plugin.vision.submit(sample);
            }
            Err(message) if !filter.reported_error => {
                filter.reported_error = true;
                ffi::warn(&format!("cannot analyze this camera: {message}"));
                let status = sceneuron_core::VisionStatus::Failed { message };
                plugin.hub.update(|engine, now| engine.set_vision_status(status, now));
            }
            Err(_) => {}
        }
    }
    if wants_preview {
        let (width, height) = PREVIEW_SIZE.for_frame(frame.width, frame.height);
        if let Ok(rgb) = convert(frame, width, height) {
            preview.store(Frame {
                rgb: rgb.into(),
                width,
                height,
                captured_ms: now,
            });
        }
    }
}

/// The frame as packed RGB at `width` x `height`, right side up.
fn convert(frame: &sys::obs_source_frame, width: u32, height: u32) -> Result<Vec<u8>, String> {
    // SAFETY: the frame's planes stay valid for the duration of this callback.
    let view = unsafe { frame_ref(frame) }.map_err(|error| error.to_string())?;
    let mut rgb = vec![0u8; width as usize * height as usize * 3];
    to_rgb(&view, width, height, &mut rgb).map_err(|error| error.to_string())?;
    Ok(if frame.flip { flip_rows(rgb, width) } else { rgb })
}

/// Turns an upside-down sample (OBS sets `flip` for such frames) right side up.
fn flip_rows(rgb: Vec<u8>, width: u32) -> Vec<u8> {
    let row = width as usize * 3;
    rgb.chunks_exact(row).rev().flatten().copied().collect()
}

unsafe extern "C" fn get_properties(_data: *mut c_void) -> *mut sys::obs_properties_t {
    guard(ptr::null_mut(), || {
        let props = properties::Properties::new();
        props.info(
            "about",
            "Sceneuron analyzes this source on your computer to see whether someone is in view and \
             what they are doing. While the editor's Camera & mic tab is open, it shows what Sceneuron \
             sees; that preview never leaves your computer. If you turn on AI scene understanding in \
             the editor, a still image from this source goes to the AI provider you chose, at most \
             about once every few seconds.",
        );
        props.open_editor_button();
        props.into_raw()
    })
}
