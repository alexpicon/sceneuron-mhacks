// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Records worker events and asserts on them, for worker tests.

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::Duration;

use sceneuron_vision::{INPUT_HEIGHT, INPUT_WIDTH, Perceiver, Sample, VisionWorker, WorkerEvent};

/// Upper bound for anything the tests wait on; generous so slow machines never flake.
pub const WAIT: Duration = Duration::from_secs(30);
/// How long the worker must stay silent for [`assert_quiet`] to pass.
pub const QUIET: Duration = Duration::from_millis(150);

pub fn recorder() -> (impl Fn(WorkerEvent) + Send + 'static, Receiver<WorkerEvent>) {
    let (sender, receiver): (Sender<WorkerEvent>, _) = channel();
    let on_event = move |event| {
        let _ = sender.send(event);
    };
    (on_event, receiver)
}

pub fn next(events: &Receiver<WorkerEvent>) -> WorkerEvent {
    events.recv_timeout(WAIT).expect("worker event")
}

pub fn assert_quiet(events: &Receiver<WorkerEvent>) {
    match events.recv_timeout(QUIET) {
        Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {}
        Ok(event) => panic!("unexpected event {event:?}"),
    }
}

pub fn sample(captured_ms: u64) -> Sample {
    Sample {
        rgb: vec![0; (INPUT_WIDTH * INPUT_HEIGHT * 3) as usize],
        width: INPUT_WIDTH,
        height: INPUT_HEIGHT,
        captured_ms,
    }
}

pub fn spawn_ready<P: Perceiver + 'static>(perceiver: P) -> (VisionWorker, Receiver<WorkerEvent>) {
    let (on_event, events) = recorder();
    let worker = VisionWorker::spawn(move || Ok(perceiver), on_event);
    assert_ready(next(&events));
    (worker, events)
}

pub fn assert_ready(event: WorkerEvent) {
    assert!(
        matches!(event, WorkerEvent::Ready { .. }),
        "expected ready, got {event:?}"
    );
}

/// Asserts a perception whose best face is `score` for the sample captured at `captured`;
/// returns its latency.
pub fn assert_detected(event: WorkerEvent, score: f32, captured: u64) -> f64 {
    match event {
        WorkerEvent::Detected {
            perception,
            captured_ms,
            latency_ms,
        } => {
            assert_eq!(perception.best_face, score);
            assert_eq!(captured_ms, captured);
            assert!(latency_ms.is_finite() && latency_ms >= 0.0, "latency {latency_ms}");
            latency_ms
        }
        other => panic!("expected a detection, got {other:?}"),
    }
}

pub fn assert_failed(event: WorkerEvent, needle: &str) {
    match event {
        WorkerEvent::Failed(message) => assert!(message.contains(needle), "{message}"),
        other => panic!("expected a failure, got {other:?}"),
    }
}
