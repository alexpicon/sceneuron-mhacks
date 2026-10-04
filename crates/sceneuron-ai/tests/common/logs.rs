// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A logger that keeps every line the test binary logs, for tests of what the clients log.

use std::sync::{Mutex, Once};

use log::{LevelFilter, Log, Metadata, Record};

/// Keeps every log line of the test binary, whichever test wrote it.
struct Capture(Mutex<Vec<String>>);

impl Log for Capture {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &Record<'_>) {
        self.0.lock().unwrap().push(record.args().to_string());
    }

    fn flush(&self) {}
}

static CAPTURE: Capture = Capture(Mutex::new(Vec::new()));

/// Every line logged at debug level or above since the first call, which starts the capture.
pub fn logged() -> Vec<String> {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        log::set_logger(&CAPTURE).unwrap();
        log::set_max_level(LevelFilter::Debug);
    });
    CAPTURE.0.lock().unwrap().clone()
}
