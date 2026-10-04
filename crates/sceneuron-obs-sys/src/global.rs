// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The process-wide function tables.

use core::ffi::{CStr, c_void};
use core::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

use crate::error::LoadError;
use crate::frontend::Frontend;
use crate::libobs::Libobs;
use crate::platform::{find_frontend_symbol, find_libobs_symbol};

struct Tables {
    libobs: Libobs,
    frontend: Option<Frontend>,
}

/// Null until the first `load` or `install`. A published pointer is never freed, so the
/// references handed out by `obs` and `frontend` stay valid for the rest of the process.
static TABLES: AtomicPtr<Tables> = AtomicPtr::new(ptr::null_mut());

/// Resolves both tables the way [`load`] does: the libobs table is required, and the frontend
/// table is `None` when any of its functions is missing (a headless libobs host).
///
/// # Errors
///
/// Returns a [`LoadError`] naming the first libobs function `libobs_lookup` did not find.
pub fn resolve_tables(
    libobs_lookup: impl Fn(&CStr) -> *mut c_void,
    frontend_lookup: impl Fn(&CStr) -> *mut c_void,
) -> Result<(Libobs, Option<Frontend>), LoadError> {
    let libobs = Libobs::resolve(libobs_lookup)?;
    Ok((libobs, Frontend::resolve(frontend_lookup).ok()))
}

/// Resolves the tables from the running process and makes them available through [`obs`] and
/// [`frontend`]. Does nothing when tables are already in place, so calling it again is cheap.
///
/// # Errors
///
/// Returns a [`LoadError`] naming the first libobs function the process does not export, which
/// means the code is not running inside OBS or the OBS version is incompatible.
pub fn load() -> Result<(), LoadError> {
    if current().is_some() {
        return Ok(());
    }
    let (libobs, frontend) = resolve_tables(find_libobs_symbol, find_frontend_symbol)?;
    let fresh = Box::into_raw(Box::new(Tables { libobs, frontend }));
    if TABLES
        .compare_exchange(ptr::null_mut(), fresh, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        // SAFETY: `fresh` came from Box::into_raw above and lost the race, so it was never
        // published and this is its only owner.
        drop(unsafe { Box::from_raw(fresh) });
    }
    Ok(())
}

/// Makes the given tables available through [`obs`] and [`frontend`], replacing any tables that
/// are already in place. Meant for tests and for embedders that resolve symbols themselves.
///
/// Replaced tables are leaked on purpose: references previously returned by [`obs`] and
/// [`frontend`] are `'static` and must stay valid.
pub fn install(libobs: Libobs, frontend: Option<Frontend>) {
    let fresh = Box::into_raw(Box::new(Tables { libobs, frontend }));
    TABLES.swap(fresh, Ordering::AcqRel);
}

/// The libobs functions.
///
/// # Panics
///
/// Panics when neither [`load`] nor [`install`] has succeeded. Never call it from code that can
/// run before a successful `load`, because a panic must not unwind into OBS.
pub fn obs() -> &'static Libobs {
    match try_obs() {
        Some(libobs) => libobs,
        None => panic!("sceneuron-obs-sys: OBS functions are not loaded; call load() or install() first"),
    }
}

/// The libobs functions, or `None` before [`load`] or [`install`] has succeeded.
pub fn try_obs() -> Option<&'static Libobs> {
    current().map(|tables| &tables.libobs)
}

/// The frontend functions, or `None` when the host does not export them or nothing is loaded yet.
pub fn frontend() -> Option<&'static Frontend> {
    current().and_then(|tables| tables.frontend.as_ref())
}

fn current() -> Option<&'static Tables> {
    let tables = TABLES.load(Ordering::Acquire);
    // SAFETY: TABLES only ever holds null or a pointer from Box::into_raw that is never freed once
    // published; the Acquire load pairs with the AcqRel publication, so the contents are visible.
    unsafe { tables.as_ref() }
}
