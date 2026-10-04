// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Quit requests: Ctrl-C, a termination signal, or a `q` line on standard input, which also
//! carries other commands.

use std::ffi::c_int;
use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// Set from a signal handler, so it must stay a plain atomic.
static REQUESTED: AtomicBool = AtomicBool::new(false);

const POLL: Duration = Duration::from_millis(50);

/// Starts listening for quit signals.
pub(crate) fn install() {
    install_signal_handlers();
}

/// Starts reading standard input: a `q` line quits, and every other line goes to `on_line`.
pub(crate) fn read_lines(on_line: impl Fn(&str) + Send + 'static) {
    // End of input does not quit, so the server keeps running with stdin closed.
    let _ = thread::Builder::new()
        .name("sceneuron-stdin".to_owned())
        .spawn(move || {
            for line in std::io::stdin().lock().lines() {
                match line {
                    Ok(line) if line.trim().eq_ignore_ascii_case("q") => {
                        REQUESTED.store(true, Ordering::SeqCst);
                        return;
                    }
                    Ok(line) => on_line(&line),
                    Err(_) => return,
                }
            }
        });
}

/// Blocks until a quit was requested.
pub(crate) fn wait() {
    while !REQUESTED.load(Ordering::SeqCst) {
        thread::sleep(POLL);
    }
}

const SIGINT: c_int = 2;
const SIGTERM: c_int = 15;

unsafe extern "C" {
    fn signal(signum: c_int, handler: extern "C" fn(c_int)) -> usize;
}

extern "C" fn on_signal(_signum: c_int) {
    REQUESTED.store(true, Ordering::SeqCst);
}

fn install_signal_handlers() {
    for signum in [SIGINT, SIGTERM] {
        // SAFETY: SIGINT and SIGTERM have these numbers on macOS, and the handler only stores to
        // an atomic, which is async-signal-safe.
        unsafe {
            signal(signum, on_signal);
        }
    }
}
