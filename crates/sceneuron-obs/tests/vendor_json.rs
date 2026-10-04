// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the obs_data-safe view of vendor responses: nulls are dropped, lists of plain
//! values become comma-separated strings, and empty lists and lists of objects keep their shape.

use sceneuron::vendor_json::obs_data_view;
use serde_json::json;

#[test]
fn reshapes_what_obs_data_cannot_hold() {
    let snapshot = json!({
        "revision": 3,
        "presence": { "state": "away", "last_score": null },
        "elements": [
            { "id": "brb", "overridden": ["visible", "text"], "automated": [], "effect": null },
        ],
        "ai": { "last_latency_ms": null, "cost_usd": 0.5 },
    });
    assert_eq!(
        obs_data_view(&snapshot),
        json!({
            "revision": 3,
            "presence": { "state": "away" },
            "elements": [{ "id": "brb", "overridden": "visible,text", "automated": [] }],
            "ai": { "cost_usd": 0.5 },
        })
    );
}

#[test]
fn plain_lists_of_numbers_and_mixed_values_become_text() {
    assert_eq!(obs_data_view(&json!([1, 2.5, true, null, "x"])), json!("1,2.5,true,x"));
}
