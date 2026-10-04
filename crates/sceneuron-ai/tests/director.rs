// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the director thread with a mock provider and a recording sink: frames becoming
//! observations, status transitions, frame throttling, hints, captions, usage from elsewhere,
//! and a prompt shutdown.

mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use common::director::{cat_answer, eager_policy, pet_condition, start, start_watching};
use common::sink::wait_until;
use common::{enabled_settings, gray_frame};
use sceneuron_ai::{AiSettings, MockProvider, Usage};
use sceneuron_core::AiPolicy;

fn cat_mock() -> Arc<MockProvider> {
    Arc::new(MockProvider::new(|_| Ok(cat_answer())))
}

#[test]
fn frames_become_observations_stamped_with_their_capture_time() {
    let mock = cat_mock();
    let (director, sink) = start_watching(&mock);
    director.offer_frame(gray_frame(1280, 720, 80, 12_345));
    assert!(wait_until(|| !sink.observations().is_empty()));
    let (observation, captured_ms) = sink.observations().remove(0);
    assert_eq!(captured_ms, 12_345);
    assert_eq!(observation.caption, "A cat sits on a desk.");
    assert_eq!(
        (observation.verdicts[0].holds, observation.verdicts[0].detail.as_str()),
        (true, "Cat")
    );
    let requests = mock.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(&requests[0].images[0][..2], [0xFF, 0xD8]);
    assert!(requests[0].text.contains("\"pet\""));
    assert!(sink.resets().is_empty());
    director.shutdown();
}

#[test]
fn status_moves_from_off_to_setup_to_watching_and_thinking() {
    let mock = cat_mock();
    let (director, sink) = start(AiSettings::default(), &mock);
    let off = sink.wait_for_state("off");
    assert_eq!(off.message, "scene understanding is off in the AI settings");
    director.set_project(pet_condition(), eager_policy(), false);
    let no_key = AiSettings {
        enabled: true,
        consent: true,
        ..AiSettings::default()
    };
    director.set_settings(no_key);
    let setup = sink.wait_for_state("needs_setup");
    assert!(setup.message.contains("API key"), "{}", setup.message);
    director.set_settings(enabled_settings());
    sink.wait_for_state("watching");
    director.offer_frame(gray_frame(64, 36, 80, 1));
    assert!(wait_until(|| sink.observations().len() == 1));
    let watching = sink.wait_for_state("watching");
    assert_eq!(
        sink.states(),
        ["off", "needs_setup", "watching", "thinking", "watching"]
    );
    let thinking = sink
        .statuses()
        .into_iter()
        .find(|status| status.state == "thinking")
        .unwrap();
    assert_eq!(thinking.message, "looking at the camera (first look)");
    assert_eq!(
        (watching.provider.as_str(), watching.model.as_str()),
        ("mock", "gemini-3.5-flash-lite")
    );
    assert_eq!(watching.calls, 1);
    assert_eq!(watching.last_latency_ms, Some(0.0));
    assert!(sink.resets().is_empty(), "never ran before, so nothing to reset");
    director.shutdown();
}

#[test]
fn consent_is_needed_before_anything_runs() {
    let mock = cat_mock();
    let without_consent = AiSettings {
        consent: false,
        ..enabled_settings()
    };
    let (director, sink) = start(without_consent, &mock);
    director.set_project(pet_condition(), eager_policy(), false);
    let status = sink.wait_for_state("needs_setup");
    assert!(status.message.contains("camera images"), "{}", status.message);
    assert!(!director.wants_frame(0));
    director.offer_frame(gray_frame(64, 36, 80, 1));
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(mock.calls(), 0);
    director.shutdown();
}

#[test]
fn frames_are_wanted_only_while_running_and_at_most_once_a_second() {
    let mock = cat_mock();
    let (director, sink) = start(AiSettings::default(), &mock);
    sink.wait_for_state("off");
    assert!(!director.wants_frame(0));
    director.set_settings(enabled_settings());
    director.set_project(pet_condition(), eager_policy(), false);
    sink.wait_for_state("watching");
    assert!(director.wants_frame(0));
    director.offer_frame(gray_frame(64, 36, 80, 5_000));
    assert!(!director.wants_frame(5_500));
    assert!(director.wants_frame(6_000));
    director.shutdown();
}

#[test]
fn the_latest_frame_is_kept_even_while_off() {
    let mock = cat_mock();
    let (director, sink) = start(AiSettings::default(), &mock);
    sink.wait_for_state("off");
    assert_eq!(director.latest_frame(), None);
    let frame = gray_frame(32, 18, 10, 77);
    director.offer_frame(frame.clone());
    assert_eq!(director.latest_frame(), Some(frame));
    assert_eq!(mock.calls(), 0);
    director.shutdown();
}

#[test]
fn local_hints_reach_the_model() {
    let mock = cat_mock();
    let (director, sink) = start_watching(&mock);
    director.hint(vec!["two faces in view".to_owned()], false, 0);
    director.offer_frame(gray_frame(64, 36, 80, 1));
    assert!(wait_until(|| sink.observations().len() == 1));
    assert!(mock.requests()[0].text.contains("- two faces in view"));
    director.shutdown();
}

#[test]
fn a_caption_alone_keeps_scene_understanding_running() {
    let mock = Arc::new(MockProvider::new(|_| {
        Ok(serde_json::json!({"caption": "A desk.", "verdicts": []}))
    }));
    let (director, sink) = start(enabled_settings(), &mock);
    director.set_project(Vec::new(), eager_policy(), false);
    let idle = sink.wait_for_state("off");
    assert!(idle.message.contains("add a condition"), "{}", idle.message);
    director.set_project(Vec::new(), eager_policy(), true);
    sink.wait_for_state("watching");
    director.offer_frame(gray_frame(64, 36, 80, 9));
    assert!(wait_until(|| sink.observations().len() == 1));
    assert_eq!(sink.observations()[0].0.caption, "A desk.");
    assert!(mock.requests()[0].text.contains("no conditions to judge"));
    director.shutdown();
}

#[test]
fn usage_recorded_elsewhere_counts_toward_totals_and_the_budget() {
    let mock = cat_mock();
    let (director, sink) = start_watching(&mock);
    let usage = Usage {
        input_tokens: 1_000,
        output_tokens: 500,
        cache_read_tokens: 200,
    };
    director.record_usage("gemini-3.8-flash", usage, 2.5);
    assert!(wait_until(|| sink
        .last_status()
        .is_some_and(|status| status.calls == 1)));
    let status = sink.last_status().unwrap();
    assert_eq!((status.input_tokens, status.output_tokens), (1_200, 500));
    assert_eq!((status.cost_usd, status.hour_cost_usd), (2.5, 2.5));
    // The pause shows at once, before any frame arrives.
    let paused = sink.wait_for_state("paused");
    assert!(
        paused.message.starts_with("hourly budget of $2.00 reached"),
        "{}",
        paused.message
    );
    director.offer_frame(gray_frame(64, 36, 80, 1));
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(sink.last_status().unwrap().state, "paused");
    assert_eq!(mock.calls(), 0);
    director.shutdown();
}

#[test]
fn unknown_prices_are_called_out_in_the_status() {
    let mock = cat_mock();
    let local = AiSettings {
        vision_model: "llava-local".to_owned(),
        ..enabled_settings()
    };
    let (director, sink) = start(local, &mock);
    director.set_project(pet_condition(), AiPolicy::default(), false);
    let status = sink.wait_for_state("watching");
    assert!(
        status.message.contains("no known price for llava-local"),
        "{}",
        status.message
    );
    director.shutdown();
}

#[test]
fn shutdown_returns_promptly_when_idle() {
    let mock = cat_mock();
    let (director, sink) = start_watching(&mock);
    assert!(!sink.statuses().is_empty());
    let started = Instant::now();
    director.shutdown();
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn hosts_can_share_the_director_and_assistant_across_threads() {
    fn shareable<T: Send + Sync>() {}
    shareable::<sceneuron_ai::Director>();
    shareable::<sceneuron_ai::Assistant>();
    shareable::<MockProvider>();
}
