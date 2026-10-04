// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The process-wide tables: `load` outside OBS, `install`, and the `obs`, `try_obs`, and
//! `frontend` accessors. A single test, because the tables are global to the test process.

use std::any::Any;
use std::ffi::{CStr, c_char, c_void};
use std::panic;
use std::ptr;

use sceneuron_obs_sys::{Libobs, frontend, install, load, obs, obs_source_t, resolve_tables, try_obs};

const SCENE: *mut obs_source_t = ptr::without_provenance_mut(0x5ce7);

extern "C" fn first_version() -> *const c_char {
    c"first".as_ptr()
}

extern "C" fn second_version() -> *const c_char {
    c"second".as_ptr()
}

extern "C" fn fake_current_scene() -> *mut obs_source_t {
    SCENE
}

/// Stands in for every function the test never calls.
extern "C" fn never_called() {}

fn fake_with_version(version: extern "C" fn() -> *const c_char) -> impl Fn(&CStr) -> *mut c_void {
    move |name| {
        let function: *const () = match name.to_bytes() {
            b"obs_get_version_string" => version as *const (),
            b"obs_frontend_get_current_scene" => fake_current_scene as *const (),
            _ => never_called as *const (),
        };
        function.cast_mut().cast()
    }
}

fn version_of(libobs: &Libobs) -> &'static CStr {
    // SAFETY: every table in this test holds one of the fake version functions, which take no
    // arguments and return static strings.
    unsafe { CStr::from_ptr((libobs.obs_get_version_string)()) }
}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    match payload.downcast::<&str>() {
        Ok(message) => (*message).to_owned(),
        Err(payload) => *payload.downcast::<String>().expect("panic payload is a string"),
    }
}

#[test]
fn tables_go_from_unloaded_to_installed_to_replaced() {
    assert!(try_obs().is_none());
    assert!(frontend().is_none());
    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let unloaded = panic::catch_unwind(obs);
    panic::set_hook(previous_hook);
    let message = panic_message(unloaded.expect_err("obs() panics before load"));
    assert!(message.contains("load()"), "{message}");

    let error = load().expect_err("the test process does not export libobs");
    assert!(
        Libobs::SYMBOLS
            .iter()
            .any(|symbol| symbol.to_str() == Ok(error.symbol()))
    );
    assert!(try_obs().is_none(), "a failed load installs nothing");

    let (libobs, _) = resolve_tables(fake_with_version(first_version), |_| ptr::null_mut()).unwrap();
    install(libobs, None);
    let first = obs();
    assert_eq!(version_of(first), c"first");
    assert!(frontend().is_none());
    load().expect("load keeps tables that are already installed");
    assert_eq!(version_of(obs()), c"first");

    let fake = fake_with_version(second_version);
    let (libobs, frontend_table) = resolve_tables(&fake, &fake).unwrap();
    install(libobs, frontend_table);
    assert_eq!(version_of(obs()), c"second");
    assert_eq!(version_of(try_obs().unwrap()), c"second");
    let frontend_table = frontend().expect("the replacement has a frontend");
    // SAFETY: the field holds `fake_current_scene`, which takes no arguments.
    assert_eq!(unsafe { (frontend_table.obs_frontend_get_current_scene)() }, SCENE);
    assert_eq!(version_of(first), c"first", "replaced tables stay valid");
}
