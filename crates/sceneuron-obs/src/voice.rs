// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The voice input inside OBS: the assistant owns it, with its settings, key, keyterms, and
//! push to talk (see `sceneuron_ai::VoiceLink`); a "Sceneuron Voice" filter feeds it
//! microphone audio.
//!
//! Only one filter feeds it at a time, as with camera analysis: its converter keeps the state of
//! one stream, and two microphones would interleave. The first filter that hands over audio owns
//! the input until it is removed.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use sceneuron_ai::Assistant;
use sceneuron_voice::VoiceInput;

#[derive(Default)]
pub struct VoiceBridge {
    /// The filter that feeds the voice input, by its id; 0 when none does yet.
    owner: Arc<AtomicUsize>,
}

impl VoiceBridge {
    /// A handle for a filter's audio callback, and the owner slot it claims.
    pub fn input(&self, assistant: &Assistant) -> (VoiceInput, Arc<AtomicUsize>) {
        (assistant.voice().input(), Arc::clone(&self.owner))
    }

    /// Which filter feeds the voice input, by its id; 0 until one hands over audio.
    pub fn owner(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.owner)
    }

    pub fn filter_destroyed(&self, id: usize) {
        let _ = self.owner.compare_exchange(id, 0, Ordering::SeqCst, Ordering::SeqCst);
    }
}

/// Whether the filter `id` may feed the voice input: it already owns it, or nobody does.
pub fn claim(owner: &AtomicUsize, id: usize) -> bool {
    match owner.compare_exchange(0, id, Ordering::SeqCst, Ordering::SeqCst) {
        Ok(_) => true,
        Err(current) => current == id,
    }
}
