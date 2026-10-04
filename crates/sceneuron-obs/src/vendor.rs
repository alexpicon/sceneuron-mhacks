// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! obs-websocket vendor "sceneuron": lets Stream Deck, Streamer.bot, and scripts drive Sceneuron.

use std::ffi::{CStr, c_void};
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

use sceneuron_core::Presence;
use sceneuron_obs_sys as sys;

use crate::ffi::{self, Data, guard};
use crate::plugin;
use crate::vendor_json::obs_data_view;
use crate::vendor_requests::{self, REQUESTS, Request};

static VENDOR: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static PROC_HANDLER: AtomicPtr<sys::proc_handler_t> = AtomicPtr::new(ptr::null_mut());

/// The callback record obs-websocket copies when a request is registered.
#[repr(C)]
struct RequestCallback {
    callback: unsafe extern "C" fn(*mut sys::obs_data_t, *mut sys::obs_data_t, *mut c_void),
    priv_data: *mut c_void,
}

/// Owned calldata, freed on drop like the C `calldata_free`.
struct CallData(sys::calldata_t);

impl CallData {
    fn new() -> CallData {
        CallData(sys::calldata_t {
            stack: ptr::null_mut(),
            size: 0,
            capacity: 0,
            fixed: false,
        })
    }

    fn set_ptr(&mut self, name: &CStr, value: *mut c_void) {
        // SAFETY: libobs copies `size_of` bytes from the pointer variable.
        unsafe {
            (sys::obs().calldata_set_data)(
                &mut self.0,
                name.as_ptr(),
                (&raw const value).cast::<c_void>(),
                size_of::<*mut c_void>(),
            )
        };
    }

    fn set_string(&mut self, name: &CStr, value: &CStr) {
        let bytes = value.to_bytes_with_nul();
        // SAFETY: libobs copies the string including its terminator.
        unsafe { (sys::obs().calldata_set_data)(&mut self.0, name.as_ptr(), bytes.as_ptr().cast(), bytes.len()) };
    }

    fn get_ptr(&self, name: &CStr) -> *mut c_void {
        let mut out: *mut c_void = ptr::null_mut();
        // SAFETY: `out` has exactly the size requested.
        unsafe {
            (sys::obs().calldata_get_data)(&self.0, name.as_ptr(), (&raw mut out).cast(), size_of::<*mut c_void>())
        };
        out
    }

    fn call(&mut self, handler: *mut sys::proc_handler_t, procedure: &CStr) -> bool {
        // SAFETY: live proc handler and calldata.
        unsafe { (sys::obs().proc_handler_call)(handler, procedure.as_ptr(), &mut self.0) }
    }
}

impl Drop for CallData {
    fn drop(&mut self) {
        if !self.0.fixed && !self.0.stack.is_null() {
            // SAFETY: libobs allocated the stack with bmalloc.
            unsafe { (sys::obs().bfree)(self.0.stack.cast::<c_void>()) };
        }
    }
}

/// Registers the vendor and its requests. Call from `obs_module_post_load`.
pub fn register() {
    // SAFETY: the global proc handler always exists after libobs startup.
    let global = unsafe { (sys::obs().obs_get_proc_handler)() };
    let mut lookup = CallData::new();
    if global.is_null() || !lookup.call(global, c"obs_websocket_api_get_ph") {
        ffi::info("obs-websocket is not available; remote control requests are disabled");
        return;
    }
    let handler = lookup.get_ptr(c"ph").cast::<sys::proc_handler_t>();
    if handler.is_null() {
        return;
    }
    let mut vendor_call = CallData::new();
    vendor_call.set_string(c"name", c"sceneuron");
    vendor_call.call(handler, c"vendor_register");
    let vendor = vendor_call.get_ptr(c"vendor");
    if vendor.is_null() {
        ffi::warn("could not register the obs-websocket vendor");
        return;
    }
    PROC_HANDLER.store(handler, Ordering::SeqCst);
    VENDOR.store(vendor, Ordering::SeqCst);
    for request in REQUESTS {
        let callback = RequestCallback {
            callback: handle_request,
            priv_data: request.as_ptr().cast_mut().cast(),
        };
        let mut call = CallData::new();
        call.set_ptr(c"vendor", vendor);
        call.set_string(c"type", request);
        call.set_ptr(c"callback", (&raw const callback).cast_mut().cast());
        call.call(handler, c"vendor_request_register");
    }
    ffi::info("registered obs-websocket vendor \"sceneuron\"");
}

/// Emits a `PresenceChanged` vendor event.
pub fn emit_presence(presence: Presence) {
    let (handler, vendor) = (PROC_HANDLER.load(Ordering::SeqCst), VENDOR.load(Ordering::SeqCst));
    if handler.is_null() || vendor.is_null() {
        return;
    }
    let data = Data::new();
    data.set_string("presence", &format!("{presence:?}").to_lowercase());
    let mut call = CallData::new();
    call.set_ptr(c"vendor", vendor);
    call.set_string(c"type", c"PresenceChanged");
    call.set_ptr(c"data", data.as_ptr().cast());
    call.call(handler, c"vendor_event_emit");
}

unsafe extern "C" fn handle_request(request: *mut sys::obs_data_t, response: *mut sys::obs_data_t, name: *mut c_void) {
    guard((), || {
        // SAFETY: `name` is one of the 'static request names registered above.
        let name = unsafe { CStr::from_ptr(name.cast()) }.to_string_lossy();
        // SAFETY: obs-websocket passes live request and response objects for the call.
        let data = unsafe { ffi::from_c((sys::obs().obs_data_get_json)(request)) };
        match dispatch(&name, &data) {
            Ok(snapshot) => {
                let exact = ffi::c_string(&snapshot);
                let view = serde_json::from_str(&snapshot).map(|value| obs_data_view(&value).to_string());
                let state = Data::from_json(&view.unwrap_or_default());
                // SAFETY: live response object and a valid C string, which libobs copies; libobs
                // adds its own reference to `state`.
                unsafe {
                    (sys::obs().obs_data_set_string)(response, c"state_json".as_ptr(), exact.as_ptr());
                    (sys::obs().obs_data_set_obj)(response, c"state".as_ptr(), state.as_ptr());
                }
            }
            Err(error) => {
                let error = ffi::c_string(&error);
                // SAFETY: live response object and valid C strings, which libobs copies.
                unsafe { (sys::obs().obs_data_set_string)(response, c"error".as_ptr(), error.as_ptr()) };
            }
        }
    })
}

/// Runs one request and returns the new snapshot JSON or an error message.
fn dispatch(name: &str, data: &str) -> Result<String, String> {
    let plugin = plugin::current().ok_or("Sceneuron is shutting down")?;
    let hub = &plugin.hub;
    match vendor_requests::parse(name, data)? {
        Request::GetState => {}
        Request::AnalyzeNow => {
            let settings = plugin.ai.settings_view();
            if !(settings.enabled && settings.consent) {
                return Err("turn on scene understanding in the editor first".to_owned());
            }
            plugin.ai.director().analyze_now();
        }
        Request::Change(change) => {
            hub.try_update(|engine, now| change.apply(engine, now))
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(hub.snapshot_json())
}
