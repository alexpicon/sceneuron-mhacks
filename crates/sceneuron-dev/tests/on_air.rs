// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises sceneuron-dev's stand-in for OBS's program: the scene "Scene 01" off air at the
//! start, and lines on standard input that start and stop streaming and recording and switch the
//! scene, as the state and a starter project's on-air pill show them.

mod common;

use common::{Running, project_path};
use serde_json::{Value, json};

fn on_air(running: &Running) -> Value {
    running.request("GET", "/api/state", "").1["on_air"].clone()
}

fn pill(running: &Running) -> Value {
    let (_, state) = running.request("GET", "/api/state", "");
    let elements = state["elements"].as_array().unwrap();
    elements.iter().find(|element| element["id"] == "on-air").unwrap()["props"]["text"].clone()
}

#[test]
fn the_simulated_obs_goes_on_and_off_air_and_switches_scenes() {
    let path = project_path("on-air");
    let mut running = Running::start(&["--port", "0", "--project", path.to_str().unwrap()]);
    running.line_starting("Type stream, record, stop, or scene NAME");
    assert_eq!(
        on_air(&running),
        json!({"scene": "Scene 01", "streaming": false, "recording": false, "since_ms": null})
    );
    assert_eq!(pill(&running), "Scene 01", "the starter's pill, off air");

    running.say("stream");
    running.line_starting("OBS (simulated): streaming, in \"Scene 01\"");
    let streaming = on_air(&running);
    assert_eq!(
        (&streaming["streaming"], &streaming["recording"]),
        (&json!(true), &json!(false))
    );
    let since = streaming["since_ms"].as_u64().expect("on air since the stream started");
    assert_eq!(pill(&running), "Scene 01 \u{b7} LIVE");

    running.say("record");
    running.line_starting("OBS (simulated): recording");
    assert_eq!(
        on_air(&running)["since_ms"],
        since,
        "still on air since the stream started"
    );
    running.say("scene Interview");
    running.line_starting("OBS (simulated): switched scenes, in \"Interview\"");
    assert_eq!(pill(&running), "Interview \u{b7} LIVE");

    running.say("dance");
    running.say("stop");
    running.line_starting("OBS (simulated): off air");
    assert_eq!(
        on_air(&running),
        json!({"scene": "Interview", "streaming": false, "recording": false, "since_ms": null})
    );
    assert!(running.quit_with_q().success());
}
