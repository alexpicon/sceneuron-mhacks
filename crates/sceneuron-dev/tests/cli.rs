// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the sceneuron-dev binary end to end: help and argument errors, serving the
//! API with simulated sources and demo presence, a control token given on the command line,
//! and quitting cleanly on a `q` line or a termination signal with the project saved.

mod common;

use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use common::{Running, WAIT, output, project_path, saved_automation};
use serde_json::json;

#[test]
fn help_prints_usage() {
    let result = output(&["--help"]);
    assert!(result.status.success());
    let usage = String::from_utf8_lossy(&result.stdout).into_owned();
    assert!(usage.starts_with("Usage: sceneuron-dev"));
    assert!(usage.contains("--demo-ai"), "{usage}");
}

#[test]
fn bad_arguments_exit_with_a_usage_error() {
    let result = output(&["--verbose"]);
    assert_eq!(result.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("unknown option \"--verbose\""), "{stderr}");
    assert!(stderr.contains("Usage:"));
}

#[test]
fn a_weak_token_is_refused() {
    let result = output(&["--port", "0", "--token", "short"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("invalid access token"));
}

#[test]
fn a_given_control_token_controls_until_it_is_revoked() {
    let control = "dev-control-token-0123456789";
    let running = Running::start(&["--port", "0", "--control-token", control]);
    loop {
        let line = running.lines.recv_timeout(WAIT).expect("the binary stopped printing");
        assert!(!line.contains(control), "the control token was printed: {line}");
        if line.starts_with("Type q") {
            break;
        }
    }
    let (status, view) = running.request_as(control, "GET", "/api/control/state", "");
    assert_eq!(status, 200, "{view}");
    assert!(view["elements"].is_array() && view.get("events").is_none());
    let (status, _) = running.request_as(control, "GET", "/api/state", "");
    assert_eq!(status, 403);
    let (status, reply) = running.request("POST", "/api/control/revoke", "");
    assert_eq!((status, reply), (200, json!({"ok": true})));
    let (status, _) = running.request_as(control, "GET", "/api/control/state", "");
    assert_eq!(status, 401);
}

#[test]
fn a_weak_or_reused_control_token_is_refused() {
    let token = "dev-test-token-0123456789";
    for control in ["short", token] {
        let result = output(&["--port", "0", "--token", token, "--control-token", control]);
        assert_eq!(result.status.code(), Some(1), "{control}");
        assert!(String::from_utf8_lossy(&result.stderr).contains("invalid access token"));
    }
}

#[test]
fn serves_simulated_sources_and_quits_on_q() {
    let mut running = Running::start(&["--port", "0"]);
    assert_eq!(running.token.len(), 32);
    let overlay = running.line_starting("Overlay:");
    assert!(overlay.contains(&format!(":{}/overlay#token=", running.port)));
    assert!(
        !overlay.contains(&running.token),
        "the overlay gets its own read-only token"
    );
    let (status, sources) = running.request("GET", "/api/sources", "");
    assert_eq!(status, 200);
    let sources = sources.as_array().unwrap().clone();
    assert_eq!(sources.len(), 2);
    let uuid = sources[0]["uuid"].as_str().unwrap();
    let (status, reply) = running.request("POST", "/api/sources/attach", &json!({ "uuid": uuid }).to_string());
    assert_eq!((status, reply), (200, json!({"ok": true})));
    assert_eq!(running.request("GET", "/api/sources", "").1[0]["analyzed"], true);
    let (status, _) = running.request("POST", "/api/sources/attach", r#"{"uuid":"nope"}"#);
    assert_eq!(status, 400);
    let (status, reply) = running.request("POST", "/api/overlay", "");
    assert_eq!((status, reply), (200, json!({"name": "Sceneuron overlay"})));
    assert!(running.quit_with_q().success());
    running.line_starting("Stopped.");
}

#[test]
fn demo_presence_drives_the_engine_and_quitting_saves_the_project() {
    let path = project_path("demo");
    let path_text = path.to_str().unwrap();
    let token = "dev-test-token-0123456789";
    let mut running = Running::start(&["--port=0", "--token", token, "--project", path_text, "--demo-presence"]);
    assert_eq!(running.token, token);
    let deadline = Instant::now() + WAIT;
    loop {
        let (_, state) = running.request("GET", "/api/state", "");
        if state["presence"]["state"] == "present" {
            assert_eq!(state["vision"]["state"], "running");
            break;
        }
        assert!(Instant::now() < deadline, "demo presence never became present: {state}");
        thread::sleep(Duration::from_millis(50));
    }
    let (status, _) = running.request("POST", "/api/automation", r#"{"enabled":false}"#);
    assert_eq!(status, 200);
    assert!(running.quit_with_q().success());
    assert!(!saved_automation(&path));

    let restarted = Running::start(&["--port", "0", "--project", path_text]);
    let (_, state) = restarted.request("GET", "/api/state", "");
    assert_eq!(state["automation_enabled"], false);
}

#[test]
fn a_termination_signal_quits_cleanly_and_saves() {
    for signal in ["-INT", "-TERM"] {
        let path = project_path(&format!("signal{signal}"));
        let mut running = Running::start(&["--port", "0", "--project", path.to_str().unwrap()]);
        let (status, _) = running.request("POST", "/api/automation", r#"{"enabled":false}"#);
        assert_eq!(status, 200);
        let pid = running.child.id().to_string();
        assert!(Command::new("kill").args([signal, &pid]).status().unwrap().success());
        assert!(running.wait_for_exit().success(), "{signal}");
        assert!(!saved_automation(&path), "{signal}");
    }
}
