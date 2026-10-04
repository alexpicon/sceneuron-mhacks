// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The background thread: presence timeouts, throttled telemetry, and debounced saves.

use std::time::Instant;

use super::schedule::Due;
use super::shared::Shared;

pub(super) fn run(shared: &Shared) {
    while let Some(due) = wait_for_work(shared) {
        if due.tick {
            shared.tick();
        }
        if due.telemetry {
            shared.publish_telemetry();
        }
        if due.save {
            // A failure is reported to listeners and retried by the schedule.
            let _ = shared.save_now();
        }
    }
}

/// Sleeps until some work is due; `None` once the hub is stopping.
fn wait_for_work(shared: &Shared) -> Option<Due> {
    let mut schedule = shared.lock_schedule();
    loop {
        if schedule.stopped {
            return None;
        }
        let now = Instant::now();
        let deadline = schedule.next_deadline(now, &shared.config);
        if deadline <= now {
            return Some(schedule.take_due(now, &shared.config));
        }
        schedule = match shared.wake.wait_timeout(schedule, deadline - now) {
            Ok((guard, _)) => guard,
            Err(poisoned) => poisoned.into_inner().0,
        };
    }
}
