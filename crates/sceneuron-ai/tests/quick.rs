// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the quick judge inside the director: Jev's judgments of on-device facts reach the
//! host as verdicts, unchanged facts are not asked again, unsure or name-needing answers bring a
//! full look from the vision model, and no key, a rejected key, or a spent budget keep it quiet.

mod common;

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use common::director::{connector, pet_condition};
use common::server::{MockServer, Reply};
use common::sink::{RecordingSink, wait_until};
use common::{condition, enabled_settings, gray_frame};
use sceneuron_ai::{AiSettings, Director, MockProvider, ModelRequest};
use sceneuron_core::{AiPolicy, Condition};
use serde_json::json;

fn answers(probability: f64, count: usize) -> Vec<Reply> {
    let reply = Reply::json(
        200,
        json!({"model": "jev-1.13.0", "answers": {"q0": {"type": "noul", "noul": probability}},
               "usage": {"input_tokens": 100, "output_tokens": 0}}),
    );
    vec![reply; count]
}

fn thumbs() -> Vec<Condition> {
    vec![condition("thumbs-up", "The person gives a thumbs up.", "")]
}

/// Only reasons start a vision call; refreshes and image changes never do.
fn quiet_policy() -> AiPolicy {
    AiPolicy {
        min_interval_ms: 1_000,
        refresh_ms: 3_600_000,
        change_threshold: 1.0,
        ..AiPolicy::default()
    }
}

fn vision_mock() -> Arc<MockProvider> {
    Arc::new(MockProvider::new(|_: &ModelRequest| {
        Ok(json!({"caption": "", "verdicts": []}))
    }))
}

fn start(
    jev: &MockServer,
    conditions: Vec<Condition>,
    policy: AiPolicy,
) -> (Director, Arc<RecordingSink>, Arc<MockProvider>) {
    let settings = AiSettings {
        jev_api_key: "jev-test-key-0123456789".to_owned(),
        jev_base_url: jev.url(),
        ..enabled_settings()
    };
    let vision = vision_mock();
    let sink = Arc::new(RecordingSink::default());
    let director = Director::spawn(settings, connector(&vision), Arc::clone(&sink) as _);
    director.set_project(conditions, policy, false);
    (director, sink, vision)
}

fn facts(text: &str) -> Vec<String> {
    vec![text.to_owned()]
}

#[test]
fn judgments_reach_the_host_and_unchanged_facts_are_not_asked_again() {
    let jev = MockServer::start(answers(0.92, 4));
    let (director, sink, _) = start(&jev, thumbs(), quiet_policy());
    director.hint(facts("right hand raised, thumbs up"), false, 1_234);
    assert!(wait_until(|| !sink.verdicts().is_empty()));
    let (verdicts, at) = sink.verdicts()[0].clone();
    assert_eq!(at, 1_234);
    assert!(verdicts[0].holds && verdicts[0].condition == "thumbs-up");
    director.hint(facts("right hand raised, thumbs up"), false, 1_300);
    thread::sleep(Duration::from_millis(700));
    assert_eq!(jev.requests().len(), 1, "the same facts were asked again");
    director.hint(facts("no hands in view"), false, 2_000);
    assert!(wait_until(|| jev.requests().len() == 2));
    assert!(wait_until(|| sink
        .last_status()
        .is_some_and(|s| s.quick_calls == 2 && s.quick_state == "watching")));
    director.shutdown();
}

#[test]
fn unsure_answers_ask_the_vision_model_for_a_closer_look() {
    let jev = MockServer::start(answers(0.5, 4));
    let (director, _, vision) = start(&jev, thumbs(), quiet_policy());
    director.offer_frame(gray_frame(32, 18, 100, 0));
    assert!(wait_until(|| vision.calls() == 1), "the first look");
    thread::sleep(Duration::from_millis(1_100));
    director.hint(facts("a hand near the face"), false, 10);
    assert!(wait_until(|| jev.requests().len() == 1));
    director.offer_frame(gray_frame(32, 18, 100, 20));
    assert!(
        wait_until(|| vision.calls() == 2),
        "no closer look after an unsure answer"
    );
    director.shutdown();
}

#[test]
fn a_condition_that_reports_a_name_gets_a_closer_look_when_it_turns_on() {
    let jev = MockServer::start(answers(0.95, 4));
    let (director, _, vision) = start(&jev, pet_condition(), quiet_policy());
    director.offer_frame(gray_frame(32, 18, 100, 0));
    assert!(wait_until(|| vision.calls() == 1));
    thread::sleep(Duration::from_millis(1_100));
    director.hint(facts("cat (0.93)"), false, 10);
    assert!(wait_until(|| jev.requests().len() == 1));
    director.offer_frame(gray_frame(32, 18, 100, 20));
    assert!(
        wait_until(|| vision.calls() == 2),
        "the vision model should name the animal"
    );
    director.shutdown();
}

#[test]
fn a_confident_answer_needs_no_closer_look() {
    let jev = MockServer::start(answers(0.95, 4));
    let (director, _, vision) = start(&jev, thumbs(), quiet_policy());
    director.offer_frame(gray_frame(32, 18, 100, 0));
    assert!(wait_until(|| vision.calls() == 1));
    thread::sleep(Duration::from_millis(1_100));
    director.hint(facts("thumbs up"), false, 10);
    assert!(wait_until(|| jev.requests().len() == 1));
    director.offer_frame(gray_frame(32, 18, 100, 20));
    thread::sleep(Duration::from_millis(500));
    assert_eq!(vision.calls(), 1);
    director.shutdown();
}

#[test]
fn without_a_key_quick_judgments_stay_off_and_nothing_is_sent() {
    let jev = MockServer::start(answers(0.9, 1));
    let vision = vision_mock();
    let sink = Arc::new(RecordingSink::default());
    let director = Director::spawn(enabled_settings(), connector(&vision), Arc::clone(&sink) as _);
    director.set_project(thumbs(), quiet_policy(), false);
    director.hint(facts("thumbs up"), false, 10);
    assert!(wait_until(|| sink.last_status().is_some_and(|s| s.quick_state
        == "off"
        && s.quick_message.contains("Jev key"))));
    assert!(jev.requests().is_empty());
    director.shutdown();
}

#[test]
fn a_rejected_key_stops_quick_judgments() {
    let jev = MockServer::start(vec![Reply::json(401, json!({"error": "Missing or invalid API key"}))]);
    let (director, sink, _) = start(&jev, thumbs(), quiet_policy());
    director.hint(facts("thumbs up"), false, 10);
    assert!(wait_until(|| sink
        .last_status()
        .is_some_and(|s| s.quick_state == "failed")));
    director.hint(facts("thumbs down"), false, 20);
    thread::sleep(Duration::from_millis(700));
    assert_eq!(jev.requests().len(), 1);
    director.shutdown();
}

#[test]
fn quick_spend_counts_toward_the_budget_and_a_spent_budget_stops_it() {
    let jev = MockServer::start(answers(0.9, 2));
    let tiny = AiPolicy {
        max_cost_per_hour: 0.000_001,
        ..quiet_policy()
    };
    let (director, sink, _) = start(&jev, thumbs(), tiny);
    director.hint(facts("thumbs up"), false, 10);
    assert!(wait_until(|| sink.last_status().is_some_and(|s| s.state == "paused")));
    assert!(sink.last_status().unwrap().hour_cost_usd > 0.0);
    director.hint(facts("thumbs down"), false, 20);
    thread::sleep(Duration::from_millis(700));
    assert_eq!(jev.requests().len(), 1);
    director.shutdown();
}
