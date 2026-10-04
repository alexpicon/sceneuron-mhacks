// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Constants from the libobs and frontend headers under their C names.

use core::ffi::c_int;

use crate::types::{
    obs_frontend_event, obs_hotkey_id, obs_icon_type, obs_source_type, obs_task_type, obs_text_info_type,
    obs_text_type, video_format,
};

/// `MAKE_SEMANTIC_VERSION` from obs-config.h: the encoding of `LIBOBS_API_VER` and of the value a
/// module returns from `obs_module_ver`.
pub const fn make_semantic_version(major: u32, minor: u32, patch: u32) -> u32 {
    (major << 24) | (minor << 16) | patch
}

/// `OBS_INVALID_HOTKEY_ID`
pub const OBS_INVALID_HOTKEY_ID: obs_hotkey_id = obs_hotkey_id::MAX;

// Log levels (util/base.h), the first argument of `blog`.
pub const LOG_ERROR: c_int = 100;
pub const LOG_WARNING: c_int = 200;
pub const LOG_INFO: c_int = 300;
pub const LOG_DEBUG: c_int = 400;

// enum obs_source_type
pub const OBS_SOURCE_TYPE_INPUT: obs_source_type = 0;
pub const OBS_SOURCE_TYPE_FILTER: obs_source_type = 1;
pub const OBS_SOURCE_TYPE_TRANSITION: obs_source_type = 2;
pub const OBS_SOURCE_TYPE_SCENE: obs_source_type = 3;

// Source output flags (obs_source_info.output_flags).
pub const OBS_SOURCE_VIDEO: u32 = 1 << 0;
pub const OBS_SOURCE_AUDIO: u32 = 1 << 1;
pub const OBS_SOURCE_ASYNC: u32 = 1 << 2;
pub const OBS_SOURCE_ASYNC_VIDEO: u32 = OBS_SOURCE_ASYNC | OBS_SOURCE_VIDEO;
pub const OBS_SOURCE_CUSTOM_DRAW: u32 = 1 << 3;
pub const OBS_SOURCE_INTERACTION: u32 = 1 << 5;
pub const OBS_SOURCE_COMPOSITE: u32 = 1 << 6;
pub const OBS_SOURCE_DO_NOT_DUPLICATE: u32 = 1 << 7;
pub const OBS_SOURCE_DEPRECATED: u32 = 1 << 8;
pub const OBS_SOURCE_DO_NOT_SELF_MONITOR: u32 = 1 << 9;
pub const OBS_SOURCE_CAP_DISABLED: u32 = 1 << 10;
pub const OBS_SOURCE_CAP_OBSOLETE: u32 = OBS_SOURCE_CAP_DISABLED;
pub const OBS_SOURCE_MONITOR_BY_DEFAULT: u32 = 1 << 11;
pub const OBS_SOURCE_SUBMIX: u32 = 1 << 12;
pub const OBS_SOURCE_CONTROLLABLE_MEDIA: u32 = 1 << 13;
pub const OBS_SOURCE_CEA_708: u32 = 1 << 14;
pub const OBS_SOURCE_SRGB: u32 = 1 << 15;
pub const OBS_SOURCE_CAP_DONT_SHOW_PROPERTIES: u32 = 1 << 16;
pub const OBS_SOURCE_REQUIRES_CANVAS: u32 = 1 << 17;

// enum video_format
pub const VIDEO_FORMAT_NONE: video_format = 0;
pub const VIDEO_FORMAT_I420: video_format = 1;
pub const VIDEO_FORMAT_NV12: video_format = 2;
pub const VIDEO_FORMAT_YVYU: video_format = 3;
pub const VIDEO_FORMAT_YUY2: video_format = 4;
pub const VIDEO_FORMAT_UYVY: video_format = 5;
pub const VIDEO_FORMAT_RGBA: video_format = 6;
pub const VIDEO_FORMAT_BGRA: video_format = 7;
pub const VIDEO_FORMAT_BGRX: video_format = 8;
pub const VIDEO_FORMAT_Y800: video_format = 9;
pub const VIDEO_FORMAT_I444: video_format = 10;
pub const VIDEO_FORMAT_BGR3: video_format = 11;
pub const VIDEO_FORMAT_I422: video_format = 12;
pub const VIDEO_FORMAT_I40A: video_format = 13;
pub const VIDEO_FORMAT_I42A: video_format = 14;
pub const VIDEO_FORMAT_YUVA: video_format = 15;
pub const VIDEO_FORMAT_AYUV: video_format = 16;
pub const VIDEO_FORMAT_I010: video_format = 17;
pub const VIDEO_FORMAT_P010: video_format = 18;
pub const VIDEO_FORMAT_I210: video_format = 19;
pub const VIDEO_FORMAT_I412: video_format = 20;
pub const VIDEO_FORMAT_YA2L: video_format = 21;
pub const VIDEO_FORMAT_P216: video_format = 22;
pub const VIDEO_FORMAT_P416: video_format = 23;
pub const VIDEO_FORMAT_V210: video_format = 24;
pub const VIDEO_FORMAT_R10L: video_format = 25;

// enum obs_icon_type
pub const OBS_ICON_TYPE_UNKNOWN: obs_icon_type = 0;
pub const OBS_ICON_TYPE_IMAGE: obs_icon_type = 1;
pub const OBS_ICON_TYPE_COLOR: obs_icon_type = 2;
pub const OBS_ICON_TYPE_SLIDESHOW: obs_icon_type = 3;
pub const OBS_ICON_TYPE_AUDIO_INPUT: obs_icon_type = 4;
pub const OBS_ICON_TYPE_AUDIO_OUTPUT: obs_icon_type = 5;
pub const OBS_ICON_TYPE_DESKTOP_CAPTURE: obs_icon_type = 6;
pub const OBS_ICON_TYPE_WINDOW_CAPTURE: obs_icon_type = 7;
pub const OBS_ICON_TYPE_GAME_CAPTURE: obs_icon_type = 8;
pub const OBS_ICON_TYPE_CAMERA: obs_icon_type = 9;
pub const OBS_ICON_TYPE_TEXT: obs_icon_type = 10;
pub const OBS_ICON_TYPE_MEDIA: obs_icon_type = 11;
pub const OBS_ICON_TYPE_BROWSER: obs_icon_type = 12;
pub const OBS_ICON_TYPE_CUSTOM: obs_icon_type = 13;
pub const OBS_ICON_TYPE_PROCESS_AUDIO_OUTPUT: obs_icon_type = 14;

// enum obs_text_type
pub const OBS_TEXT_DEFAULT: obs_text_type = 0;
pub const OBS_TEXT_PASSWORD: obs_text_type = 1;
pub const OBS_TEXT_MULTILINE: obs_text_type = 2;
pub const OBS_TEXT_INFO: obs_text_type = 3;

// enum obs_text_info_type
pub const OBS_TEXT_INFO_NORMAL: obs_text_info_type = 0;
pub const OBS_TEXT_INFO_WARNING: obs_text_info_type = 1;
pub const OBS_TEXT_INFO_ERROR: obs_text_info_type = 2;

// enum obs_frontend_event
pub const OBS_FRONTEND_EVENT_STREAMING_STARTING: obs_frontend_event = 0;
pub const OBS_FRONTEND_EVENT_STREAMING_STARTED: obs_frontend_event = 1;
pub const OBS_FRONTEND_EVENT_STREAMING_STOPPING: obs_frontend_event = 2;
pub const OBS_FRONTEND_EVENT_STREAMING_STOPPED: obs_frontend_event = 3;
pub const OBS_FRONTEND_EVENT_RECORDING_STARTING: obs_frontend_event = 4;
pub const OBS_FRONTEND_EVENT_RECORDING_STARTED: obs_frontend_event = 5;
pub const OBS_FRONTEND_EVENT_RECORDING_STOPPING: obs_frontend_event = 6;
pub const OBS_FRONTEND_EVENT_RECORDING_STOPPED: obs_frontend_event = 7;
pub const OBS_FRONTEND_EVENT_SCENE_CHANGED: obs_frontend_event = 8;
pub const OBS_FRONTEND_EVENT_SCENE_LIST_CHANGED: obs_frontend_event = 9;
pub const OBS_FRONTEND_EVENT_TRANSITION_CHANGED: obs_frontend_event = 10;
pub const OBS_FRONTEND_EVENT_TRANSITION_STOPPED: obs_frontend_event = 11;
pub const OBS_FRONTEND_EVENT_TRANSITION_LIST_CHANGED: obs_frontend_event = 12;
pub const OBS_FRONTEND_EVENT_SCENE_COLLECTION_CHANGED: obs_frontend_event = 13;
pub const OBS_FRONTEND_EVENT_SCENE_COLLECTION_LIST_CHANGED: obs_frontend_event = 14;
pub const OBS_FRONTEND_EVENT_PROFILE_CHANGED: obs_frontend_event = 15;
pub const OBS_FRONTEND_EVENT_PROFILE_LIST_CHANGED: obs_frontend_event = 16;
pub const OBS_FRONTEND_EVENT_EXIT: obs_frontend_event = 17;
pub const OBS_FRONTEND_EVENT_REPLAY_BUFFER_STARTING: obs_frontend_event = 18;
pub const OBS_FRONTEND_EVENT_REPLAY_BUFFER_STARTED: obs_frontend_event = 19;
pub const OBS_FRONTEND_EVENT_REPLAY_BUFFER_STOPPING: obs_frontend_event = 20;
pub const OBS_FRONTEND_EVENT_REPLAY_BUFFER_STOPPED: obs_frontend_event = 21;
pub const OBS_FRONTEND_EVENT_STUDIO_MODE_ENABLED: obs_frontend_event = 22;
pub const OBS_FRONTEND_EVENT_STUDIO_MODE_DISABLED: obs_frontend_event = 23;
pub const OBS_FRONTEND_EVENT_PREVIEW_SCENE_CHANGED: obs_frontend_event = 24;
pub const OBS_FRONTEND_EVENT_SCENE_COLLECTION_CLEANUP: obs_frontend_event = 25;
pub const OBS_FRONTEND_EVENT_FINISHED_LOADING: obs_frontend_event = 26;
pub const OBS_FRONTEND_EVENT_RECORDING_PAUSED: obs_frontend_event = 27;
pub const OBS_FRONTEND_EVENT_RECORDING_UNPAUSED: obs_frontend_event = 28;
pub const OBS_FRONTEND_EVENT_TRANSITION_DURATION_CHANGED: obs_frontend_event = 29;
pub const OBS_FRONTEND_EVENT_REPLAY_BUFFER_SAVED: obs_frontend_event = 30;
pub const OBS_FRONTEND_EVENT_VIRTUALCAM_STARTED: obs_frontend_event = 31;
pub const OBS_FRONTEND_EVENT_VIRTUALCAM_STOPPED: obs_frontend_event = 32;
pub const OBS_FRONTEND_EVENT_TBAR_VALUE_CHANGED: obs_frontend_event = 33;
pub const OBS_FRONTEND_EVENT_SCENE_COLLECTION_CHANGING: obs_frontend_event = 34;
pub const OBS_FRONTEND_EVENT_PROFILE_CHANGING: obs_frontend_event = 35;
pub const OBS_FRONTEND_EVENT_SCRIPTING_SHUTDOWN: obs_frontend_event = 36;
pub const OBS_FRONTEND_EVENT_PROFILE_RENAMED: obs_frontend_event = 37;
pub const OBS_FRONTEND_EVENT_SCENE_COLLECTION_RENAMED: obs_frontend_event = 38;
pub const OBS_FRONTEND_EVENT_THEME_CHANGED: obs_frontend_event = 39;
pub const OBS_FRONTEND_EVENT_SCREENSHOT_TAKEN: obs_frontend_event = 40;
pub const OBS_FRONTEND_EVENT_CANVAS_ADDED: obs_frontend_event = 41;
pub const OBS_FRONTEND_EVENT_CANVAS_REMOVED: obs_frontend_event = 42;

/// `OBS_TASK_UI`: the frontend's UI thread.
pub const OBS_TASK_UI: obs_task_type = 0;
/// `OBS_TASK_GRAPHICS`
pub const OBS_TASK_GRAPHICS: obs_task_type = 1;
/// `OBS_TASK_AUDIO`
pub const OBS_TASK_AUDIO: obs_task_type = 2;
/// `OBS_TASK_DESTROY`
pub const OBS_TASK_DESTROY: obs_task_type = 3;
