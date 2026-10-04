// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The AI settings a fresh install starts with: Gemini with its default models, voice control
//! listening all the time once it is on, and subtitles translated into Japanese and sent as stream
//! captions, but no key, no consent, and nothing that sends data switched on. Nothing leaves the
//! computer until the user adds a key, agrees to what it sends, and switches it on.
//!
//! They are kept as a settings file (`starter/ai.json`) that a fresh install puts in place, so they
//! load, then and at every later start, as saved settings do. An existing file is never changed.

use std::fs;
use std::io;
use std::path::Path;

use crate::settings_file::write_settings_text;

/// The starter AI settings, as a saved settings file.
const STARTER_SETTINGS: &str = include_str!("../starter/ai.json");

/// Puts the starter AI settings at `path` when no file is there yet; returns whether it did. Like
/// saved settings, the file is written atomically and, on Unix, readable only by its owner.
pub fn place_starter_settings(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => return Ok(false),
        Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
        Err(_) => {}
    }
    write_settings_text(path, STARTER_SETTINGS)?;
    Ok(true)
}
