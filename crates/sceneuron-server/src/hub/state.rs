// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What the engine lock guards, and the numbered views taken under it.

use std::sync::Arc;

use sceneuron_core::{Engine, Snapshot};

use crate::control_view::ControlView;

pub(super) struct EngineState {
    pub(super) engine: Engine,
    /// Orders views taken under this lock, so a slow publisher never overwrites a newer one.
    sequence: u64,
    /// Telemetry changed since the last published snapshot.
    pub(super) telemetry_pending: bool,
}

/// A change's snapshot and control view, taken together under the engine lock.
pub(super) struct Published {
    pub(super) sequence: u64,
    pub(super) snapshot: Snapshot,
    pub(super) control: Arc<ControlView>,
}

impl EngineState {
    pub(super) fn new(engine: Engine) -> Self {
        Self {
            engine,
            sequence: 0,
            telemetry_pending: false,
        }
    }

    pub(super) fn take_snapshot(&mut self, now_ms: u64) -> (u64, Snapshot) {
        self.sequence += 1;
        (self.sequence, self.engine.snapshot(now_ms))
    }

    pub(super) fn take_published(&mut self, now_ms: u64) -> Published {
        let (sequence, snapshot) = self.take_snapshot(now_ms);
        let control = Arc::new(ControlView::new(&snapshot, self.engine.project()));
        Published {
            sequence,
            snapshot,
            control,
        }
    }

    pub(super) fn take_control_view(&mut self, now_ms: u64) -> (u64, Arc<ControlView>) {
        let published = self.take_published(now_ms);
        (published.sequence, published.control)
    }

    pub(super) fn control_view(&self, now_ms: u64) -> ControlView {
        ControlView::new(&self.engine.snapshot(now_ms), self.engine.project())
    }
}
