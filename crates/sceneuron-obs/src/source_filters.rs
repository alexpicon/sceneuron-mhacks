// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Walking OBS's sources and the Sceneuron filters on them: which sources have a filter of a
//! type, adding one, and keeping a filter type on one source only.

use std::ffi::{CStr, c_void};

use sceneuron_obs_sys as sys;

use crate::ffi::{SourceRef, c_string, from_c};

/// Calls `f` for every public input source.
pub fn for_each_source(mut f: impl FnMut(*mut sys::obs_source_t)) {
    unsafe extern "C" fn trampoline(param: *mut c_void, source: *mut sys::obs_source_t) -> bool {
        // SAFETY: `param` is the `&mut dyn FnMut` passed below, valid for the enumeration.
        let f = unsafe { &mut *param.cast::<&mut dyn FnMut(*mut sys::obs_source_t)>() };
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(source))).is_ok()
    }
    let mut callback: &mut dyn FnMut(*mut sys::obs_source_t) = &mut f;
    // SAFETY: the trampoline matches the C signature and `callback` outlives the call.
    unsafe { (sys::obs().obs_enum_sources)(Some(trampoline), (&raw mut callback).cast::<c_void>()) };
}

/// Calls `f` for every public input source that is not removed. OBS keeps a removed source, and
/// its items in scenes, until the next video frame prunes them, so a check of what is in place
/// made right after a removal would still find it.
pub fn for_each_live_source(mut f: impl FnMut(*mut sys::obs_source_t)) {
    for_each_source(|source| {
        // SAFETY: `source` is valid during enumeration.
        if !unsafe { (sys::obs().obs_source_removed)(source) } {
            f(source);
        }
    });
}

/// The filters of type `id` on `source`, each with a reference of its own.
pub fn filters_of(source: *mut sys::obs_source_t, id: &CStr) -> Vec<SourceRef> {
    let mut search = Search { id, found: Vec::new() };
    // SAFETY: valid source; the callback adds a reference to each filter of the type.
    unsafe { (sys::obs().obs_source_enum_filters)(source, Some(collect), (&raw mut search).cast::<c_void>()) };
    search.found
}

/// Whether `source` has a filter of type `id`.
pub fn has_filter(source: *mut sys::obs_source_t, id: &CStr) -> bool {
    !filters_of(source, id).is_empty()
}

/// Adds a new filter of type `id`, called `name`, to `source`.
pub fn add_filter(source: *mut sys::obs_source_t, id: &CStr, name: &CStr) -> Result<(), String> {
    // SAFETY: registered filter id and valid name; returns an owned reference or null.
    let filter = unsafe {
        SourceRef::from_owned((sys::obs().obs_source_create)(
            id.as_ptr(),
            name.as_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        ))
    }
    .ok_or_else(|| format!("could not create the {} filter", name.to_string_lossy()))?;
    // SAFETY: both are live sources; the parent takes its own reference to the filter.
    unsafe { (sys::obs().obs_source_filter_add)(source, filter.as_ptr()) };
    Ok(())
}

/// Removes every filter of type `id` from every source except `keep`.
pub fn remove_filters_except(keep: *mut sys::obs_source_t, id: &CStr) {
    let mut found: Vec<(SourceRef, SourceRef)> = Vec::new();
    for_each_source(|source| {
        if source == keep {
            return;
        }
        for filter in filters_of(source, id) {
            // SAFETY: valid source during enumeration; get_ref returns an owned reference.
            if let Some(parent) = unsafe { SourceRef::from_owned((sys::obs().obs_source_get_ref)(source)) } {
                found.push((parent, filter));
            }
        }
    });
    for (parent, filter) in found {
        // SAFETY: both are live sources held by the references above.
        unsafe { (sys::obs().obs_source_filter_remove)(parent.as_ptr(), filter.as_ptr()) };
    }
}

/// The source with this UUID, with a reference of its own.
pub fn source_by_uuid(uuid: &str) -> Result<SourceRef, String> {
    let uuid = c_string(uuid);
    // SAFETY: valid C string; returns an owned reference or null.
    unsafe { SourceRef::from_owned((sys::obs().obs_get_source_by_uuid)(uuid.as_ptr())) }
        .ok_or_else(|| "that source no longer exists".to_owned())
}

/// The UUID, name, and unversioned type id of `source`.
pub fn identity(source: *mut sys::obs_source_t) -> (String, String, String) {
    // SAFETY: valid source; returned strings are owned by the source.
    unsafe {
        (
            from_c((sys::obs().obs_source_get_uuid)(source)),
            from_c((sys::obs().obs_source_get_name)(source)),
            from_c((sys::obs().obs_source_get_unversioned_id)(source)),
        )
    }
}

/// Whether `source` is of the registered type `id`.
pub fn is_type(source: *mut sys::obs_source_t, id: &CStr) -> bool {
    // SAFETY: `source` is live; the id string it returns is owned by libobs.
    let kind = unsafe { (sys::obs().obs_source_get_unversioned_id)(source) };
    // SAFETY: a non-null id is a valid NUL-terminated string.
    !kind.is_null() && unsafe { CStr::from_ptr(kind) } == id
}

struct Search<'a> {
    id: &'a CStr,
    found: Vec<SourceRef>,
}

unsafe extern "C" fn collect(_parent: *mut sys::obs_source_t, child: *mut sys::obs_source_t, param: *mut c_void) {
    // SAFETY: `param` points to the caller's search for the duration of the enumeration.
    let search = unsafe { &mut *param.cast::<Search>() };
    if is_type(child, search.id) {
        // SAFETY: `child` is live during enumeration; get_ref returns an owned reference.
        if let Some(filter) = unsafe { SourceRef::from_owned((sys::obs().obs_source_get_ref)(child)) } {
            search.found.push(filter);
        }
    }
}
