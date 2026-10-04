// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the background vision worker: startup and perception events, the perceiver's
//! name and input size, boxed perceivers, dropping samples while busy, failure reporting for
//! setup errors, analysis errors, and panics, which failures stop the worker for good, prompt
//! shutdown, and event callbacks that submit or shut down from the worker thread.

mod support {
    pub mod events;
    pub mod fakes;
}

use std::path::Path;
use std::sync::mpsc::{RecvTimeoutError, TryRecvError, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use sceneuron_vision::{DetectError, InputSize, Perceiver, Sample, UltraFace, VisionWorker, WorkerEvent};
use support::events::{
    QUIET, WAIT, assert_detected, assert_failed, assert_quiet, assert_ready, next, recorder, sample, spawn_ready,
};
use support::fakes::{FAKE_INPUT, FailsOnce, Fake, GATE_WAIT, Panics};

#[test]
fn reports_ready_then_detections_with_latency() {
    let (worker, events) = spawn_ready(Fake {
        delay: Duration::from_millis(20),
        ..Fake::new(0.8)
    });
    assert!(worker.is_idle());
    assert!(worker.try_submit(sample(42)));
    let latency_ms = assert_detected(next(&events), 0.8, 42);
    assert!(latency_ms >= 20.0, "latency {latency_ms}");
    assert!(worker.is_idle());
    assert!(worker.try_submit(sample(43)));
    assert_detected(next(&events), 0.8, 43);
}

#[test]
fn ready_names_the_perceiver_and_publishes_its_input_size() {
    let (on_event, events) = recorder();
    let worker = VisionWorker::spawn(|| Ok(Fake::new(0.3)), on_event);
    assert_eq!(next(&events), WorkerEvent::Ready { perceiver: "fake" });
    assert_eq!(worker.input_size(), Some(FAKE_INPUT));
}

#[test]
fn runs_a_boxed_perceiver() {
    let (on_event, events) = recorder();
    let worker = VisionWorker::spawn(|| Ok(Box::new(Fake::new(0.7)) as Box<dyn Perceiver>), on_event);
    assert_eq!(next(&events), WorkerEvent::Ready { perceiver: "fake" });
    assert!(worker.try_submit(sample(9)));
    assert_detected(next(&events), 0.7, 9);
}

#[test]
fn is_not_idle_until_the_perceiver_exists() {
    let (release, gate) = channel::<()>();
    let (on_event, events) = recorder();
    let worker = VisionWorker::spawn(
        move || {
            gate.recv_timeout(GATE_WAIT).expect("test releases setup");
            Ok(Fake::new(0.1))
        },
        on_event,
    );
    assert!(!worker.is_idle());
    assert_eq!(worker.input_size(), None);
    assert!(!worker.try_submit(sample(1)));
    release.send(()).unwrap();
    assert_ready(next(&events));
    assert!(worker.is_idle());
    assert!(worker.input_size().is_some());
}

#[test]
fn drops_samples_while_busy() {
    let (release, gate) = channel();
    let (worker, events) = spawn_ready(Fake {
        gate: Some(gate),
        ..Fake::new(0.6)
    });
    assert!(worker.try_submit(sample(1)));
    assert!(!worker.is_idle());
    assert!(!worker.try_submit(sample(2)));
    assert!(!worker.try_submit(sample(3)));
    release.send(()).unwrap();
    assert_detected(next(&events), 0.6, 1);
    assert_quiet(&events);
    release.send(()).unwrap();
    assert!(worker.try_submit(sample(4)));
    assert_detected(next(&events), 0.6, 4);
}

#[test]
fn setup_error_reports_failed_and_stops() {
    let (on_event, events) = recorder();
    let worker = VisionWorker::spawn(
        || Err::<Fake, _>(DetectError::Load("model file missing".into())),
        on_event,
    );
    assert_failed(next(&events), "model file missing");
    assert!(!worker.is_idle());
    assert!(!worker.try_submit(sample(1)));
    assert!(
        stops(&worker),
        "a worker whose perceiver never started is stopped for good"
    );
    assert_quiet(&events);
}

/// Waits briefly for the worker to report itself stopped.
fn stops(worker: &VisionWorker) -> bool {
    let deadline = Instant::now() + WAIT;
    while !worker.is_stopped() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    worker.is_stopped()
}

#[test]
fn setup_panic_reports_failed_and_stops() {
    let (on_event, events) = recorder();
    let worker = VisionWorker::spawn(|| -> Result<Fake, DetectError> { panic!("no camera today") }, on_event);
    assert_failed(next(&events), "no camera today");
    assert!(!worker.try_submit(sample(1)));
}

#[test]
fn analysis_error_reports_failed_and_keeps_running() {
    let (worker, events) = spawn_ready(FailsOnce(false));
    assert!(worker.try_submit(sample(1)));
    assert_failed(next(&events), "model hiccup");
    assert!(worker.is_idle());
    assert!(!worker.is_stopped());
    assert!(worker.try_submit(sample(2)));
    assert_detected(next(&events), 0.0, 2);
}

#[test]
fn perceiver_panic_reports_failed_and_rejects_samples() {
    let (worker, events) = spawn_ready(Panics);
    assert!(worker.try_submit(sample(1)));
    assert_failed(next(&events), "perceiver exploded");
    assert!(!worker.is_idle());
    assert!(!worker.try_submit(sample(2)));
    assert!(stops(&worker));
    assert_quiet(&events);
}

#[test]
fn shutdown_is_prompt_when_idle() {
    let (worker, events) = spawn_ready(Fake::new(0.5));
    let started = Instant::now();
    worker.shutdown();
    assert!(started.elapsed() < Duration::from_secs(1), "{:?}", started.elapsed());
    assert_eq!(events.try_recv(), Err(TryRecvError::Disconnected));
}

#[test]
fn shutdown_while_busy_waits_for_the_analysis_and_discards_it() {
    let (announce, detecting) = channel();
    let (release, gate) = channel();
    let (worker, events) = spawn_ready(Fake {
        started: Some(announce),
        gate: Some(gate),
        ..Fake::new(0.9)
    });
    assert!(worker.try_submit(sample(1)));
    detecting.recv_timeout(WAIT).expect("analysis starts");
    let (stopped, shutdown_returned) = channel();
    let stopper = thread::spawn(move || {
        worker.shutdown();
        let _ = stopped.send(());
    });
    assert_eq!(shutdown_returned.recv_timeout(QUIET), Err(RecvTimeoutError::Timeout));
    let released = Instant::now();
    release.send(()).unwrap();
    shutdown_returned.recv_timeout(WAIT).expect("shutdown returns");
    assert!(released.elapsed() < Duration::from_secs(2), "{:?}", released.elapsed());
    stopper.join().unwrap();
    assert_eq!(events.recv_timeout(WAIT), Err(RecvTimeoutError::Disconnected));
}

#[test]
fn drop_during_setup_joins_without_reporting_ready() {
    let (on_event, events) = recorder();
    let worker = VisionWorker::spawn(
        || {
            // Long enough that the drop below lands during setup even on a loaded machine.
            thread::sleep(Duration::from_millis(500));
            Ok(Fake::new(0.1))
        },
        on_event,
    );
    let started = Instant::now();
    drop(worker);
    assert!(started.elapsed() < Duration::from_secs(3), "{:?}", started.elapsed());
    assert_eq!(events.recv_timeout(WAIT), Err(RecvTimeoutError::Disconnected));
}

#[test]
fn callbacks_can_submit_and_shut_down_from_the_worker_thread() {
    let slot: Arc<Mutex<Option<VisionWorker>>> = Arc::new(Mutex::new(None));
    let (release, gate) = channel::<()>();
    let (record, events) = recorder();
    let worker = VisionWorker::spawn(
        move || {
            gate.recv_timeout(GATE_WAIT).expect("test releases setup");
            Ok(Fake::new(0.4))
        },
        {
            let slot = Arc::clone(&slot);
            move |event: WorkerEvent| {
                let mut slot = slot.lock().unwrap_or_else(PoisonError::into_inner);
                match &event {
                    WorkerEvent::Ready { .. } => {
                        assert!(slot.as_ref().is_some_and(|worker| worker.try_submit(sample(5))));
                    }
                    _ => drop(slot.take()),
                }
                record(event);
            }
        },
    );
    *slot.lock().unwrap() = Some(worker);
    release.send(()).unwrap();
    assert_ready(next(&events));
    assert_detected(next(&events), 0.4, 5);
    assert_eq!(events.recv_timeout(WAIT), Err(RecvTimeoutError::Disconnected));
}

#[test]
fn runs_the_bundled_detector() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/face-close-320x240.rgb");
    let (on_event, events) = recorder();
    let worker = VisionWorker::spawn(UltraFace::new, on_event);
    assert_eq!(next(&events), WorkerEvent::Ready { perceiver: "UltraFace" });
    assert_eq!(
        worker.input_size(),
        Some(InputSize::Fixed {
            width: 320,
            height: 240
        })
    );
    assert!(worker.try_submit(Sample {
        rgb: std::fs::read(path).unwrap(),
        ..sample(7)
    }));
    let WorkerEvent::Detected { perception, .. } = next(&events) else {
        panic!("expected a perception");
    };
    assert!(perception.best_face >= 0.9, "{perception:?}");
    assert_eq!(perception.faces.len(), 1);
    assert!(
        perception.hands.is_empty() && perception.labels.is_empty(),
        "{perception:?}"
    );
    worker.shutdown();
}
