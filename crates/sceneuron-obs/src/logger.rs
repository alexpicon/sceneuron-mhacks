// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Sends the log records of Sceneuron's own crates to the OBS log. Debug records, such as how a
//! model request asked for its answer's shape, appear when OBS runs with `--verbose`.

use std::ffi::c_int;

use log::{Level, LevelFilter, Log, Metadata, Record};
use sceneuron_obs_sys as sys;

use crate::ffi;

struct ObsLog;

static OBS_LOG: ObsLog = ObsLog;

/// Installs the OBS log as the process's logger, unless the host already installed one.
pub(crate) fn install() {
    if log::set_logger(&OBS_LOG).is_ok() {
        log::set_max_level(LevelFilter::Debug);
    }
}

/// Whether a record goes to the OBS log: only Sceneuron's own, so the libraries it uses add
/// nothing.
pub fn forwards(metadata: &Metadata<'_>) -> bool {
    metadata.target().starts_with("sceneuron")
}

/// The OBS log level for a record's level; trace goes out as debug.
pub fn obs_level(level: Level) -> c_int {
    match level {
        Level::Error => sys::LOG_ERROR,
        Level::Warn => sys::LOG_WARNING,
        Level::Info => sys::LOG_INFO,
        Level::Debug | Level::Trace => sys::LOG_DEBUG,
    }
}

impl Log for ObsLog {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        forwards(metadata)
    }

    fn log(&self, record: &Record<'_>) {
        if forwards(record.metadata()) && sys::try_obs().is_some() {
            ffi::log(obs_level(record.level()), &record.args().to_string());
        }
    }

    fn flush(&self) {}
}
