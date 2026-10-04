// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A fresh install: the first time the plugin loads, with no project file yet, it starts from the
//! starter project and puts the starter AI settings in place, unless an AI settings file is already
//! there. A project file that exists, even one that cannot be read or was set aside as unreadable
//! at an earlier start, means the install is not fresh, so its AI settings stay as they are, and no
//! file that exists is ever changed.

use sceneuron_ai::place_starter_settings;
use sceneuron_core::{Loaded, load_or_starter};

use crate::config::Paths;

/// Loads the project from `paths`; on a fresh install, the starter project, with the starter AI
/// settings put in place. A failure to write them is added to the warning.
pub fn load_project(paths: &Paths) -> Loaded {
    let mut loaded = load_or_starter(&paths.project);
    if !loaded.fresh {
        return loaded;
    }
    if let Err(error) = place_starter_settings(&paths.ai) {
        let note = format!(
            "the starter AI settings could not be written to {}: {error}",
            paths.ai.display()
        );
        loaded.warning = Some(match loaded.warning.take() {
            Some(warning) => format!("{warning}; {note}"),
            None => note,
        });
    }
    loaded
}
