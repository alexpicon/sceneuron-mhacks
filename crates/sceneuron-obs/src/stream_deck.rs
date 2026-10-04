// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Pairing the Stream Deck without the editor: whether Sceneuron's plugin is installed in the
//! Stream Deck app on this Mac, and the pairing OBS sends by itself once it started, at most once
//! per OBS run. The server decides whether a link goes out (see `Server::pair_stream_deck`).

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// How long OBS waits after it starts before it pairs, so that a Stream Deck paired in an earlier
/// run reconnects first and counts as paired: while Sceneuron is away, the plugin tries again at
/// most 30 seconds apart.
pub const RECONNECT_GRACE: Duration = Duration::from_secs(40);

/// Where the Stream Deck app keeps Sceneuron's plugin, under the home folder.
const PLUGIN_FOLDER: &str = "Library/Application Support/com.elgato.StreamDeck/Plugins/ai.chaski.sceneuron.sdPlugin";

/// The folder of Sceneuron's plugin among the Stream Deck app's plugins under `home`.
pub fn plugin_folder(home: &Path) -> PathBuf {
    home.join(PLUGIN_FOLDER)
}

/// The home folder as Foundation finds it, from `CFFIXED_USER_HOME` (`fixed`) before `HOME`, so
/// an OBS started with a home of its own, as the end-to-end proof starts one, sees only that
/// home's Stream Deck plugins and never pairs the user's own Stream Deck. Empty values are unset.
pub fn home_folder(fixed: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    [fixed, home]
        .into_iter()
        .flatten()
        .find(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Whether Sceneuron's Stream Deck plugin is installed under `home`: its folder is there, or a
/// link to one, as `streamdeck link` makes for development.
pub fn plugin_installed_under(home: &Path) -> bool {
    plugin_folder(home).is_dir()
}

/// Whether Sceneuron's Stream Deck plugin is installed for the user OBS runs as.
pub fn plugin_installed() -> bool {
    home_folder(std::env::var_os("CFFIXED_USER_HOME"), std::env::var_os("HOME"))
        .is_some_and(|home| plugin_installed_under(&home))
}

/// Lets one pairing through per OBS run, however often it is asked.
#[derive(Debug, Default)]
pub struct OncePerRun(AtomicBool);

impl OncePerRun {
    pub const fn new() -> Self {
        Self(AtomicBool::new(false))
    }

    /// True the first time only.
    pub fn first(&self) -> bool {
        !self.0.swap(true, Ordering::SeqCst)
    }
}

/// The pairing OBS sends once it started: a thread waits, then pairs, unless OBS quits first.
#[derive(Debug)]
pub struct PairAtStart {
    /// Dropped to wake the waiting thread, which then pairs nothing.
    stop: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl PairAtStart {
    /// Calls `pair` once `wait` has passed, unless [`PairAtStart::stop`] comes first.
    pub fn start(wait: Duration, pair: impl FnOnce() + Send + 'static) -> io::Result<PairAtStart> {
        let (stop, stopped) = mpsc::channel::<()>();
        let thread = thread::Builder::new()
            .name("sceneuron-stream-deck".to_owned())
            .spawn(move || {
                if let Err(RecvTimeoutError::Timeout) = stopped.recv_timeout(wait) {
                    pair();
                }
            })?;
        Ok(PairAtStart {
            stop: Some(stop),
            thread: Some(thread),
        })
    }

    /// Cancels the pairing if it has not begun and waits for one that has, so no pairing runs
    /// once this returns. Safe to call more than once.
    pub fn stop(&mut self) {
        drop(self.stop.take());
        if let Some(thread) = self.thread.take()
            && thread.thread().id() != thread::current().id()
        {
            let _ = thread.join();
        }
    }
}

impl Drop for PairAtStart {
    fn drop(&mut self) {
        self.stop();
    }
}
