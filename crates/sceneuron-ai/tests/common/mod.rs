// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Shared test helpers: a scripted HTTP server on 127.0.0.1, a recording director sink, a JSON
//! schema checker, assistant fixtures, test frames and settings, settings saved for Anthropic, a
//! test ElevenLabs key and audio, a log capture, and temporary directories under the cargo target
//! directory. Nothing here reaches beyond this computer.

#![allow(dead_code)]

pub mod assistant;
pub mod director;
pub mod gate;
pub mod gemini;
#[cfg(feature = "hub")]
pub mod hub_subtitles;
pub mod logs;
pub mod plans;
pub mod retired;
pub mod schema;
pub mod server;
pub mod sink;
pub mod speech;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use sceneuron_ai::{AiSettings, Frame};
use sceneuron_core::Condition;

/// A key-shaped test value; never a real credential.
pub const TEST_KEY: &str = "test-key-0123456789abcd";

/// The defaults, on Gemini, with scene understanding on, consent given, and a test key.
pub fn enabled_settings() -> AiSettings {
    AiSettings {
        enabled: true,
        consent: true,
        api_key: TEST_KEY.to_owned(),
        ..AiSettings::default()
    }
}

/// A frame of one gray level.
pub fn gray_frame(width: u32, height: u32, luma: u8, captured_ms: u64) -> Frame {
    let rgb = vec![luma; width as usize * height as usize * 3];
    Frame {
        rgb: Arc::from(rgb),
        width,
        height,
        captured_ms,
    }
}

/// A frame built pixel by pixel.
pub fn frame_from(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 3]) -> Frame {
    let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
    for y in 0..height {
        for x in 0..width {
            rgb.extend_from_slice(&pixel(x, y));
        }
    }
    Frame {
        rgb: Arc::from(rgb),
        width,
        height,
        captured_ms: 0,
    }
}

/// An enabled condition with the core's default thresholds.
pub fn condition(id: &str, prompt: &str, detail: &str) -> Condition {
    Condition {
        id: id.to_owned(),
        name: format!("Name of {id}"),
        prompt: prompt.to_owned(),
        detail: detail.to_owned(),
        enabled: true,
        min_confidence: 0.6,
        confirm: 1,
        release: 2,
    }
}

/// A new empty directory under the cargo target directory.
pub fn temp_dir(name: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ai-{name}-{}-{unique}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
