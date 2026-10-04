// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! OBS hotkeys: registers, describes, and unregisters the hotkeys [`hotkey_set`] lists, runs
//! them when pressed, and keeps their bindings.
//!
//! Plugin hotkey bindings are not saved by OBS itself, so they are stored in the scene
//! collection through the frontend save callback.
//!
//! Hotkeys follow the project. Project changes can come from any thread, including a libobs
//! hotkey callback, so the follow-up runs on the UI thread, one at a time.
//!
//! The state lock is never held across a libobs hotkey call: libobs runs [`pressed`] under its
//! own hotkey lock, and [`pressed`] takes the state lock.

use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use sceneuron_obs_sys as sys;

use crate::ffi::{Data, c_string, guard};
use crate::hotkey_set::{self, Action, Hotkey};
use crate::plugin;
use crate::ui;

const SAVE_KEY: &str = "sceneuron-hotkeys";

struct Registered {
    id: sys::obs_hotkey_id,
    action: Action,
    /// What OBS Settings > Hotkeys shows; updated when an element, deck, or card is renamed.
    description: String,
}

#[derive(Default)]
struct State {
    by_name: BTreeMap<String, Registered>,
    saved_bindings: Option<String>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

/// Held for a whole sync, so two syncs never interleave their changes.
static SYNC: Mutex<()> = Mutex::new(());

/// A sync is queued on the UI thread; later requests until it runs are covered by it.
static SYNC_QUEUED: AtomicBool = AtomicBool::new(false);

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut guard = STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    f(guard.get_or_insert_with(State::default))
}

/// Registers the fixed hotkeys and those of the current project.
pub fn register_all() {
    sync();
}

/// Brings the hotkeys up to date with the project soon, on the UI thread.
pub fn request_sync() {
    if SYNC_QUEUED.swap(true, Ordering::SeqCst) {
        return;
    }
    let queued = ui::spawn(|| {
        SYNC_QUEUED.store(false, Ordering::SeqCst);
        sync();
    });
    if !queued {
        // Without a frontend there is no UI thread to defer to.
        SYNC_QUEUED.store(false, Ordering::SeqCst);
        sync();
    }
}

/// Adds hotkeys the project gained, removes those it lost, and updates renamed descriptions.
fn sync() {
    let _sync = SYNC.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(plugin) = plugin::current() else {
        return;
    };
    let wanted = plugin.hub.read(|engine| hotkey_set::wanted(engine.project()));
    let (registered, saved) = with_state(|state| {
        let registered: BTreeMap<String, String> = state
            .by_name
            .iter()
            .map(|(name, registered)| (name.clone(), registered.description.clone()))
            .collect();
        (registered, state.saved_bindings.clone())
    });
    let plan = hotkey_set::plan_sync(&registered, wanted);
    for name in plan.remove {
        unregister(&name);
    }
    for (name, description) in plan.describe {
        describe(&name, &description);
    }
    // Parsed once per sync: a project can add hundreds of card hotkeys at once.
    let saved = saved.map(|json| Data::from_json(&json));
    for (name, hotkey) in plan.add {
        register(&name, hotkey, saved.as_ref());
    }
}

fn register(name: &str, hotkey: Hotkey, saved: Option<&Data>) {
    let (c_name, c_description) = (c_string(name), c_string(&hotkey.description));
    // SAFETY: valid C strings, which libobs copies, and a callback matching obs_hotkey_func.
    let id = unsafe {
        (sys::obs().obs_hotkey_register_frontend)(
            c_name.as_ptr(),
            c_description.as_ptr(),
            Some(pressed),
            std::ptr::null_mut(),
        )
    };
    if id == sys::OBS_INVALID_HOTKEY_ID {
        return;
    }
    with_state(|state| {
        let registered = Registered {
            id,
            action: hotkey.action,
            description: hotkey.description,
        };
        state.by_name.insert(name.to_owned(), registered);
    });
    if let Some(saved) = saved {
        apply_binding(saved, name, id);
    }
}

fn describe(name: &str, description: &str) {
    let id = with_state(|state| {
        let registered = state.by_name.get_mut(name)?;
        registered.description = description.to_owned();
        Some(registered.id)
    });
    if let Some(id) = id {
        let c_description = c_string(description);
        // SAFETY: a registered id and a valid C string, which libobs copies.
        unsafe { (sys::obs().obs_hotkey_set_description)(id, c_description.as_ptr()) };
    }
}

fn unregister(name: &str) {
    if let Some(registered) = with_state(|state| state.by_name.remove(name)) {
        // SAFETY: the id was returned by obs_hotkey_register_frontend.
        unsafe { (sys::obs().obs_hotkey_unregister)(registered.id) };
    }
}

fn apply_binding(saved: &Data, name: &str, id: sys::obs_hotkey_id) {
    let key = c_string(name);
    // SAFETY: live data object; the returned array is an owned reference or null.
    let array = unsafe { (sys::obs().obs_data_get_array)(saved.as_ptr(), key.as_ptr()) };
    if !array.is_null() {
        // SAFETY: valid hotkey id and array; the array reference is released after loading.
        unsafe {
            (sys::obs().obs_hotkey_load)(id, array);
            (sys::obs().obs_data_array_release)(array);
        }
    }
}

/// Unregisters every Sceneuron hotkey.
pub fn unregister_all() {
    let ids: Vec<_> = with_state(|state| std::mem::take(&mut state.by_name).into_values().map(|r| r.id).collect());
    for id in ids {
        // SAFETY: each id came from obs_hotkey_register_frontend.
        unsafe { (sys::obs().obs_hotkey_unregister)(id) };
    }
}

/// Writes bindings into, or reads them from, the scene collection data.
pub fn save_or_load(save_data: *mut sys::obs_data_t, saving: bool) {
    let key = c_string(SAVE_KEY);
    if saving {
        let bindings = Data::new();
        let registered: Vec<_> = with_state(|state| state.by_name.iter().map(|(n, r)| (n.clone(), r.id)).collect());
        for (name, id) in registered {
            // SAFETY: valid hotkey id; the returned array is an owned reference.
            let array = unsafe { (sys::obs().obs_hotkey_save)(id) };
            let name = c_string(&name);
            // SAFETY: live objects; the data object takes its own reference to the array.
            unsafe {
                (sys::obs().obs_data_set_array)(bindings.as_ptr(), name.as_ptr(), array);
                (sys::obs().obs_data_array_release)(array);
            }
        }
        // SAFETY: live save data; it takes its own reference to `bindings`.
        unsafe { (sys::obs().obs_data_set_obj)(save_data, key.as_ptr(), bindings.as_ptr()) };
        return;
    }
    // SAFETY: live save data; the result is an owned reference or null.
    let Some(saved) = (unsafe { Data::from_owned((sys::obs().obs_data_get_obj)(save_data, key.as_ptr())) }) else {
        return;
    };
    let registered: Vec<_> = with_state(|state| {
        state.saved_bindings = Some(saved.json());
        state.by_name.iter().map(|(n, r)| (n.clone(), r.id)).collect()
    });
    for (name, id) in registered {
        apply_binding(&saved, &name, id);
    }
}

unsafe extern "C" fn pressed(_data: *mut c_void, id: sys::obs_hotkey_id, _hotkey: *mut sys::obs_hotkey_t, down: bool) {
    guard((), || {
        let action = with_state(|state| state.by_name.values().find(|r| r.id == id).map(|r| r.action.clone()));
        let (Some(action), Some(plugin)) = (action, plugin::current()) else {
            return;
        };
        if action == Action::PushToTalk {
            plugin.hub.update(|engine, now| engine.set_talking(down, now));
            return;
        }
        if !down {
            return;
        }
        // A refused press, such as one for a card removed a moment ago, changes nothing.
        let _ = plugin.hub.try_update(|engine, now| action.apply(engine, now));
    })
}
