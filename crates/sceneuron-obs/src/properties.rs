// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A small builder over `obs_properties_t` for the plugin's settings dialogs.

use std::ffi::c_void;

use sceneuron_obs_sys as sys;

use crate::ffi::{c_string, guard};
use crate::plugin;

/// Properties under construction; ownership passes to libobs with [`Properties::into_raw`].
pub struct Properties(*mut sys::obs_properties_t);

impl Properties {
    pub fn new() -> Properties {
        // SAFETY: no preconditions; libobs takes ownership when the pointer is returned to it.
        Properties(unsafe { (sys::obs().obs_properties_create)() })
    }

    pub fn into_raw(self) -> *mut sys::obs_properties_t {
        self.0
    }

    /// Read-only text, shown as an informational line.
    pub fn info(&self, name: &str, text: &str) -> *mut sys::obs_property_t {
        let (name, text) = (c_string(name), c_string(text));
        // SAFETY: live properties object and valid C strings, which libobs copies.
        unsafe { (sys::obs().obs_properties_add_text)(self.0, name.as_ptr(), text.as_ptr(), sys::OBS_TEXT_INFO) }
    }

    /// Informational text styled as a warning.
    pub fn warning(&self, name: &str, text: &str) {
        let property = self.info(name, text);
        // SAFETY: `property` was just created on this object.
        unsafe { (sys::obs().obs_property_text_set_info_type)(property, sys::OBS_TEXT_INFO_WARNING) };
    }

    pub fn bool(&self, name: &str, label: &str) {
        let (name, label) = (c_string(name), c_string(label));
        // SAFETY: live properties object and valid C strings.
        unsafe { (sys::obs().obs_properties_add_bool)(self.0, name.as_ptr(), label.as_ptr()) };
    }

    pub fn int(&self, name: &str, label: &str, min: i32, max: i32) {
        let (name, label) = (c_string(name), c_string(label));
        // SAFETY: live properties object and valid C strings.
        unsafe { (sys::obs().obs_properties_add_int)(self.0, name.as_ptr(), label.as_ptr(), min, max, 1) };
    }

    pub fn button(&self, name: &str, label: &str, clicked: sys::obs_property_clicked_t) -> *mut sys::obs_property_t {
        let (name, label) = (c_string(name), c_string(label));
        // SAFETY: live properties object, valid C strings, and a matching callback.
        unsafe {
            (sys::obs().obs_properties_add_button2)(
                self.0,
                name.as_ptr(),
                label.as_ptr(),
                clicked,
                std::ptr::null_mut(),
            )
        }
    }

    pub fn describe(&self, property: *mut sys::obs_property_t, text: &str) {
        let text = c_string(text);
        // SAFETY: `property` belongs to this object; libobs copies the text.
        unsafe { (sys::obs().obs_property_set_long_description)(property, text.as_ptr()) };
    }

    /// A button that opens the web editor in the default browser.
    pub fn open_editor_button(&self) {
        let button = self.button("open_editor", "Open the Sceneuron editor", Some(open_editor_clicked));
        self.describe(
            button,
            "Opens the layout, rule, and control editor in your web browser.",
        );
    }
}

unsafe extern "C" fn open_editor_clicked(
    _props: *mut sys::obs_properties_t,
    _property: *mut sys::obs_property_t,
    _data: *mut c_void,
) -> bool {
    guard(false, || {
        if let Some(plugin) = plugin::current() {
            crate::open_url::open(&plugin.editor_link());
        }
        false
    })
}
