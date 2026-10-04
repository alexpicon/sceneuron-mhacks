// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Short forms for gate policies, decisions, and a gate that already analyzed a picture.

use sceneuron_ai::{Decision, Gate, Thumbnail};
use sceneuron_core::AiPolicy;

pub fn policy() -> AiPolicy {
    AiPolicy::default()
}

pub fn call(reason: &'static str) -> Decision {
    Decision::Call { reason }
}

pub fn wait(until_ms: u64) -> Decision {
    Decision::Wait { until_ms }
}

/// A gate that analyzed a black picture with a call at `at_ms`.
pub fn analyzed_at(at_ms: u64) -> Gate {
    let mut gate = Gate::new();
    gate.started(at_ms, Thumbnail::uniform(0));
    gate.succeeded();
    gate
}
