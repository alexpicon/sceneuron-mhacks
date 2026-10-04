// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The starter project a fresh install begins with, so a new user finds everything set up: the
//! default project's cards and rules, the meeting signals, planning poker, classroom, and status
//! decks, and live subtitles bars. Planning poker and meeting signals answer hand signals, voice,
//! and speech; classroom and status wait for the user to switch them on.
//!
//! It is kept as a project file (`starter/project.json`) and goes through the same reading and
//! checks as a saved one. It is used only while no project file exists and none was set aside as
//! unreadable; a saved project is never replaced, and nothing is written until the project changes.

use std::path::Path;

use crate::model::Project;
use crate::persist::{Loaded, PersistError, load_or_default, parse};

/// The starter project, as a saved project file.
const STARTER_PROJECT: &str = include_str!("../starter/project.json");

/// The project a fresh install starts with, read and checked as a saved project file is.
pub fn starter_project() -> Result<Project, PersistError> {
    parse(STARTER_PROJECT).map(|(project, ..)| project)
}

/// Loads the project at `path` as [`load_or_default`] does, except that a fresh install, with no
/// file there yet and none set aside as unreadable, starts from the starter project.
pub fn load_or_starter(path: &Path) -> Loaded {
    let mut loaded = load_or_default(path);
    if loaded.fresh {
        match starter_project() {
            Ok(project) => loaded.project = project,
            Err(error) => {
                loaded.warning = Some(format!(
                    "the starter project cannot be used ({error}); starting from the default project"
                ));
            }
        }
    }
    loaded
}
