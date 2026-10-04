// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The Tools menu: Tools > Sceneuron opens the editor in the browser, and Tools > Sceneuron
//! settings opens a settings window built from the properties of a settings-only source.
//!
//! Rust cannot create Qt widgets, so the settings window is the standard OBS properties dialog of
//! a private source that is never added to a scene.
//!
//! OBS hands the source its whole settings object on every change, and the old one again on
//! Cancel. Only values that differ from what Sceneuron last showed in the dialog are applied, so
//! a pause made elsewhere while the dialog is open is never undone by an unrelated edit.

use std::ffi::{c_char, c_void};
use std::ptr;
use std::sync::Mutex;

use sceneuron_obs_sys as sys;

use crate::ffi::{Data, SourceRef, data_bool, data_int, guard};
use crate::plugin;
use crate::properties::Properties;

const SETTINGS_ID: &std::ffi::CStr = c"sceneuron_settings";

static SETTINGS_SOURCE: Mutex<Option<SourceRef>> = Mutex::new(None);

/// The values the dialog holds as far as Sceneuron knows: what it showed, or what it applied.
static SHOWN: Mutex<Shown> = Mutex::new(Shown {
    automation: None,
    port: None,
});

struct Shown {
    automation: Option<bool>,
    port: Option<i64>,
}

pub fn register() {
    // SAFETY: an all-zero obs_source_info is valid: null pointers and absent callbacks.
    let mut info: sys::obs_source_info = unsafe { std::mem::zeroed() };
    info.id = SETTINGS_ID.as_ptr();
    info.type_ = sys::OBS_SOURCE_TYPE_INPUT;
    info.output_flags = sys::OBS_SOURCE_CAP_DISABLED;
    info.get_name = Some(get_name);
    info.create = Some(create);
    info.destroy = Some(destroy);
    info.update = Some(update);
    info.get_properties = Some(get_properties);
    // SAFETY: libobs copies `info` during the call; the id is a 'static C string.
    unsafe { (sys::obs().obs_register_source_s)(&info, std::mem::size_of::<sys::obs_source_info>()) };
}

/// Adds Tools > Sceneuron and Tools > Sceneuron settings. Requires the OBS frontend.
pub fn add_tools_menu() {
    if let Some(frontend) = sys::frontend() {
        // SAFETY: 'static names and callbacks matching obs_frontend_cb.
        unsafe {
            (frontend.obs_frontend_add_tools_menu_item)(c"Sceneuron".as_ptr(), Some(open_editor), ptr::null_mut());
            (frontend.obs_frontend_add_tools_menu_item)(
                c"Sceneuron settings".as_ptr(),
                Some(open_settings),
                ptr::null_mut(),
            );
        };
    }
}

/// Releases the settings source; called on exit.
pub fn release() {
    SETTINGS_SOURCE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take();
}

/// Opens the editor in the browser, or the settings window when the editor is not running, since
/// that window shows why.
unsafe extern "C" fn open_editor(data: *mut c_void) {
    guard((), || {
        let Some(plugin) = plugin::current() else {
            return;
        };
        let link = plugin.editor_link();
        if link.is_empty() {
            // SAFETY: called on the UI thread with the menu item's data, as OBS calls it.
            unsafe { open_settings(data) };
        } else {
            crate::open_url::open(&link);
        }
    })
}

unsafe extern "C" fn open_settings(_data: *mut c_void) {
    guard((), || {
        let (Some(plugin), Some(frontend)) = (plugin::current(), sys::frontend()) else {
            return;
        };
        let mut slot = SETTINGS_SOURCE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let settings = Data::new();
        let port = i64::from(plugin.port_setting());
        let automation = plugin.hub.read(|engine| engine.project().automation_enabled);
        settings.set_int("port", port);
        settings.set_bool("automation_enabled", automation);
        // Recorded first, so the update this push triggers finds nothing to apply.
        *SHOWN.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Shown {
            automation: Some(automation),
            port: Some(port),
        };
        let source = match slot.as_ref() {
            Some(source) => {
                // SAFETY: live source; the current values replace stale ones before showing the dialog.
                unsafe { (sys::obs().obs_source_update)(source.as_ptr(), settings.as_ptr()) };
                source.as_ptr()
            }
            None => {
                // SAFETY: registered id, valid name and settings; returns an owned reference or null.
                let created = unsafe {
                    (sys::obs().obs_source_create_private)(
                        SETTINGS_ID.as_ptr(),
                        c"Sceneuron".as_ptr(),
                        settings.as_ptr(),
                    )
                };
                // SAFETY: `created` is null or an owned reference.
                *slot = unsafe { SourceRef::from_owned(created) };
                match slot.as_ref() {
                    Some(source) => source.as_ptr(),
                    None => return,
                }
            }
        };
        drop(slot);
        // SAFETY: live source; the frontend opens its properties dialog on the UI thread.
        unsafe { (frontend.obs_frontend_open_source_properties)(source) };
    })
}

unsafe extern "C" fn get_name(_type_data: *mut c_void) -> *const c_char {
    c"Sceneuron settings".as_ptr()
}

unsafe extern "C" fn create(_settings: *mut sys::obs_data_t, _source: *mut sys::obs_source_t) -> *mut c_void {
    // The source keeps no per-instance state, but libobs treats null as a failed create.
    ptr::NonNull::<u8>::dangling().as_ptr().cast::<c_void>()
}

unsafe extern "C" fn destroy(_data: *mut c_void) {}

unsafe extern "C" fn update(_data: *mut c_void, settings: *mut sys::obs_data_t) {
    guard((), || {
        let Some(plugin) = plugin::current() else {
            return;
        };
        // SAFETY: libobs passes a live settings object.
        let (automation, port) = unsafe { (data_bool(settings, "automation_enabled"), data_int(settings, "port")) };
        let (automation_changed, port_changed) = {
            let mut shown = SHOWN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let changed = (shown.automation != Some(automation), shown.port != Some(port));
            *shown = Shown {
                automation: Some(automation),
                port: Some(port),
            };
            changed
        };
        if automation_changed {
            plugin.hub.update(|engine, now| engine.set_automation(automation, now));
        }
        if port_changed
            && let Ok(port) = u16::try_from(port)
            && port >= 1024
        {
            plugin.reconfigure(port, false);
        }
    })
}

unsafe extern "C" fn get_properties(_data: *mut c_void) -> *mut sys::obs_properties_t {
    guard(ptr::null_mut(), || {
        let props = Properties::new();
        let Some(plugin) = plugin::current() else {
            return props.into_raw();
        };
        props.info(
            "intro",
            "Sceneuron is an AI live producer: it watches your camera and drives your overlays, \
             while you stay in charge. Set up the AI in the editor.",
        );
        props.open_editor_button();
        let url = props.info("editor_url", &format!("Editor address: {}", plugin.editor_url()));
        props.describe(
            url,
            "To keep the editor inside OBS, add this address under Docks > Custom Browser Docks.",
        );
        props.bool(
            "automation_enabled",
            "Let rules and the AI change overlays automatically",
        );
        props.int("port", "Editor port on 127.0.0.1", 1024, 65535);
        let regenerate = props.button(
            "regenerate_token",
            "Create a new editor address",
            Some(regenerate_clicked),
        );
        props.describe(
            regenerate,
            "Use this if someone else saw the editor address. Docks must use the new one.",
        );
        if let Some(error) = plugin.server_error() {
            props.warning("server_error", &error);
        }
        props.into_raw()
    })
}

unsafe extern "C" fn regenerate_clicked(
    _props: *mut sys::obs_properties_t,
    _property: *mut sys::obs_property_t,
    _data: *mut c_void,
) -> bool {
    guard(false, || {
        if let Some(plugin) = plugin::current() {
            let port = plugin.port_setting();
            plugin.reconfigure(port, true);
        }
        true
    })
}
