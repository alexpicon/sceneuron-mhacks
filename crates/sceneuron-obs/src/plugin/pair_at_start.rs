// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The Stream Deck pairing OBS sends by itself: once OBS started, and a Stream Deck paired in an
//! earlier run had time to reconnect, Sceneuron's Stream Deck plugin gets a pairing link when it
//! is installed and no control surface is paired. It goes at most once per OBS run, never to a
//! paired Stream Deck, and the log says what happened without the code.

use std::sync::{Arc, PoisonError};

use sceneuron_server::SetupOutcome;

use super::Plugin;
use crate::ffi;
use crate::stream_deck::{OncePerRun, PairAtStart, RECONNECT_GRACE};

/// However often the plugin starts within one OBS process, it pairs once.
static PAIRED_THIS_RUN: OncePerRun = OncePerRun::new();

impl Plugin {
    /// Pairs the Stream Deck once [`RECONNECT_GRACE`] has passed, unless OBS quits first.
    pub(super) fn pair_stream_deck_later(self: &Arc<Self>) {
        let plugin = Arc::downgrade(self);
        let started = PairAtStart::start(RECONNECT_GRACE, move || {
            if let Some(plugin) = plugin.upgrade()
                && PAIRED_THIS_RUN.first()
            {
                plugin.pair_stream_deck();
            }
        });
        match started {
            Ok(pairing) => *self.pair_at_start.lock().unwrap_or_else(PoisonError::into_inner) = Some(pairing),
            Err(error) => ffi::warn(&format!("could not start pairing the Stream Deck: {error}")),
        }
    }

    /// Ends the wait, or waits for a pairing under way. Call it before stopping the server,
    /// which the pairing locks.
    pub(super) fn stop_pairing(&self) {
        let pairing = self.pair_at_start.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(mut pairing) = pairing {
            pairing.stop();
        }
    }

    fn pair_stream_deck(&self) {
        let state = self.server.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(server) = state.server.as_ref() else {
            ffi::warn("no Stream Deck pairing link was sent: the editor server is not running");
            return;
        };
        let port = server.port();
        let step = server.pair_stream_deck();
        drop(state);
        match step.outcome {
            SetupOutcome::Added => ffi::info(&format!(
                "sent Sceneuron's Stream Deck plugin a pairing link for port {port}, since no paired Stream Deck \
                 connected within {} s of OBS starting",
                RECONNECT_GRACE.as_secs()
            )),
            SetupOutcome::Present => ffi::info("a Stream Deck is paired, so no pairing link was sent"),
            SetupOutcome::Skipped => ffi::info(&format!("no Stream Deck pairing link was sent: {}", step.reason)),
        }
    }
}
