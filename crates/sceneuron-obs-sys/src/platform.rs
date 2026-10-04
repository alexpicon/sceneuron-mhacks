// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Finding OBS exports in the running process without linking against OBS.

use core::ffi::{CStr, c_char, c_void};

unsafe extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

/// `RTLD_DEFAULT`, `((void *) -2)` on macOS: search every image the process has loaded.
const RTLD_DEFAULT: *mut c_void = core::ptr::without_provenance_mut(usize::MAX - 1);

/// Returns the address of the libobs export `name` in the running process, or null.
///
/// This is `dlsym(RTLD_DEFAULT, name)`.
pub fn find_libobs_symbol(name: &CStr) -> *mut c_void {
    global(name)
}

/// Returns the address of the obs-frontend-api export `name` in the running process, or null.
///
/// This is `dlsym(RTLD_DEFAULT, name)`.
pub fn find_frontend_symbol(name: &CStr) -> *mut c_void {
    global(name)
}

fn global(name: &CStr) -> *mut c_void {
    // SAFETY: RTLD_DEFAULT is a pseudo-handle dlsym always accepts, and `name` is a valid
    // NUL-terminated string that outlives the call. dlsym is thread-safe.
    unsafe { dlsym(RTLD_DEFAULT, name.as_ptr()) }
}
