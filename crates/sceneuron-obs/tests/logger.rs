// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the bridge from log records to the OBS log: which records it forwards and the OBS
//! level each one gets.

use log::{Level, Metadata};
use sceneuron::logger::{forwards, obs_level};
use sceneuron_obs_sys::{LOG_DEBUG, LOG_ERROR, LOG_INFO, LOG_WARNING};

fn record(target: &str) -> Metadata<'_> {
    Metadata::builder().target(target).level(Level::Debug).build()
}

#[test]
fn only_sceneuron_records_reach_the_obs_log() {
    for ours in ["sceneuron_ai::gemini", "sceneuron", "sceneuron_server::hub"] {
        assert!(forwards(&record(ours)), "{ours}");
    }
    // tungstenite traces the WebSocket handshake with the ElevenLabs key in it.
    for theirs in [
        "ureq::unversioned",
        "rustls::client",
        "tract_onnx",
        "tungstenite::handshake::client",
    ] {
        assert!(!forwards(&record(theirs)), "{theirs}");
    }
}

#[test]
fn levels_map_to_obs_levels() {
    let levels = [Level::Error, Level::Warn, Level::Info, Level::Debug, Level::Trace].map(obs_level);
    assert_eq!(levels, [LOG_ERROR, LOG_WARNING, LOG_INFO, LOG_DEBUG, LOG_DEBUG]);
}
