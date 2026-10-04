// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! State of each "Sceneuron Overlay" source and its private browser child.
//!
//! The page URL carries the overlay token, so it lives only in the private child's settings and is
//! never written to the scene collection. libobs calls the source from the UI thread (update) and
//! the graphics thread (tick, render) at the same time, so every field is atomic or locked.
//!
//! The child is created on the UI thread without making the caller wait, because libobs and
//! scripts create sources on threads that hold locks the UI thread may need. Until it exists the
//! overlay draws nothing. Spoken cards the child plays reach OBS through the overlay's own audio.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use sceneuron_obs_sys as sys;

use crate::ffi::{self, Data, SourceRef, WeakSource, data_int};
use crate::ui;
use crate::{overlay_audio, plugin};

/// Live overlays, so their browsers can be closed while OBS can still finish closing them.
static OVERLAYS: Mutex<Vec<usize>> = Mutex::new(Vec::new());
/// Set once OBS begins to exit; no new browser children are created after that.
static CLOSING: AtomicBool = AtomicBool::new(false);

pub struct Overlay {
    pub source: *mut sys::obs_source_t,
    width: AtomicU32,
    height: AtomicU32,
    url_generation: AtomicU64,
    child: Mutex<Option<SourceRef>>,
    /// A child is being created on the UI thread.
    creating: AtomicBool,
}

impl Overlay {
    /// Creates the state, registers it, and starts its browser child.
    pub fn create(settings: *mut sys::obs_data_t, source: *mut sys::obs_source_t) -> *mut Overlay {
        let overlay = Box::into_raw(Box::new(Overlay {
            source,
            width: AtomicU32::new(1920),
            height: AtomicU32::new(1080),
            url_generation: AtomicU64::new(0),
            child: Mutex::new(None),
            creating: AtomicBool::new(false),
        }));
        lock(&OVERLAYS).push(overlay as usize);
        // SAFETY: just allocated above; only shared references are created from here on.
        let state = unsafe { &*overlay };
        state.read_size(settings);
        state.ensure_child();
        overlay
    }

    /// Unregisters and frees the state.
    ///
    /// # Safety
    /// `overlay` must come from [`Overlay::create`] and must not be used afterwards.
    pub unsafe fn destroy(overlay: *mut Overlay) {
        lock(&OVERLAYS).retain(|entry| *entry != overlay as usize);
        // SAFETY: per the function contract, this is the last use of the allocation.
        let state = unsafe { Box::from_raw(overlay) };
        state.detach_child();
    }

    pub fn width(&self) -> u32 {
        self.width.load(Ordering::Relaxed)
    }

    pub fn height(&self) -> u32 {
        self.height.load(Ordering::Relaxed)
    }

    pub fn read_size(&self, settings: *mut sys::obs_data_t) {
        // SAFETY: libobs passes a live settings object.
        let (width, height) = unsafe { (data_int(settings, "width"), data_int(settings, "height")) };
        self.width.store(width.clamp(16, 8192) as u32, Ordering::Relaxed);
        self.height.store(height.clamp(16, 8192) as u32, Ordering::Relaxed);
    }

    /// A new reference to the browser child, if there is one.
    pub fn child(&self) -> Option<SourceRef> {
        let child = lock(&self.child);
        // SAFETY: the child is live while the lock holds our reference; get_ref adds one.
        child
            .as_ref()
            .and_then(|child| unsafe { SourceRef::from_owned((sys::obs().obs_source_get_ref)(child.as_ptr())) })
    }

    /// Starts creating the browser child on the UI thread, where obs-browser can start CEF
    /// safely, unless it exists or is on its way. Never waits.
    pub fn ensure_child(&self) {
        if CLOSING.load(Ordering::SeqCst) || lock(&self.child).is_some() || self.creating.swap(true, Ordering::SeqCst) {
            return;
        }
        let weak = WeakSource::new(self.source);
        if !ui::spawn(move || install_child(&weak)) {
            self.creating.store(false, Ordering::SeqCst);
            ffi::warn("the overlay needs the OBS Browser Source plugin and the OBS window");
        }
    }

    /// Stops taking the child's audio, removes the child from this source's active tree, and
    /// releases it.
    fn detach_child(&self) {
        if let Some(child) = lock(&self.child).take() {
            // SAFETY: both sources are live; these balance `install_child`.
            unsafe {
                overlay_audio::stop(self.source, child.as_ptr());
                (sys::obs().obs_source_remove_active_child)(self.source, child.as_ptr());
            }
        }
    }

    /// Pushes the current URL and size to the browser child when either changed.
    pub fn refresh_child(&self, force: bool) {
        let Some(plugin) = plugin::current() else {
            return;
        };
        let generation = plugin.url_generation();
        if !force && self.url_generation.load(Ordering::SeqCst) == generation {
            return;
        }
        let Some(child) = self.child() else {
            return;
        };
        self.url_generation.store(generation, Ordering::SeqCst);
        let settings = child_settings(&plugin.overlay_url(), self.width(), self.height());
        // SAFETY: live child source and settings object.
        unsafe { (sys::obs().obs_source_update)(child.as_ptr(), settings.as_ptr()) };
    }
}

/// Closes every overlay's browser while OBS still pumps the events CEF needs to finish closing.
///
/// `exiting` also stops new browsers from being created for the rest of the process.
pub fn release_browsers(exiting: bool) {
    if exiting {
        CLOSING.store(true, Ordering::SeqCst);
    }
    for entry in lock(&OVERLAYS).iter() {
        // SAFETY: registered overlays stay allocated until `destroy` removes them under this lock.
        unsafe { &*(*entry as *const Overlay) }.detach_child();
    }
}

/// Creates and installs the child of the overlay `weak` points to, on the UI thread.
///
/// The strong reference held here keeps the overlay, and so its state, alive until the end.
fn install_child(weak: &WeakSource) {
    let Some(source) = weak.upgrade() else {
        return;
    };
    let state = lock(&OVERLAYS)
        .iter()
        .map(|entry| *entry as *const Overlay)
        // SAFETY: registered overlays stay allocated until `destroy`, which cannot run while
        // `source` holds a reference.
        .find(|overlay| unsafe { &**overlay }.source == source.as_ptr());
    let Some(state) = state else {
        return;
    };
    // SAFETY: see above.
    let state = unsafe { &*state };
    let created = match plugin::current() {
        Some(plugin) if !CLOSING.load(Ordering::SeqCst) => {
            state.url_generation.store(plugin.url_generation(), Ordering::SeqCst);
            create_child(&plugin.overlay_url(), state.width(), state.height())
        }
        _ => None,
    };
    match created {
        Some(child) => {
            let mut slot = lock(&state.child);
            if slot.is_none() && !CLOSING.load(Ordering::SeqCst) {
                // The parent may already be showing; this gives the late child the same counts.
                // SAFETY: both sources are live, and `detach_child` stops the audio before the
                // parent is destroyed or the child released.
                unsafe {
                    (sys::obs().obs_source_add_active_child)(state.source, child.as_ptr());
                    overlay_audio::start(state.source, child.as_ptr());
                }
                *slot = Some(child);
            }
        }
        None if !CLOSING.load(Ordering::SeqCst) => {
            ffi::warn("the overlay needs the OBS Browser Source plugin");
        }
        None => {}
    }
    state.creating.store(false, Ordering::SeqCst);
}

/// Settings for the private browser source that draws the overlay page.
fn child_settings(url: &str, width: u32, height: u32) -> Data {
    let data = Data::new();
    data.set_string("url", url);
    data.set_bool("is_local_file", false);
    data.set_int("width", i64::from(width));
    data.set_int("height", i64::from(height));
    // The page is transparent on its own; obs-browser would inject custom CSS as an inline
    // style, which the page's Content-Security-Policy refuses.
    data.set_string("css", "");
    // obs-browser's "Control audio via OBS": spoken cards go to OBS, not the speakers.
    data.set_bool("reroute_audio", true);
    data.set_bool("shutdown", false);
    data.set_bool("restart_when_active", false);
    data.set_int("webpage_control_level", 0);
    data
}

fn create_child(url: &str, width: u32, height: u32) -> Option<SourceRef> {
    let settings = child_settings(url, width, height);
    // SAFETY: valid id, name, and settings; returns an owned reference or null.
    let child = unsafe {
        (sys::obs().obs_source_create_private)(
            c"browser_source".as_ptr(),
            c"Sceneuron overlay renderer".as_ptr(),
            settings.as_ptr(),
        )
    };
    // SAFETY: `child` is null or an owned reference.
    unsafe { SourceRef::from_owned(child) }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
