// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Constant values against the OBS 32.2.2 headers: video formats, frontend events, source types
//! and flags, icon and text types, log levels, the version encoding, and hotkey ids.

use sceneuron_obs_sys::*;

#[test]
fn video_format_codes_match_the_enum() {
    let formats = [
        VIDEO_FORMAT_NONE,
        VIDEO_FORMAT_I420,
        VIDEO_FORMAT_NV12,
        VIDEO_FORMAT_YVYU,
        VIDEO_FORMAT_YUY2,
        VIDEO_FORMAT_UYVY,
        VIDEO_FORMAT_RGBA,
        VIDEO_FORMAT_BGRA,
        VIDEO_FORMAT_BGRX,
        VIDEO_FORMAT_Y800,
        VIDEO_FORMAT_I444,
        VIDEO_FORMAT_BGR3,
        VIDEO_FORMAT_I422,
        VIDEO_FORMAT_I40A,
        VIDEO_FORMAT_I42A,
        VIDEO_FORMAT_YUVA,
        VIDEO_FORMAT_AYUV,
        VIDEO_FORMAT_I010,
        VIDEO_FORMAT_P010,
        VIDEO_FORMAT_I210,
        VIDEO_FORMAT_I412,
        VIDEO_FORMAT_YA2L,
        VIDEO_FORMAT_P216,
        VIDEO_FORMAT_P416,
        VIDEO_FORMAT_V210,
        VIDEO_FORMAT_R10L,
    ];
    let expected: Vec<u32> = (0..=25).collect();
    assert_eq!(formats.to_vec(), expected);
}

#[test]
fn frontend_events_match_the_enum() {
    assert_eq!(OBS_FRONTEND_EVENT_STREAMING_STARTING, 0);
    assert_eq!(OBS_FRONTEND_EVENT_SCENE_CHANGED, 8);
    assert_eq!(OBS_FRONTEND_EVENT_SCENE_COLLECTION_CHANGED, 13);
    assert_eq!(OBS_FRONTEND_EVENT_EXIT, 17);
    assert_eq!(OBS_FRONTEND_EVENT_SCENE_COLLECTION_CLEANUP, 25);
    assert_eq!(OBS_FRONTEND_EVENT_FINISHED_LOADING, 26);
    assert_eq!(OBS_FRONTEND_EVENT_SCENE_COLLECTION_CHANGING, 34);
    assert_eq!(OBS_FRONTEND_EVENT_SCRIPTING_SHUTDOWN, 36);
    assert_eq!(OBS_FRONTEND_EVENT_CANVAS_REMOVED, 42);
}

#[test]
fn source_types_and_flags_match_the_header() {
    assert_eq!(
        [
            OBS_SOURCE_TYPE_INPUT,
            OBS_SOURCE_TYPE_FILTER,
            OBS_SOURCE_TYPE_TRANSITION,
            OBS_SOURCE_TYPE_SCENE
        ],
        [0, 1, 2, 3]
    );
    assert_eq!(OBS_SOURCE_VIDEO, 1);
    assert_eq!(OBS_SOURCE_ASYNC_VIDEO, 0b101);
    assert_eq!(OBS_SOURCE_CUSTOM_DRAW, 1 << 3);
    assert_eq!(OBS_SOURCE_CAP_OBSOLETE, OBS_SOURCE_CAP_DISABLED);
    assert_eq!(OBS_SOURCE_SRGB, 1 << 15);
    assert_eq!(OBS_SOURCE_REQUIRES_CANVAS, 1 << 17);
}

#[test]
fn icon_and_text_types_match_the_header() {
    assert_eq!(OBS_ICON_TYPE_CAMERA, 9);
    assert_eq!(OBS_ICON_TYPE_TEXT, 10);
    assert_eq!(OBS_ICON_TYPE_BROWSER, 12);
    assert_eq!(OBS_ICON_TYPE_CUSTOM, 13);
    assert_eq!(
        [OBS_TEXT_DEFAULT, OBS_TEXT_PASSWORD, OBS_TEXT_MULTILINE, OBS_TEXT_INFO],
        [0, 1, 2, 3]
    );
    assert_eq!(
        [OBS_TEXT_INFO_NORMAL, OBS_TEXT_INFO_WARNING, OBS_TEXT_INFO_ERROR],
        [0, 1, 2]
    );
}

#[test]
fn task_types_match_the_header() {
    assert_eq!(
        [OBS_TASK_UI, OBS_TASK_GRAPHICS, OBS_TASK_AUDIO, OBS_TASK_DESTROY],
        [0, 1, 2, 3]
    );
}

#[test]
fn log_levels_match_the_header() {
    assert_eq!([LOG_ERROR, LOG_WARNING, LOG_INFO, LOG_DEBUG], [100, 200, 300, 400]);
}

#[test]
fn semantic_version_matches_the_libobs_macro() {
    assert_eq!(make_semantic_version(32, 2, 2), 537_001_986);
    assert_eq!(make_semantic_version(31, 1, 0), 520_159_232);
    const AT_COMPILE_TIME: u32 = make_semantic_version(1, 2, 3);
    assert_eq!(AT_COMPILE_TIME, 0x0102_0003);
}

#[test]
fn invalid_hotkey_id_is_all_ones() {
    assert_eq!(OBS_INVALID_HOTKEY_ID, usize::MAX);
}

#[test]
fn frames_hold_eight_planes() {
    assert_eq!(MAX_AV_PLANES, 8);
}
