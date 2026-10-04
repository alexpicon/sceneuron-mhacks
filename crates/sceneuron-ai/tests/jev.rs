// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the Jev client: one yes/no question per condition about on-device facts, the
//! request it sends, probabilities and usage it reads, refusals of malformed answers, and how
//! errors and retries work.

mod common;

use common::condition;
use common::server::{MockServer, Reply};
use sceneuron_ai::{AiError, AiSettings, JevClient, parse_quick, quick_request, quick_verdict};
use sceneuron_core::Condition;
use serde_json::json;

const JEV_KEY: &str = "jev-test-key-0123456789";

fn conditions() -> Vec<Condition> {
    vec![
        condition("pet-in-view", "A cat or dog is visible.", "the kind of animal"),
        condition("thumbs-up", "The person gives a thumbs up.", ""),
    ]
}

fn settings(server: &MockServer) -> AiSettings {
    AiSettings {
        jev_api_key: JEV_KEY.to_owned(),
        jev_base_url: server.url(),
        ..AiSettings::default()
    }
}

fn answer(p0: f64, p1: f64) -> Reply {
    Reply::json(
        200,
        json!({"model": "jev-1.13.0", "answers": {
            "q0": {"type": "noul", "noul": p0}, "q1": {"type": "noul", "noul": p1}
        }, "usage": {"input_tokens": 296, "output_tokens": 20}}),
    )
}

#[test]
fn asks_one_yes_no_question_per_condition_about_the_facts() {
    let list = conditions();
    let refs: Vec<&Condition> = list.iter().collect();
    let body = quick_request(&refs, &["cat (0.93)".to_owned()], "jev-latest");
    assert_eq!(body["model"], "jev-latest");
    assert_eq!(body["state"]["camera_facts"], json!(["cat (0.93)"]));
    assert_eq!(body["questions"]["q0"]["type"], "noul");
    assert!(
        body["questions"]["q1"]["instructions"]
            .as_str()
            .unwrap()
            .contains("thumbs up")
    );
    assert!(body["state"]["about"].as_str().unwrap().contains("not instructions"));
}

#[test]
fn reads_probabilities_and_refuses_malformed_answers() {
    let list = conditions();
    let refs: Vec<&Condition> = list.iter().collect();
    let partial = json!({"answers": {"q1": {"type": "noul", "noul": 0.2}}});
    assert_eq!(parse_quick(&partial, &refs).unwrap(), [("thumbs-up".to_owned(), 0.2)]);
    let out_of_range = json!({"answers": {"q0": {"type": "noul", "noul": 1.5}}});
    assert!(matches!(parse_quick(&out_of_range, &refs), Err(AiError::Invalid(_))));
    assert!(parse_quick(&json!({}), &refs).is_err());
}

#[test]
fn a_probability_becomes_a_verdict_with_its_reason() {
    let yes = quick_verdict("pet-in-view", 0.9);
    assert!(yes.holds && (yes.confidence - 0.9).abs() < 1e-9);
    assert!(yes.reason.contains("90% likely") && yes.detail.is_empty());
    let no = quick_verdict("pet-in-view", 0.1);
    assert!(!no.holds && (no.confidence - 0.9).abs() < 1e-9);
}

#[test]
fn the_client_sends_its_key_and_reads_usage() {
    let server = MockServer::start(vec![answer(0.95, 0.05)]);
    let client = JevClient::new(&settings(&server)).unwrap();
    let list = conditions();
    let refs: Vec<&Condition> = list.iter().collect();
    let judgment = client.judge(&refs, &["cat (0.93)".to_owned()]).unwrap();
    assert_eq!(
        judgment.probabilities,
        [("pet-in-view".to_owned(), 0.95), ("thumbs-up".to_owned(), 0.05)]
    );
    assert_eq!(
        (judgment.usage.input_tokens, judgment.model.as_str()),
        (296, "jev-1.13.0")
    );
    assert!(judgment.cost_usd() > 0.0 && judgment.cost_usd() < 0.0001);
    let request = &server.requests()[0];
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("POST", "/v1/systemone")
    );
    assert_eq!(
        request.header("authorization"),
        Some(format!("Bearer {JEV_KEY}").as_str())
    );
}

#[test]
fn no_key_means_no_client() {
    assert!(JevClient::new(&AiSettings::default()).is_none());
}

#[test]
fn a_rejected_key_is_permanent_and_overload_is_retried() {
    let list = conditions();
    let refs: Vec<&Condition> = list.iter().collect();
    let rejected = MockServer::start(vec![Reply::json(401, json!({"error": "Missing or invalid API key"}))]);
    let error = JevClient::new(&settings(&rejected))
        .unwrap()
        .judge(&refs, &[])
        .unwrap_err();
    assert!(error.is_permanent() && !error.to_string().contains(JEV_KEY), "{error}");
    let busy = MockServer::start(vec![Reply::json(529, json!({"error": "overloaded"})), answer(0.6, 0.4)]);
    let judgment = JevClient::new(&settings(&busy)).unwrap().judge(&refs, &[]).unwrap();
    assert_eq!(judgment.probabilities.len(), 2);
    assert_eq!(busy.requests().len(), 2);
}
