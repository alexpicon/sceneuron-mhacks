// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Stream captions inside OBS: while stream captions are on, a hub listener picks the new final
//! lines of the chosen language, and a thread paces them into the streaming output's CEA-608
//! captions (see [`crate::captions`]). libobs takes captions only while the output is active, and
//! the sender drops them while it is not, so nothing piles up before going live. With nothing to
//! send, the thread sleeps until the listener hands it a line, and never looks up the output, which
//! the frontend may be replacing on its own thread.

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use sceneuron_core::DEFAULT_CLEAR_AFTER_MS;
use sceneuron_obs_sys as sys;
use sceneuron_server::{Hub, HubEvent};

use crate::captions::{CaptionFeed, CaptionOutput, CaptionSender};
use crate::ffi;

struct State {
    sender: CaptionSender,
    stop: bool,
}

struct Shared {
    state: Mutex<State>,
    wake: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

struct Running {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

static CAPTIONS: Mutex<Option<Running>> = Mutex::new(None);

/// Starts stream captions for `hub`. They need the OBS frontend, which headless hosts lack.
pub fn start(hub: &Arc<Hub>) {
    if sys::frontend().is_none() {
        return;
    }
    let shared = Arc::new(Shared {
        state: Mutex::new(State {
            sender: CaptionSender::new(DEFAULT_CLEAR_AFTER_MS),
            stop: false,
        }),
        wake: Condvar::new(),
    });
    follow(hub, &shared);
    let paced = Arc::clone(&shared);
    let thread = thread::Builder::new()
        .name("sceneuron-captions".to_owned())
        .spawn(move || ffi::guard((), || run(&paced)))
        .map_err(|error| ffi::warn(&format!("stream captions cannot start: {error}")))
        .ok();
    *CAPTIONS.lock().unwrap_or_else(PoisonError::into_inner) = Some(Running { shared, thread });
}

/// Stops the thread and waits for it. Safe to call more than once.
pub fn stop() {
    let running = CAPTIONS.lock().unwrap_or_else(PoisonError::into_inner).take();
    if let Some(mut running) = running {
        running.shared.lock().stop = true;
        running.shared.wake.notify_all();
        if let Some(thread) = running.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Hands the sender each new final line of the chosen language while stream captions are on.
fn follow(hub: &Arc<Hub>, shared: &Arc<Shared>) {
    let (hub_ref, shared_ref) = (Arc::downgrade(hub), Arc::downgrade(shared));
    let feed = Mutex::new(CaptionFeed::default());
    hub.add_listener(Box::new(move |event| {
        let (Some(hub), Some(shared)) = (hub_ref.upgrade(), shared_ref.upgrade()) else {
            return;
        };
        if !matches!(event, HubEvent::StateChanged { .. }) {
            return;
        }
        let (on, language, clear_after_ms, lines) = hub.read(|engine| {
            let status = engine.subtitle_status();
            let on = status.stream_captions;
            let lines = if on {
                engine.caption_lines(&status.stream_language)
            } else {
                Vec::new()
            };
            (on, status.stream_language.clone(), status.clear_after_ms, lines)
        });
        let mut feed = feed.lock().unwrap_or_else(PoisonError::into_inner);
        let mut state = shared.lock();
        if !on {
            feed.reset();
            state.sender.reset();
            return;
        }
        state.sender.set_clear_after(clear_after_ms);
        let fresh = feed.new_lines(&language, &lines);
        fresh.iter().for_each(|line| state.sender.push(line));
        if !fresh.is_empty() {
            shared.wake.notify_one();
        }
    }));
}

fn run(shared: &Shared) {
    let clock = Instant::now();
    let mut output = StreamOutput;
    let mut state = shared.lock();
    while !state.stop {
        let now_ms = u64::try_from(clock.elapsed().as_millis()).unwrap_or(u64::MAX);
        state.sender.poll(now_ms, &mut output);
        state = match state.sender.wait_ms(now_ms) {
            Some(wait_ms) => {
                let waited = shared.wake.wait_timeout(state, Duration::from_millis(wait_ms));
                waited.unwrap_or_else(PoisonError::into_inner).0
            }
            None => shared.wake.wait(state).unwrap_or_else(PoisonError::into_inner),
        };
    }
}

/// OBS's streaming output, looked up for each use, since the frontend replaces it when the
/// stream settings change.
struct StreamOutput;

impl CaptionOutput for StreamOutput {
    fn streaming(&self) -> bool {
        // SAFETY: the output is a valid reference for the duration of the call.
        with_output(|obs, output| unsafe { (obs.obs_output_active)(output) }).unwrap_or(false)
    }

    fn send(&mut self, text: &str, seconds: f64) -> bool {
        let text = ffi::c_string(text);
        // SAFETY: the output is a valid reference and `text` a NUL-terminated string, both alive
        // for the call; libobs copies the text.
        let sent = with_output(|obs, output| unsafe {
            (obs.obs_output_output_caption_text2)(output, text.as_ptr(), seconds);
        });
        sent.is_some()
    }
}

/// Calls `use_output` with a reference to the streaming output, released afterwards; `None` when
/// there is none.
fn with_output<R>(use_output: impl FnOnce(&sys::Libobs, *mut sys::obs_output_t) -> R) -> Option<R> {
    let frontend = sys::frontend()?;
    let obs = sys::obs();
    // SAFETY: the frontend returns a new reference to the streaming output, or null.
    let output = unsafe { (frontend.obs_frontend_get_streaming_output)() };
    if output.is_null() {
        return None;
    }
    let result = use_output(obs, output);
    // SAFETY: releases the reference taken above, once.
    unsafe { (obs.obs_output_release)(output) };
    Some(result)
}
