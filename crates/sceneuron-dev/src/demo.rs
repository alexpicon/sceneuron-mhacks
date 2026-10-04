// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A simulated camera that alternates between someone in view and an empty room, with a live
//! description that matches the drawn picture.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use sceneuron_core::{Outcome, Presence, Sighting, VisionStatus};
use sceneuron_server::Hub;

const SAMPLE_INTERVAL: Duration = Duration::from_millis(200);
const PHASE_MS: u64 = 8_000;
const PRESENT_SCORE: f64 = 0.95;
const AWAY_SCORE: f64 = 0.05;
const SIMULATED_LATENCY_MS: f64 = 12.0;

pub(crate) struct Demo {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

/// Starts feeding observations to the hub from a background thread.
pub(crate) fn start(hub: Arc<Hub>) -> std::io::Result<Demo> {
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = Arc::clone(&stop);
    let thread = thread::Builder::new()
        .name("sceneuron-demo".to_owned())
        .spawn(move || {
            let status = VisionStatus::Running {
                latency_ms: SIMULATED_LATENCY_MS,
                mouths: false,
                hands: false,
            };
            hub.update(|engine, now| engine.set_vision_status(status, now));
            let started = hub.now_ms();
            while !stopping.load(Ordering::Acquire) {
                hub.update(|engine, now| {
                    let observed = engine.observe(score(present_at(now.saturating_sub(started))), now);
                    // The drawn picture shows the person while presence says so, and so does the line.
                    let shown = engine.presence() == Presence::Present;
                    let seen = engine.observe_sighting(&sighting(shown), now);
                    Outcome {
                        changed: observed.changed || seen.changed,
                        project_dirty: false,
                        telemetry: observed.telemetry || seen.telemetry,
                    }
                });
                thread::park_timeout(SAMPLE_INTERVAL);
            }
        })?;
    Ok(Demo { stop, thread })
}

impl Demo {
    pub(crate) fn stop(self) {
        self.stop.store(true, Ordering::Release);
        self.thread.thread().unpark();
        let _ = self.thread.join();
    }
}

/// Someone is in view for the first phase, away for the next, and so on.
fn present_at(elapsed_ms: u64) -> bool {
    (elapsed_ms / PHASE_MS).is_multiple_of(2)
}

fn score(present: bool) -> f64 {
    if present { PRESENT_SCORE } else { AWAY_SCORE }
}

/// What on-device analysis would find in the drawn picture: one person while it shows one.
fn sighting(present: bool) -> Sighting {
    Sighting {
        people: usize::from(present),
        ..Sighting::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phases_alternate_every_eight_seconds() {
        let scores = [0, 7_999, 8_000, 15_999, 16_000].map(|elapsed| score(present_at(elapsed)));
        assert_eq!(
            scores,
            [PRESENT_SCORE, PRESENT_SCORE, AWAY_SCORE, AWAY_SCORE, PRESENT_SCORE]
        );
    }

    #[test]
    fn the_live_description_follows_the_phase() {
        assert_eq!(sighting(true).describe(), "1 person");
        assert_eq!(sighting(false).describe(), "Nobody in view");
    }
}
