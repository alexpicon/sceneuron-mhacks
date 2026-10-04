// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Reading and writing `ai.json`, which holds the API key and is readable only by its owner.

use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use crate::retired::leave_anthropic;
use crate::settings::{AiSettings, NamedModels};

/// Reads the settings, or the defaults plus a warning when the file cannot be used.
///
/// A missing file is normal on first run and gives the defaults without a warning. A file
/// without a provider gets the default one; a base URL or model the file leaves out is its
/// provider's default, so a file that names only `gemini` gets Gemini's URL and models. A file
/// saved for Anthropic, which is no longer offered, loads as Gemini with a warning and is saved
/// that way, so the warning shows once; when that save fails, the warning says so. Subtitle
/// languages an older Sceneuron accepted and this one refuses are dropped with a warning, and the
/// rest loads.
pub fn load_settings(path: &Path) -> (AiSettings, Option<String>) {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return (AiSettings::default(), None),
        Err(error) => return defaults_because(path, &error.to_string()),
    };
    let mut stored: Value = match serde_json::from_str(&text) {
        Ok(stored) => stored,
        Err(error) => return defaults_because(path, &format!("it is not valid JSON: {error}")),
    };
    let departure = leave_anthropic(&mut stored, path);
    // The text itself when nothing moved, so an error can name its line and column.
    let read = match departure {
        None => serde_json::from_str(&text),
        Some(_) => AiSettings::deserialize(&stored),
    };
    let mut settings = match read {
        Ok(settings) => settings,
        Err(error) => return defaults_because(path, &format!("it is not valid JSON: {error}")),
    };
    let named = |pointer: &str| stored.pointer(pointer).is_some();
    if !named("/base_url") {
        settings.provider.default_base_url().clone_into(&mut settings.base_url);
    }
    settings.default_unnamed_models(NamedModels {
        vision: named("/vision_model"),
        planner: named("/planner_model"),
        translation: named("/subtitles/translation_model"),
    });
    let repaired = settings.subtitles.repair();
    if let Err(error) = settings.validate() {
        return defaults_because(path, &error.to_string());
    }
    let warning = departure.map(|moved| match save_settings(path, &settings) {
        Ok(()) => moved,
        Err(error) => format!("{moved}; saving the change failed, so this shows again at the next start: {error}"),
    });
    let repaired = repaired.map(|why| format!("cannot use all the subtitle settings in {}: {why}", path.display()));
    let warnings = warning.into_iter().chain(repaired);
    (settings, warnings.reduce(|one, other| format!("{one}; {other}")))
}

fn defaults_because(path: &Path, why: &str) -> (AiSettings, Option<String>) {
    let warning = format!(
        "cannot use the AI settings in {}: {why}; using the defaults",
        path.display()
    );
    (AiSettings::default(), Some(warning))
}

/// Writes the settings atomically: a crash leaves either the old or the new file.
///
/// The file is created with mode 0600 before the key is written, so the key is never readable by
/// other users, not even briefly.
pub fn save_settings(path: &Path, settings: &AiSettings) -> io::Result<()> {
    let text = serde_json::to_string_pretty(settings).map_err(io::Error::other)?;
    write_settings_text(path, &text)
}

/// Writes `text` as the settings file at `path`, atomically and, on Unix, readable only by its
/// owner, as [`save_settings`] does.
pub(crate) fn write_settings_text(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        fs::create_dir_all(dir)?;
    }
    let temporary = temporary_path(path);
    let result = write_private(&temporary, text.as_bytes()).and_then(|()| fs::rename(&temporary, path));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    // The mode above applies only to new files; a temporary file left by a crash keeps its own.
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}
