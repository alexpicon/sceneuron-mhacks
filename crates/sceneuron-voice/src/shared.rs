// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What the voice input's handles and its thread share: the capture flags the audio thread
//! reads, the ring it writes, and the status.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::ring::AudioRing;
use crate::status::VoiceStatus;

/// State the handles share with the thread. The audio thread only reads the atomics and writes
/// the ring.
///
/// Audio is wanted while `streaming`, or while `held` and `may_listen`. Push to talk sets
/// `held` itself, so capture starts and stops with the key, never later because the voice
/// thread has not caught up; the thread sets the other two from the session.
pub(crate) struct Shared {
    /// Always-on and allowed to send.
    pub streaming: AtomicBool,
    /// Push to talk is held.
    pub held: AtomicBool,
    /// Audio may leave the computer under the current settings: on, consented, and keyed.
    pub may_listen: AtomicBool,
    /// Counts settings changes, so the thread never turns capture back on for settings that
    /// newer ones already replaced.
    pub generation: AtomicU64,
    pub ring: AudioRing,
    pub status: Mutex<VoiceStatus>,
}

impl Shared {
    pub(crate) fn capturing(&self) -> bool {
        self.streaming.load(Ordering::Acquire)
            || (self.held.load(Ordering::Acquire) && self.may_listen.load(Ordering::Acquire))
    }

    /// Stores a capture flag for settings of generation `applied`. Turning capture on is undone
    /// when newer settings arrived meanwhile, since the handle that sent them may have just
    /// turned it off; turning it off always holds.
    pub(crate) fn allow(&self, flag: &AtomicBool, on: bool, applied: u64) {
        flag.store(on, Ordering::Release);
        if on && self.generation.load(Ordering::Acquire) != applied {
            flag.store(false, Ordering::Release);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shared() -> Shared {
        Shared {
            streaming: AtomicBool::new(false),
            held: AtomicBool::new(false),
            may_listen: AtomicBool::new(false),
            generation: AtomicU64::new(0),
            ring: AudioRing::new(64),
            status: Mutex::new(VoiceStatus::Ready),
        }
    }

    #[test]
    fn capture_stays_off_once_newer_settings_turned_it_off() {
        let shared = shared();
        shared.allow(&shared.streaming, true, 0);
        assert!(shared.capturing(), "the settings in use allow it");
        // A handle sends settings that forbid sending and turns capture off at once...
        shared.generation.fetch_add(1, Ordering::AcqRel);
        shared.streaming.store(false, Ordering::Release);
        // ...and the thread, still on the old settings, must not turn it back on.
        shared.allow(&shared.streaming, true, 0);
        assert!(!shared.capturing());
        shared.allow(&shared.may_listen, false, 0);
        shared.allow(&shared.streaming, true, 1);
        assert!(shared.capturing(), "once the thread runs the new settings, they decide");
    }
}
