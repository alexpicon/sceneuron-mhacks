// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the real AI mode of the development server: optional keys from a `.env` file are
//! used but never shown or saved, and the server runs the same without any key.

mod common;

use std::fs;

use common::{Running, project_path};

#[test]
fn keys_from_an_env_file_are_used_but_not_shown_or_saved() {
    let dir = project_path("real-ai").parent().unwrap().to_path_buf();
    let env = dir.join("keys.env");
    let settings = dir.join("ai.json");
    fs::write(&env, "# test keys\nSCENEURON_JEV_API_KEY=jev-test-key-0123456789\n").unwrap();
    let env_arg = env.to_str().unwrap().to_owned();
    let settings_arg = settings.to_str().unwrap().to_owned();
    let mut running = Running::start(&[
        "--port",
        "0",
        "--ai",
        "--env-file",
        &env_arg,
        "--ai-settings",
        &settings_arg,
    ]);
    let (status, ai) = running.request("GET", "/api/ai", "");
    assert_eq!(status, 200);
    assert_eq!(ai["has_jev_key"], true);
    assert_eq!(ai["jev_key_source"], "environment");
    assert_eq!(ai["jev_key_hint"], "...6789");
    assert_eq!(ai["has_key"], false);
    let text = ai.to_string();
    assert!(!text.contains("jev-test-key"));
    let (status, _) = running.request("PUT", "/api/ai", r#"{"consent": true}"#);
    assert_eq!(status, 200);
    assert!(!fs::read_to_string(&settings).unwrap().contains("jev-test-key"));
    assert!(running.quit_with_q().success());
}

#[test]
fn the_real_ai_mode_runs_without_any_key() {
    let dir = project_path("real-ai-none").parent().unwrap().to_path_buf();
    let settings = dir.join("ai.json");
    let missing = dir.join("no-such.env");
    let (settings_arg, missing_arg) = (
        settings.to_str().unwrap().to_owned(),
        missing.to_str().unwrap().to_owned(),
    );
    let mut running = Running::start(&[
        "--port",
        "0",
        "--ai",
        "--env-file",
        &missing_arg,
        "--ai-settings",
        &settings_arg,
    ]);
    let (status, ai) = running.request("GET", "/api/ai", "");
    assert_eq!(
        (status, ai["available"].as_bool(), ai["has_key"].as_bool()),
        (200, Some(true), Some(false))
    );
    let (status, state) = running.request("GET", "/api/state", "");
    assert_eq!((status, state["ai"]["state"].as_str()), (200, Some("off")));
    let (status, reply) = running.request("POST", "/api/ai/plan", r#"{"request": "add a hype banner"}"#);
    assert_eq!(status, 400, "{reply}");
    assert!(reply["error"].as_str().unwrap().contains("key"), "{reply}");
    assert!(running.quit_with_q().success());
}
