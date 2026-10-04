// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Connects camera filters to the background perceiver and reports results to the hub: presence,
//! card signals, the facepalm reaction, and the live description in one update per analyzed frame.
//! What it found also goes to the editor's camera preview while someone watches it.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError, RwLock};

use sceneuron_core::{Outcome, VisionStatus};
use sceneuron_server::Hub;
use sceneuron_vision::{DetectError, InputSize, Perceiver, Perception, Sample, UltraFace, VisionWorker, WorkerEvent};

use crate::ffi;
use crate::preview::PreviewTap;
use crate::sighting::sighting;
use crate::signals::signal_summary;

/// Receives every perception, on the analysis thread.
type PerceptionHook = Arc<dyn Fn(&Perception) + Send + Sync>;

/// One perceiver shared by every camera filter; started when the first filter appears and
/// restarted when a filter appears after it stopped for good.
///
/// Only one filter feeds it at a time: presence describes one person in front of one camera, and
/// frames from two cameras would contradict each other. The first filter that samples a frame
/// owns analysis until it is removed.
pub struct VisionBridge {
    hub: Arc<Hub>,
    worker: Mutex<Option<VisionWorker>>,
    filters: Arc<AtomicUsize>,
    /// The filter that owns analysis, by its id; 0 when none does yet.
    active: AtomicUsize,
    perception_hook: Arc<RwLock<Option<PerceptionHook>>>,
    preview: Arc<PreviewTap>,
}

impl VisionBridge {
    pub fn new(hub: Arc<Hub>) -> VisionBridge {
        let filters = Arc::new(AtomicUsize::new(0));
        VisionBridge {
            hub,
            worker: Mutex::new(None),
            preview: Arc::new(PreviewTap::new(Arc::clone(&filters))),
            filters,
            active: AtomicUsize::new(0),
            perception_hook: Arc::new(RwLock::new(None)),
        }
    }

    /// The editor's camera preview, which the filter that owns analysis feeds.
    pub fn preview(&self) -> &Arc<PreviewTap> {
        &self.preview
    }

    pub fn filter_created(&self) {
        self.filters.fetch_add(1, Ordering::SeqCst);
        let mut worker = self.worker.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if worker.as_ref().is_none_or(VisionWorker::is_stopped) {
            if let Some(stopped) = worker.take() {
                stopped.shutdown();
            }
            let now = self.hub.now_ms();
            self.hub
                .update(|engine, _| engine.set_vision_status(VisionStatus::Starting, now));
            let (hub, hook, filters, preview) = (
                Arc::clone(&self.hub),
                Arc::clone(&self.perception_hook),
                Arc::clone(&self.filters),
                Arc::clone(&self.preview),
            );
            // Set on the analysis thread before it reports being ready.
            let (mouths, hands) = (Arc::new(AtomicBool::new(false)), Arc::new(AtomicBool::new(false)));
            let (measured, located) = (Arc::clone(&mouths), Arc::clone(&hands));
            let make = move || {
                let (perceiver, abilities) = make_perceiver()?;
                measured.store(abilities.mouths, Ordering::Release);
                located.store(abilities.hands, Ordering::Release);
                Ok(perceiver)
            };
            *worker = Some(VisionWorker::spawn(make, move |event| {
                // A result that arrives after the last filter left would show a removed camera
                // as running.
                if filters.load(Ordering::SeqCst) > 0 {
                    let abilities = Abilities {
                        mouths: mouths.load(Ordering::Acquire),
                        hands: hands.load(Ordering::Acquire),
                    };
                    handle(&hub, &hook, &preview, abilities, event);
                }
            }));
        }
    }

    /// Whether the filter `id` may feed analysis: it already owns it, or nobody does.
    pub fn claim(&self, id: usize) -> bool {
        match self.active.compare_exchange(0, id, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => true,
            Err(owner) => owner == id,
        }
    }

    pub fn filter_destroyed(&self, id: usize) {
        let owned = self
            .active
            .compare_exchange(id, 0, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok();
        let previous = self.filters.fetch_sub(1, Ordering::SeqCst);
        self.preview.filter_removed(owned);
        if previous == 1 {
            let now = self.hub.now_ms();
            self.hub
                .update(|engine, _| engine.set_vision_status(VisionStatus::Idle, now));
        }
    }

    /// The perceiver's input size when a sample would be accepted now; `None` while the
    /// perceiver is starting, busy, or stopped. Never blocks the video thread.
    pub fn accepts(&self) -> Option<InputSize> {
        let worker = self.worker.try_lock().ok()?;
        let worker = worker.as_ref()?;
        if worker.is_idle() { worker.input_size() } else { None }
    }

    pub fn submit(&self, sample: Sample) -> bool {
        match self.worker.try_lock() {
            Ok(worker) => worker.as_ref().is_some_and(|worker| worker.try_submit(sample)),
            Err(_) => false,
        }
    }

    /// Calls `hook` with every perception from now on, replacing any earlier hook. It runs
    /// on the analysis thread after presence is updated, so it must return quickly and must
    /// not call back into the bridge.
    pub fn on_perception(&self, hook: impl Fn(&Perception) + Send + Sync + 'static) {
        *self.perception_hook.write().unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(hook));
    }

    pub fn shutdown(&self) {
        let worker = self
            .worker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(worker) = worker {
            worker.shutdown();
        }
    }
}

/// What a perceiver can see beyond faces, for the editor to say which fun reactions can run.
#[derive(Clone, Copy, Debug, Default)]
struct Abilities {
    /// It measures how far apart the lips are.
    mouths: bool,
    /// It locates hands and their joints, and so sees a facepalm.
    hands: bool,
}

/// Apple Vision, falling back to UltraFace when it is unavailable; and what it sees beyond faces,
/// which only Apple Vision does.
fn make_perceiver() -> Result<(Box<dyn Perceiver>, Abilities), DetectError> {
    match sceneuron_vision_apple::AppleVision::new() {
        Ok(vision) => {
            ffi::info(&format!("Apple Vision analyses: {}", vision.active().join(", ")));
            for reason in vision.skipped() {
                ffi::warn(&format!("Apple Vision skipped an analysis: {reason}"));
            }
            let abilities = Abilities {
                mouths: vision.measures_mouths(),
                hands: vision.locates_hands(),
            };
            return Ok((Box::new(vision), abilities));
        }
        Err(error) => ffi::warn(&format!("Apple Vision is unavailable, using UltraFace: {error}")),
    }
    Ok((Box::new(UltraFace::new()?), Abilities::default()))
}

/// Reports `event` from a perceiver with `abilities`.
fn handle(
    hub: &Hub,
    hook: &RwLock<Option<PerceptionHook>>,
    preview: &PreviewTap,
    abilities: Abilities,
    event: WorkerEvent,
) {
    let Abilities { mouths, hands } = abilities;
    match event {
        WorkerEvent::Ready { perceiver } => {
            ffi::info(&format!("camera analysis ready with {perceiver}"));
            let running = VisionStatus::Running {
                latency_ms: 0.0,
                mouths,
                hands,
            };
            hub.update(|engine, now| engine.set_vision_status(running, now));
        }
        WorkerEvent::Detected {
            perception,
            captured_ms,
            latency_ms,
        } => {
            let latency_ms = (latency_ms * 10.0).round() / 10.0;
            let signals = signal_summary(&perception);
            let sighting = sighting(&perception);
            hub.update(|engine, now| {
                let observed = engine.observe(f64::from(perception.best_face), captured_ms);
                let signaled = engine.observe_signals(&signals, captured_ms);
                let facepalm = engine.observe_facepalm(perception.facepalm(), captured_ms);
                let seen = engine.observe_sighting(&sighting, captured_ms);
                let running = VisionStatus::Running {
                    latency_ms,
                    mouths,
                    hands,
                };
                let status = engine.set_vision_status(running, now);
                merge(merge(merge(merge(observed, signaled), facepalm), seen), status)
            });
            preview.observe(&perception, captured_ms, hub.now_ms());
            let hook = hook.read().unwrap_or_else(PoisonError::into_inner).clone();
            if let Some(hook) = hook
                && catch_unwind(AssertUnwindSafe(|| hook(&perception))).is_err()
            {
                ffi::warn("the perception hook panicked");
            }
        }
        WorkerEvent::Failed(message) => {
            ffi::warn(&format!("camera analysis failed: {message}"));
            hub.update(|engine, now| engine.set_vision_status(VisionStatus::Failed { message }, now));
        }
    }
}

fn merge(a: Outcome, b: Outcome) -> Outcome {
    Outcome {
        changed: a.changed || b.changed,
        project_dirty: a.project_dirty || b.project_dirty,
        telemetry: a.telemetry || b.telemetry,
    }
}
