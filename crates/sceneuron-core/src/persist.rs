// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Saving and loading the project file.

use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::defaults::default_project;
use crate::migrate::{forget_unknown_theme, free_reserved_ids};
use crate::model::{Project, SCHEMA_VERSION, ValidationError};

/// Why a project file could not be used.
#[derive(Debug)]
pub enum PersistError {
    Io(io::Error),
    Parse(serde_json::Error),
    /// The file was written by a newer Sceneuron.
    NewerVersion(u32),
    Invalid(ValidationError),
}

impl fmt::Display for PersistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersistError::Io(error) => write!(f, "cannot read the project file: {error}"),
            PersistError::Parse(error) => write!(f, "the project file is not valid JSON: {error}"),
            PersistError::NewerVersion(version) => {
                write!(
                    f,
                    "the project file has version {version}; this build reads version {SCHEMA_VERSION}"
                )
            }
            PersistError::Invalid(error) => write!(f, "the project file is invalid: {error}"),
        }
    }
}

impl std::error::Error for PersistError {}

/// Reads a project; `Ok(None)` means the file does not exist yet.
///
/// Files from older versions are upgraded in memory; every older format is a subset of the
/// current one, so missing sections take their defaults.
pub fn load_project(path: &Path) -> Result<Option<Project>, PersistError> {
    Ok(read(path)?.map(|(project, ..)| project))
}

/// The project, the version it was saved with, and notes on what loading changed in it.
fn read(path: &Path) -> Result<Option<(Project, u32, Vec<String>)>, PersistError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(PersistError::Io(error)),
    };
    parse(&text).map(Some)
}

/// Reads the text of a project file: the project, upgraded and checked, the version it was saved
/// with, and notes on what loading changed in it.
pub(crate) fn parse(text: &str) -> Result<(Project, u32, Vec<String>), PersistError> {
    let mut value: serde_json::Value = serde_json::from_str(text).map_err(PersistError::Parse)?;
    let version = value.get("version").and_then(serde_json::Value::as_u64).unwrap_or(0);
    let version = u32::try_from(version).unwrap_or(u32::MAX);
    if version > SCHEMA_VERSION {
        return Err(PersistError::NewerVersion(version));
    }
    let mut notes: Vec<String> = forget_unknown_theme(&mut value).into_iter().collect();
    let mut project: Project = serde_json::from_value(value).map_err(PersistError::Parse)?;
    project.version = SCHEMA_VERSION;
    notes.extend(free_reserved_ids(&mut project));
    project.validate().map_err(PersistError::Invalid)?;
    Ok((project, version, notes))
}

/// Writes a project atomically: a crash leaves either the old or the new file.
pub fn save_project(path: &Path, project: &Project) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(project).map_err(io::Error::other)?;
    let temporary = sibling(path, "tmp");
    {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&temporary, path)
}

/// Loaded project plus a warning when the saved file had to be set aside.
#[derive(Debug)]
pub struct Loaded {
    pub project: Project,
    pub warning: Option<String>,
    /// False when the file could be neither read nor set aside, so saving to its path would
    /// destroy a project nobody has read; keep changes in memory instead.
    pub writable: bool,
    /// True when there was no project file yet, nor one set aside as unreadable: a fresh install.
    pub fresh: bool,
}

/// Loads the project, or starts from the default one.
///
/// A file that cannot be used is renamed with an `.unreadable` suffix instead of being
/// overwritten, so a later fix can still recover it; earlier set-aside files are kept too.
/// A file from an older version is copied with a `.v<version>` suffix before the first save
/// replaces it, so an older Sceneuron can still use the copy.
pub fn load_or_default(path: &Path) -> Loaded {
    match read(path) {
        Ok(Some((project, version, mut notes))) => {
            let kept = (version < SCHEMA_VERSION)
                .then(|| keep_old_version(path, version))
                .flatten();
            notes.extend(kept);
            let warning = (!notes.is_empty()).then(|| notes.join("; "));
            Loaded {
                project,
                warning,
                writable: true,
                fresh: false,
            }
        }
        // A project set aside as unreadable on an earlier start means the install was used before,
        // even though no project has been saved since.
        Ok(None) => Loaded {
            project: default_project(),
            warning: None,
            writable: true,
            fresh: !sibling(path, "unreadable").exists(),
        },
        Err(error) => {
            let backup = free_sibling(path, "unreadable");
            let moved = fs::rename(path, &backup).is_ok();
            let warning = if moved {
                format!(
                    "{error}; kept it as {}; starting from the default project",
                    backup.display()
                )
            } else {
                format!("{error}; it could not be set aside, so changes are not saved until OBS restarts")
            };
            Loaded {
                project: default_project(),
                warning: Some(warning),
                writable: moved,
                fresh: false,
            }
        }
    }
}

/// Copies an older-version file aside once; returns a warning only when the copy fails.
fn keep_old_version(path: &Path, version: u32) -> Option<String> {
    let copy = sibling(path, &format!("v{version}"));
    if copy.exists() {
        return None;
    }
    match fs::copy(path, &copy) {
        Ok(_) => None,
        Err(error) => Some(format!(
            "could not keep a copy of the version {version} project: {error}"
        )),
    }
}

/// The first `<name>.<suffix>`, `<name>.<suffix>.2`, ... that does not exist yet.
fn free_sibling(path: &Path, suffix: &str) -> PathBuf {
    let first = sibling(path, suffix);
    if !first.exists() {
        return first;
    }
    (2..1_000)
        .map(|n| sibling(path, &format!("{suffix}.{n}")))
        .find(|candidate| !candidate.exists())
        .unwrap_or(first)
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".");
    name.push(suffix);
    path.with_file_name(name)
}
