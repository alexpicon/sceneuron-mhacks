// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Small safe helpers over the raw libobs bindings: logging, C strings, and owned handles.

use std::ffi::{CStr, CString, c_char, c_void};

use sceneuron_obs_sys as sys;

/// Writes a line to the OBS log with a `[sceneuron]` prefix.
pub fn log(level: i32, message: &str) {
    let line = c_string(&format!("[sceneuron] {message}"));
    // SAFETY: `blog` is resolved at load; the "%s" format consumes exactly one C string argument.
    unsafe { (sys::obs().blog)(level, c"%s".as_ptr(), line.as_ptr()) };
}

pub fn info(message: &str) {
    log(sys::LOG_INFO, message);
}

pub fn warn(message: &str) {
    log(sys::LOG_WARNING, message);
}

/// Converts to a C string, dropping interior NUL bytes instead of failing.
pub fn c_string(text: &str) -> CString {
    CString::new(text.replace('\0', "")).unwrap_or_default()
}

/// Copies a borrowed C string; null becomes an empty string.
///
/// # Safety
/// `text` must be null or point to a NUL-terminated string valid for the call.
pub unsafe fn from_c(text: *const c_char) -> String {
    if text.is_null() {
        return String::new();
    }
    // SAFETY: the caller guarantees a valid NUL-terminated string.
    unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned()
}

/// Copies and frees a string that libobs allocated with `bmalloc`.
///
/// # Safety
/// `text` must be null or a `bmalloc`-allocated, NUL-terminated string owned by the caller.
pub unsafe fn take_bmalloc_string(text: *mut c_char) -> String {
    // SAFETY: valid per the caller's contract.
    let copy = unsafe { from_c(text) };
    if !text.is_null() {
        // SAFETY: libobs allocated the string and ownership passed to us.
        unsafe { (sys::obs().bfree)(text.cast::<c_void>()) };
    }
    copy
}

/// An owned `obs_data_t` reference, released on drop.
pub struct Data(*mut sys::obs_data_t);

impl Data {
    pub fn new() -> Data {
        // SAFETY: obs_data_create has no preconditions and returns a new reference.
        Data(unsafe { (sys::obs().obs_data_create)() })
    }

    /// Parses JSON into a new data object; invalid JSON yields an empty object.
    pub fn from_json(json: &str) -> Data {
        let text = c_string(json);
        // SAFETY: the argument is a valid C string; the result is a new reference or null.
        let data = unsafe { (sys::obs().obs_data_create_from_json)(text.as_ptr()) };
        if data.is_null() { Data::new() } else { Data(data) }
    }

    /// Wraps a reference the caller already owns.
    ///
    /// # Safety
    /// `data` must be null or an owned `obs_data_t` reference.
    pub unsafe fn from_owned(data: *mut sys::obs_data_t) -> Option<Data> {
        (!data.is_null()).then_some(Data(data))
    }

    pub fn as_ptr(&self) -> *mut sys::obs_data_t {
        self.0
    }

    pub fn json(&self) -> String {
        // SAFETY: self.0 is a live reference; the returned string is owned by the data object.
        unsafe { from_c((sys::obs().obs_data_get_json)(self.0)) }
    }

    pub fn set_string(&self, key: &str, value: &str) {
        let (key, value) = (c_string(key), c_string(value));
        // SAFETY: live data object and valid C strings.
        unsafe { (sys::obs().obs_data_set_string)(self.0, key.as_ptr(), value.as_ptr()) };
    }

    pub fn set_int(&self, key: &str, value: i64) {
        let key = c_string(key);
        // SAFETY: live data object and valid C string.
        unsafe { (sys::obs().obs_data_set_int)(self.0, key.as_ptr(), value) };
    }

    pub fn set_bool(&self, key: &str, value: bool) {
        let key = c_string(key);
        // SAFETY: live data object and valid C string.
        unsafe { (sys::obs().obs_data_set_bool)(self.0, key.as_ptr(), value) };
    }
}

impl Default for Data {
    fn default() -> Self {
        Data::new()
    }
}

impl Drop for Data {
    fn drop(&mut self) {
        // SAFETY: self.0 is an owned reference released exactly once.
        unsafe { (sys::obs().obs_data_release)(self.0) };
    }
}

/// Reads an integer setting from a borrowed data object.
///
/// # Safety
/// `data` must be a live `obs_data_t`.
pub unsafe fn data_int(data: *mut sys::obs_data_t, key: &str) -> i64 {
    let key = c_string(key);
    // SAFETY: valid per the caller's contract.
    unsafe { (sys::obs().obs_data_get_int)(data, key.as_ptr()) }
}

/// Reads a boolean setting from a borrowed data object.
///
/// # Safety
/// `data` must be a live `obs_data_t`.
pub unsafe fn data_bool(data: *mut sys::obs_data_t, key: &str) -> bool {
    let key = c_string(key);
    // SAFETY: valid per the caller's contract.
    unsafe { (sys::obs().obs_data_get_bool)(data, key.as_ptr()) }
}

/// An owned source reference, released on drop.
pub struct SourceRef(*mut sys::obs_source_t);

// SAFETY: libobs source references are reference counted with atomic operations and may be
// released from any thread.
unsafe impl Send for SourceRef {}
// SAFETY: shared access only passes the pointer to thread-safe libobs functions.
unsafe impl Sync for SourceRef {}

impl SourceRef {
    /// Wraps a reference the caller already owns.
    ///
    /// # Safety
    /// `source` must be null or an owned source reference.
    pub unsafe fn from_owned(source: *mut sys::obs_source_t) -> Option<SourceRef> {
        (!source.is_null()).then_some(SourceRef(source))
    }

    pub fn as_ptr(&self) -> *mut sys::obs_source_t {
        self.0
    }
}

impl Drop for SourceRef {
    fn drop(&mut self) {
        // SAFETY: self.0 is an owned reference released exactly once.
        unsafe { (sys::obs().obs_source_release)(self.0) };
    }
}

/// A weak reference to a source: it does not keep the source alive.
pub struct WeakSource(*mut sys::obs_weak_source_t);

// SAFETY: libobs weak references are reference counted with atomic operations and may be used
// and released from any thread.
unsafe impl Send for WeakSource {}
// SAFETY: shared access only passes the pointer to thread-safe libobs functions.
unsafe impl Sync for WeakSource {}

impl WeakSource {
    /// A weak reference to a live source.
    pub fn new(source: *mut sys::obs_source_t) -> WeakSource {
        // SAFETY: the caller passes a live source; libobs returns a new weak reference.
        WeakSource(unsafe { (sys::obs().obs_source_get_weak_source)(source) })
    }

    /// A strong reference, or `None` once the source is being destroyed.
    pub fn upgrade(&self) -> Option<SourceRef> {
        // SAFETY: a live weak reference; libobs returns an owned reference or null.
        unsafe { SourceRef::from_owned((sys::obs().obs_weak_source_get_source)(self.0)) }
    }
}

impl Drop for WeakSource {
    fn drop(&mut self) {
        // SAFETY: self.0 is a weak reference released exactly once; libobs accepts null.
        unsafe { (sys::obs().obs_weak_source_release)(self.0) };
    }
}

/// Runs `f` and turns a panic into `fallback`, so panics never cross into C.
pub fn guard<T>(fallback: T, f: impl FnOnce() -> T) -> T {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(_) => {
            if sys::try_obs().is_some() {
                log(sys::LOG_ERROR, "recovered from an internal error in a callback");
            }
            fallback
        }
    }
}
