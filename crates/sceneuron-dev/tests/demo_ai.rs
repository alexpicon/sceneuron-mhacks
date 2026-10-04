// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises sceneuron-dev with and without --demo-ai: a build without AI says so, the simulated
//! AI is labeled, simulates Gemini, and needs consent before it watches, and it turns conditions
//! on from its scripted camera while its plans apply with the project revision they were made for.

mod common;

use std::thread;
use std::time::{Duration, Instant};

use common::{Running, WAIT};
use serde_json::{Value, json};

/// Polls the state until `ready` accepts it, asking the simulated AI to look before each try.
fn state_when(running: &Running, what: &str, ready: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + WAIT;
    loop {
        let (_, state) = running.request("GET", "/api/state", "");
        if ready(&state) {
            return state;
        }
        assert!(Instant::now() < deadline, "{what} never happened: {state}");
        running.request("POST", "/api/ai/analyze", "");
        thread::sleep(Duration::from_millis(400));
    }
}

#[test]
fn without_demo_ai_the_server_reports_no_ai() {
    let running = Running::start(&["--port", "0"]);
    assert_eq!(
        running.request("GET", "/api/ai", ""),
        (200, json!({"available": false}))
    );
}

#[test]
fn demo_ai_is_labeled_and_needs_consent_before_watching() {
    let running = Running::start(&["--port", "0", "--demo-ai"]);
    assert!(running.line_starting("Demo AI:").contains("simulated"));
    let (status, settings) = running.request("GET", "/api/ai", "");
    assert_eq!(status, 200);
    assert_eq!(
        [
            &settings["available"],
            &settings["simulated"],
            &settings["enabled"],
            &settings["has_key"]
        ],
        [&json!(true), &json!(true), &json!(false), &json!(true)]
    );
    let state = state_when(&running, "the needs-setup status", |state| {
        state["ai"]["state"] == "needs_setup"
    });
    assert!(state["ai"]["message"].as_str().unwrap().starts_with("Simulated"));
    let (status, _) = running.request("PUT", "/api/ai", r#"{"enabled":true}"#);
    assert_eq!(status, 400);
    let (status, _) = running.request("POST", "/api/ai/analyze", "");
    assert_eq!(status, 400, "looking needs watching on");
    let (status, _) = running.request("POST", "/api/ai/plan", r#"{"request":"wave","use_camera":true}"#);
    assert_eq!(status, 400, "the camera needs consent");
    let (status, reply) = running.request("PUT", "/api/ai", r#"{"api_key":"sk-test-wxyz"}"#);
    assert_eq!((status, &reply["key_hint"]), (200, &json!("wxyz")));
    assert!(!reply.to_string().contains("sk-test"));
}

#[test]
fn demo_ai_simulates_gemini_and_a_new_provider_gets_its_url_unless_one_is_given() {
    let running = Running::start(&["--port", "0", "--demo-ai"]);
    let url = |settings: &Value| (settings["provider"].clone(), settings["base_url"].clone());
    let (_, settings) = running.request("GET", "/api/ai", "");
    assert_eq!(
        url(&settings),
        (json!("gemini"), json!("https://generativelanguage.googleapis.com"))
    );
    let (_, settings) = running.request("PUT", "/api/ai", r#"{"provider":"openai-compatible"}"#);
    assert_eq!(
        url(&settings),
        (json!("openai-compatible"), json!("http://127.0.0.1:11434/v1"))
    );
    let (status, _) = running.request("PUT", "/api/ai", r#"{"provider":"anthropic"}"#);
    assert_eq!(status, 400, "Anthropic is no longer offered");
    let given = r#"{"provider":"gemini","base_url":"https://proxy.example.com"}"#;
    let (_, settings) = running.request("PUT", "/api/ai", given);
    assert_eq!(url(&settings), (json!("gemini"), json!("https://proxy.example.com")));
}

#[test]
fn demo_ai_flips_conditions_and_plans_apply_with_the_base_revision() {
    let running = Running::start(&["--port", "0", "--demo-ai"]);
    let (status, settings) = running.request("PUT", "/api/ai", r#"{"consent":true,"enabled":true}"#);
    assert_eq!((status, &settings["enabled"]), (200, &json!(true)));
    let state = state_when(&running, "a held item", |state| {
        state["conditions"][0]["on"] == true && state["ai"]["calls"].as_u64() >= Some(2)
    });
    assert_eq!(state["conditions"][0]["detail"], "Mechanical keyboard");
    assert!(state["ai"]["model"].as_str().unwrap().ends_with("(simulated)"));
    let showing = state["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == "now-showing")
        .unwrap();
    assert_eq!(showing["props"]["text"], "Now showing: Mechanical keyboard");
    assert_eq!(showing["props"]["visible"], true);
    assert!(state["scene"]["caption"].as_str().unwrap().contains("keyboard"));
    let events = state["events"].as_array().unwrap();
    assert!(events.iter().any(|event| event["source"] == "ai"), "{events:?}");

    let (status, plan) = running.request("POST", "/api/ai/plan", r#"{"request":"Say hi when I wave"}"#);
    assert_eq!(status, 200, "{plan}");
    assert!(plan["summary"].as_str().unwrap().starts_with("Simulated"));
    assert_eq!(plan["operations"][0]["op"], "add_condition");
    assert_eq!(plan["model"], "simulated-planner (simulated)");
    let base = plan["base_revision"].as_u64().unwrap();
    let if_match = format!("If-Match: {base}\r\n");
    let body = plan["project"].to_string();
    let (status, saved) = running.request_with("PUT", "/api/project", &if_match, &body);
    assert_eq!(status, 200, "{saved}");
    assert!(
        saved["conditions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == "waving")
    );
    let (status, conflict) = running.request_with("PUT", "/api/project", &if_match, &body);
    assert_eq!((status, &conflict["project_revision"]), (409, &json!(base + 1)));

    let (status, _) = running.request("PUT", "/api/ai", r#"{"consent":false}"#);
    assert_eq!(status, 200);
    state_when(&running, "conditions off after withdrawing consent", |state| {
        state["ai"]["state"] == "needs_setup"
            && state["conditions"].as_array().unwrap().iter().all(|c| c["on"] == false)
    });
}
