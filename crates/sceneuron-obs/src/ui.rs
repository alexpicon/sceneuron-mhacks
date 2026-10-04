// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Runs work on the OBS UI thread.
//!
//! obs-browser initializes CEF on whichever thread creates the first browser source, and CEF must
//! start on the main thread on macOS. Scene and source changes requested by the editor therefore
//! run on the UI thread instead of the server thread that received the request.
//!
//! [`run`] waits for the result, so only threads that hold no libobs locks may use it, such as
//! the server's request threads. Source callbacks use [`spawn`], which never waits.

use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};

use sceneuron_obs_sys as sys;

/// One queued call: the closure to run and the slot for its result.
struct Task<F, T> {
    work: Option<F>,
    result: Option<T>,
}

/// Runs `work` on the UI thread and waits for its result.
///
/// Returns `None` when no OBS frontend runs a UI task loop (for example headless libobs) or when
/// `work` panicked.
pub fn run<T: Send, F: FnOnce() -> T + Send>(work: F) -> Option<T> {
    // SAFETY: obs_in_task_thread only reads a thread-local flag.
    if unsafe { (sys::obs().obs_in_task_thread)(sys::OBS_TASK_UI) } {
        return catch_unwind(AssertUnwindSafe(work)).ok();
    }
    sys::frontend()?;
    let mut task = Task {
        work: Some(work),
        result: None,
    };
    // SAFETY: `wait` is true, so `task` outlives the call; the trampoline matches obs_task_t.
    unsafe {
        (sys::obs().obs_queue_task)(
            sys::OBS_TASK_UI,
            Some(trampoline::<F, T>),
            (&raw mut task).cast::<c_void>(),
            true,
        );
    }
    task.result
}

unsafe extern "C" fn trampoline<F: FnOnce() -> T, T>(param: *mut c_void) {
    // SAFETY: `param` is the `Task` that `run` keeps alive until this call returns.
    let task = unsafe { &mut *param.cast::<Task<F, T>>() };
    if let Some(work) = task.work.take() {
        task.result = catch_unwind(AssertUnwindSafe(work)).ok();
    }
}

/// Queues `work` on the UI thread without waiting, or runs it at once when already there.
///
/// Returns false, dropping `work`, when no OBS frontend runs a UI task loop. Work still queued
/// when OBS quits is dropped without running.
pub fn spawn<F: FnOnce() + Send + 'static>(work: F) -> bool {
    // SAFETY: obs_in_task_thread only reads a thread-local flag.
    if unsafe { (sys::obs().obs_in_task_thread)(sys::OBS_TASK_UI) } {
        let _ = catch_unwind(AssertUnwindSafe(work));
        return true;
    }
    if sys::frontend().is_none() {
        return false;
    }
    let work: Box<Box<dyn FnOnce() + Send>> = Box::new(Box::new(work));
    // SAFETY: the trampoline takes back ownership of the box exactly once.
    unsafe {
        (sys::obs().obs_queue_task)(
            sys::OBS_TASK_UI,
            Some(spawned),
            Box::into_raw(work).cast::<c_void>(),
            false,
        );
    }
    true
}

unsafe extern "C" fn spawned(param: *mut c_void) {
    // SAFETY: `param` is the box `spawn` leaked for this task.
    let work = unsafe { Box::from_raw(param.cast::<Box<dyn FnOnce() + Send>>()) };
    let _ = catch_unwind(AssertUnwindSafe(*work));
}
