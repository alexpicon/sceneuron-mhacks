// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Resolving the function tables: fake lookups backed by functions defined here (success, calls
//! with a string and a double through the table, whether OBS streams and records, a missing
//! required symbol, an absent frontend through the same `resolve_tables` that `load` uses), and the
//! real platform lookup.

use std::cell::RefCell;
use std::ffi::{CStr, c_char, c_longlong, c_void};
use std::ptr;
use std::sync::Mutex;

use sceneuron_obs_sys::{
    Frontend, Libobs, find_frontend_symbol, find_libobs_symbol, obs_data_t, obs_output_t, obs_source_t, resolve_tables,
};

const SCENE: *mut obs_source_t = ptr::without_provenance_mut(0x5ce7);
const STREAM: *mut obs_output_t = ptr::without_provenance_mut(0x57ea);

/// What the fake caption function received: the output, the text, and the display duration.
static CAPTIONS: Mutex<Vec<(usize, String, f64)>> = Mutex::new(Vec::new());

extern "C" fn fake_caption(output: *mut obs_output_t, text: *const c_char, display_duration: f64) {
    // SAFETY: the test passes a NUL-terminated literal.
    let text = unsafe { CStr::from_ptr(text) }.to_str().unwrap().to_owned();
    CAPTIONS.lock().unwrap().push((output.addr(), text, display_duration));
}

extern "C" fn fake_streaming_output() -> *mut obs_output_t {
    STREAM
}

extern "C" fn fake_version() -> *const c_char {
    c"32.2.2-fake".as_ptr()
}

extern "C" fn fake_data_get_int(data: *mut obs_data_t, name: *const c_char) -> c_longlong {
    assert!(data.is_null());
    // SAFETY: the tests pass a NUL-terminated literal.
    unsafe { CStr::from_ptr(name) }.to_bytes().len() as c_longlong
}

extern "C" fn fake_current_scene() -> *mut obs_source_t {
    SCENE
}

extern "C" fn fake_streaming_active() -> bool {
    true
}

extern "C" fn fake_recording_active() -> bool {
    false
}

/// Stands in for every function a test never calls.
extern "C" fn never_called() {}

fn fake(name: &CStr) -> *mut c_void {
    let function: *const () = match name.to_bytes() {
        b"obs_get_version_string" => fake_version as *const (),
        b"obs_data_get_int" => fake_data_get_int as *const (),
        b"obs_frontend_get_current_scene" => fake_current_scene as *const (),
        b"obs_output_output_caption_text2" => fake_caption as *const (),
        b"obs_frontend_get_streaming_output" => fake_streaming_output as *const (),
        b"obs_frontend_streaming_active" => fake_streaming_active as *const (),
        b"obs_frontend_recording_active" => fake_recording_active as *const (),
        _ => never_called as *const (),
    };
    function.cast_mut().cast()
}

fn fake_without(missing: &'static str) -> impl Fn(&CStr) -> *mut c_void {
    move |name| {
        if name.to_bytes() == missing.as_bytes() {
            ptr::null_mut()
        } else {
            fake(name)
        }
    }
}

fn names(symbols: &[&CStr]) -> Vec<String> {
    symbols
        .iter()
        .map(|symbol| symbol.to_str().unwrap().to_owned())
        .collect()
}

#[test]
fn libobs_table_covers_exactly_the_required_functions() {
    let expected = "blog bfree obs_get_version_string obs_register_source_s obs_module_get_config_path \
        obs_source_create obs_source_create_private obs_source_release obs_source_get_ref obs_source_get_weak_source \
        obs_weak_source_get_source obs_weak_source_release obs_source_add_active_child obs_source_remove_active_child \
        obs_source_video_render \
        obs_source_update obs_source_get_name obs_source_get_uuid obs_source_get_id obs_source_get_unversioned_id \
        obs_source_get_output_flags obs_source_active obs_source_removed obs_source_get_type obs_source_get_settings obs_source_filter_add \
        obs_source_filter_remove \
        obs_source_enum_filters obs_filter_get_parent obs_get_source_by_uuid obs_enum_sources \
        obs_get_output_source obs_obj_get_data obs_get_audio audio_output_get_channels audio_output_get_sample_rate obs_scene_from_source \
        obs_scene_add obs_scene_find_source_recursive obs_data_create obs_data_create_from_json obs_data_release \
        obs_data_get_json \
        obs_data_get_string obs_data_set_string obs_data_get_int obs_data_set_int obs_data_get_bool \
        obs_data_set_bool obs_data_get_double obs_data_set_double obs_data_set_default_int obs_data_set_default_bool \
        obs_data_set_default_string obs_data_set_default_double obs_data_get_obj obs_data_set_obj \
        obs_data_get_array obs_data_set_array obs_data_array_release obs_properties_create obs_properties_add_bool \
        obs_properties_add_int obs_properties_add_float_slider obs_properties_add_text obs_properties_add_button2 \
        obs_property_set_long_description obs_property_text_set_info_type obs_property_set_enabled \
        obs_hotkey_register_frontend obs_hotkey_unregister obs_hotkey_set_description obs_hotkey_save obs_hotkey_load \
        obs_get_proc_handler \
        proc_handler_call calldata_set_data calldata_get_data obs_queue_task obs_in_task_thread \
        obs_get_audio_info obs_source_add_audio_capture_callback obs_source_remove_audio_capture_callback \
        obs_source_output_audio obs_output_active obs_output_release obs_output_output_caption_text2";
    assert_eq!(names(Libobs::SYMBOLS), expected.split_whitespace().collect::<Vec<_>>());
}

#[test]
fn frontend_table_covers_exactly_the_required_functions() {
    let expected = "obs_frontend_add_tools_menu_item obs_frontend_open_source_properties \
        obs_frontend_add_event_callback obs_frontend_remove_event_callback obs_frontend_add_save_callback \
        obs_frontend_remove_save_callback obs_frontend_get_current_scene obs_frontend_get_streaming_output \
        obs_frontend_streaming_active obs_frontend_recording_active";
    assert_eq!(
        names(Frontend::SYMBOLS),
        expected.split_whitespace().collect::<Vec<_>>()
    );
}

#[test]
fn resolve_looks_up_every_symbol_once_and_calls_through_to_it() {
    let requested = RefCell::new(Vec::new());
    let libobs = Libobs::resolve(|name| {
        requested.borrow_mut().push(name.to_str().unwrap().to_owned());
        fake(name)
    })
    .expect("every symbol is present");
    assert_eq!(requested.into_inner(), names(Libobs::SYMBOLS));

    // SAFETY: the field holds `fake_version`, which takes no arguments and returns a static string.
    let version = unsafe { CStr::from_ptr((libobs.obs_get_version_string)()) };
    assert_eq!(version, c"32.2.2-fake");
    // SAFETY: the field holds `fake_data_get_int`, which accepts a null data pointer and a C string.
    let value = unsafe { (libobs.obs_data_get_int)(ptr::null_mut(), c"width".as_ptr()) };
    assert_eq!(value, 5);
}

#[test]
fn a_caption_goes_through_the_tables_to_the_streaming_output_with_its_duration() {
    let (libobs, frontend) = resolve_tables(fake, fake).expect("libobs is present");
    let frontend = frontend.expect("the frontend is present");
    // SAFETY: the field holds `fake_streaming_output`, which takes no arguments.
    let output = unsafe { (frontend.obs_frontend_get_streaming_output)() };
    assert_eq!(output, STREAM);
    // SAFETY: the field holds `fake_caption`, which reads a NUL-terminated string.
    unsafe { (libobs.obs_output_output_caption_text2)(output, c"Hola a todos".as_ptr(), 2.5) };
    assert_eq!(*CAPTIONS.lock().unwrap(), [(0x57ea, "Hola a todos".to_owned(), 2.5)]);
}

#[test]
fn each_missing_libobs_symbol_is_named_in_the_error() {
    for symbol in names(Libobs::SYMBOLS) {
        let symbol: &'static str = symbol.leak();
        let error = Libobs::resolve(fake_without(symbol)).expect_err("a required symbol is missing");
        assert_eq!(error.symbol(), symbol);
        assert!(error.to_string().contains(&format!("`{symbol}`")), "{error}");
    }
}

#[test]
fn each_missing_frontend_symbol_is_named_in_the_error() {
    for symbol in names(Frontend::SYMBOLS) {
        let symbol: &'static str = symbol.leak();
        let error = Frontend::resolve(fake_without(symbol)).expect_err("a frontend symbol is missing");
        assert_eq!(error.symbol(), symbol);
    }
}

#[test]
fn tables_include_the_frontend_when_it_is_exported() {
    let (_, frontend) = resolve_tables(fake, fake).expect("libobs is present");
    let frontend = frontend.expect("the frontend is present");
    // SAFETY: the field holds `fake_current_scene`, which takes no arguments.
    assert_eq!(unsafe { (frontend.obs_frontend_get_current_scene)() }, SCENE);
}

#[test]
fn whether_obs_streams_and_records_comes_through_the_frontend_table() {
    let (_, frontend) = resolve_tables(fake, fake).expect("libobs is present");
    let frontend = frontend.expect("the frontend is present");
    // SAFETY: the fields hold `fake_streaming_active` and `fake_recording_active`, which take no
    // arguments.
    let (streaming, recording) = unsafe {
        (
            (frontend.obs_frontend_streaming_active)(),
            (frontend.obs_frontend_recording_active)(),
        )
    };
    assert_eq!((streaming, recording), (true, false));
}

#[test]
fn tables_drop_the_frontend_when_any_frontend_symbol_is_missing() {
    let (_, frontend) = resolve_tables(fake, |_| ptr::null_mut()).expect("libobs is present");
    assert!(frontend.is_none());
    let (_, frontend) =
        resolve_tables(fake, fake_without("obs_frontend_remove_save_callback")).expect("libobs is present");
    assert!(frontend.is_none());
}

#[test]
fn tables_fail_when_libobs_is_missing_even_with_a_frontend() {
    let error = resolve_tables(|_| ptr::null_mut(), fake).expect_err("libobs is required");
    assert_eq!(error.symbol(), "blog");
    let error = resolve_tables(fake_without("calldata_get_data"), fake).expect_err("libobs is required");
    assert_eq!(error.symbol(), "calldata_get_data");
}

#[test]
fn platform_lookup_returns_null_for_unknown_symbols() {
    assert!(find_libobs_symbol(c"sceneuron_no_such_symbol").is_null());
    assert!(find_frontend_symbol(c"sceneuron_no_such_symbol").is_null());
}

#[test]
fn platform_lookup_finds_process_exports_through_rtld_default() {
    for lookup in [find_libobs_symbol, find_frontend_symbol] {
        let address = lookup(c"strlen");
        assert!(!address.is_null());
        // SAFETY: `strlen` from the C library has exactly this signature.
        let strlen =
            unsafe { std::mem::transmute::<*mut c_void, unsafe extern "C" fn(*const c_char) -> usize>(address) };
        // SAFETY: the argument is a NUL-terminated literal.
        assert_eq!(unsafe { strlen(c"sceneuron".as_ptr()) }, 9);
    }
}
