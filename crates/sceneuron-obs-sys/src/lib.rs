// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Raw bindings to the parts of the libobs and OBS frontend C APIs that Sceneuron uses.
//!
//! Nothing here links against OBS. [`load`] resolves every function from the running process at
//! runtime with `dlsym(RTLD_DEFAULT, ...)`, so the plugin builds without an OBS SDK and a missing
//! export becomes a [`LoadError`] naming it instead of a loader failure. The libobs table is
//! required; the frontend table is optional because headless libobs hosts do not export it.
//!
//! Declarations follow the OBS 32.2.2 headers, and the struct layouts are pinned by compile-time
//! assertions on 64-bit targets. Every call through a table is `unsafe`: the caller upholds the C
//! contract of the function. Safe wrappers belong in the plugin crate.

#![allow(non_camel_case_types)]

#[cfg(not(target_os = "macos"))]
compile_error!("Sceneuron supports macOS only for now; Linux and Windows support is planned.");

mod audio;
mod consts;
mod error;
mod frontend;
mod global;
#[cfg(target_pointer_width = "64")]
mod layout;
mod libobs;
mod platform;
mod source_info;
mod table;
mod types;

pub use audio::*;
pub use consts::*;
pub use error::LoadError;
pub use frontend::Frontend;
pub use global::{frontend, install, load, obs, resolve_tables, try_obs};
pub use libobs::Libobs;
pub use platform::{find_frontend_symbol, find_libobs_symbol};
pub use source_info::obs_source_info;
pub use types::*;
