// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! When the background thread next has work: ticks, telemetry, and saves.

use std::time::{Duration, Instant};

use super::HubConfig;

/// Keeps a zero tick interval from turning the background thread into a busy loop.
const MIN_TICK: Duration = Duration::from_millis(1);

/// Work that became due.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Due {
    pub(super) tick: bool,
    pub(super) telemetry: bool,
    pub(super) save: bool,
}

#[derive(Debug)]
pub(super) struct Schedule {
    pub(super) stopped: bool,
    next_tick: Instant,
    dirty: bool,
    save_due: Option<Instant>,
    telemetry_wanted: bool,
    last_publish: Option<Instant>,
}

impl Schedule {
    pub(super) fn new(now: Instant, config: &HubConfig) -> Self {
        Self {
            stopped: false,
            next_tick: now + tick_interval(config),
            dirty: false,
            save_due: None,
            telemetry_wanted: false,
            last_publish: None,
        }
    }

    /// The earliest time something becomes due.
    pub(super) fn next_deadline(&self, now: Instant, config: &HubConfig) -> Instant {
        let mut deadline = self.next_tick;
        if let Some(save_due) = self.save_due {
            deadline = deadline.min(save_due);
        }
        if self.telemetry_wanted {
            deadline = deadline.min(self.telemetry_due(now, config));
        }
        deadline
    }

    /// Returns the work due at `now` and reschedules it.
    pub(super) fn take_due(&mut self, now: Instant, config: &HubConfig) -> Due {
        let mut due = Due::default();
        if now >= self.next_tick {
            due.tick = true;
            self.next_tick = now + tick_interval(config);
        }
        if self.telemetry_wanted && now >= self.telemetry_due(now, config) {
            due.telemetry = true;
            self.telemetry_wanted = false;
        }
        if self.save_due.is_some_and(|save_due| now >= save_due) {
            due.save = true;
            self.save_due = None;
        }
        due
    }

    /// Records a project change; the save waits for `debounce` without further changes.
    pub(super) fn mark_dirty(&mut self, now: Instant, debounce: Duration) {
        self.dirty = true;
        self.save_due = Some(now + debounce);
    }

    /// Clears the unsaved-changes flag and returns whether it was set.
    pub(super) fn take_dirty(&mut self) -> bool {
        self.save_due = None;
        std::mem::take(&mut self.dirty)
    }

    /// Keeps the changes unsaved and tries again at `at`, unless a newer change already
    /// scheduled a save.
    pub(super) fn retry_save(&mut self, at: Instant) {
        self.dirty = true;
        if self.save_due.is_none() {
            self.save_due = Some(at);
        }
    }

    pub(super) fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub(super) fn want_telemetry(&mut self) {
        self.telemetry_wanted = true;
    }

    pub(super) fn published(&mut self, now: Instant) {
        self.last_publish = Some(now);
    }

    fn telemetry_due(&self, now: Instant, config: &HubConfig) -> Instant {
        self.last_publish.map_or(now, |last| last + config.telemetry_interval)
    }
}

fn tick_interval(config: &HubConfig) -> Duration {
    config.tick_interval.max(MIN_TICK)
}
