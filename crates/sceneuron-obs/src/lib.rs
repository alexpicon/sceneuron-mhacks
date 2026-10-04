// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The Sceneuron OBS module: entry points, source registration, and wiring to the shared runtime.

#[cfg(not(target_os = "macos"))]
compile_error!("Sceneuron supports macOS only for now; Linux and Windows support is planned.");

use std::ffi::c_char;
use std::path::PathBuf;
use std::sync::atomic::{AtomicPtr, Ordering};

use sceneuron_obs_sys as sys;
use sceneuron_server::HubEvent;

mod ai;
pub mod audio_block;
mod audio_host;
mod caption_output;
pub mod captions;
pub mod config;
mod ffi;
mod filter;
pub mod frame;
mod frontend;
mod host;
pub mod hotkey_set;
mod hotkeys;
pub mod logger;
mod on_air;
mod open_url;
mod overlay;
pub mod overlay_audio;
mod overlay_source;
mod plugin;
pub mod preview;
mod properties;
mod settings;
pub mod sighting;
pub mod signals;
mod source_filters;
pub mod starter;
pub mod stream_deck;
mod ui;
mod vendor;
pub mod vendor_json;
pub mod vendor_requests;
mod vision;
mod voice;
mod voice_filter;

static MODULE: AtomicPtr<sys::obs_module_t> = AtomicPtr::new(std::ptr::null_mut());

/// Oldest libobs this module supports; libobs refuses modules built for a newer major.minor.
const MIN_OBS_VERSION: u32 = sys::make_semantic_version(31, 1, 0);

#[unsafe(no_mangle)]
pub extern "C" fn obs_module_set_pointer(module: *mut sys::obs_module_t) {
    MODULE.store(module, Ordering::SeqCst);
}

#[unsafe(no_mangle)]
pub extern "C" fn obs_current_module() -> *mut sys::obs_module_t {
    MODULE.load(Ordering::SeqCst)
}

#[unsafe(no_mangle)]
pub extern "C" fn obs_module_ver() -> u32 {
    MIN_OBS_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn obs_module_name() -> *const c_char {
    c"Sceneuron".as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn obs_module_description() -> *const c_char {
    c"AI-driven, editable live overlays that react to your camera.".as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn obs_module_author() -> *const c_char {
    c"Alexander Picon".as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn obs_module_load() -> bool {
    if let Err(error) = sys::load() {
        eprintln!("[sceneuron] cannot use this OBS version: {error}");
        return false;
    }
    logger::install();
    ffi::guard(false, || match load() {
        Ok(()) => true,
        Err(error) => {
            ffi::log(sys::LOG_ERROR, &format!("failed to load: {error}"));
            false
        }
    })
}

fn load() -> Result<(), String> {
    // SAFETY: the module pointer was set by libobs; an empty file name yields the directory.
    let dir = unsafe {
        ffi::take_bmalloc_string((sys::obs().obs_module_get_config_path)(
            obs_current_module(),
            c"".as_ptr(),
        ))
    };
    if dir.is_empty() {
        return Err("OBS did not provide a config directory".to_owned());
    }
    plugin::Plugin::start(PathBuf::from(dir))?;
    overlay_source::register();
    filter::register();
    voice_filter::register();
    settings::register();
    ffi::info(&format!("loaded version {}", env!("CARGO_PKG_VERSION")));
    Ok(())
}

#[unsafe(no_mangle)]
pub extern "C" fn obs_module_post_load() {
    ffi::guard((), || {
        let Some(plugin) = plugin::current() else {
            return;
        };
        settings::add_tools_menu();
        frontend::connect();
        caption_output::start(&plugin.hub);
        hotkeys::register_all();
        vendor::register();
        plugin.hub.add_listener(Box::new(|event| match event {
            HubEvent::PresenceChanged(presence) => vendor::emit_presence(*presence),
            HubEvent::ProjectChanged => hotkeys::request_sync(),
            HubEvent::SaveFailed(message) => ffi::warn(message),
            HubEvent::StateChanged { .. } => {}
        }));
        ffi::info(&format!("editor at 127.0.0.1:{}", plugin.port_setting()));
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn obs_module_unload() {
    ffi::guard((), || {
        hotkeys::unregister_all();
        caption_output::stop();
        plugin::shutdown();
        ffi::info("unloaded");
    })
}
