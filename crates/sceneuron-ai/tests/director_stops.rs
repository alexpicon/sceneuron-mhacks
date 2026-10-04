// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises how the director stops and slows down: the hourly budget, permanent failures that
//! stop calls until the settings change, refusals, transient failures with backoff, turning
//! scene understanding off, and answers that arrive after the question changed.

mod common;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use common::director::{cat_answer, eager_policy, pet_condition, start, start_watching};
use common::sink::wait_until;
use common::{enabled_settings, gray_frame};
use sceneuron_ai::{AiError, AiSettings, MockProvider, Usage};
use sceneuron_core::AiPolicy;

#[test]
fn the_hourly_budget_pauses_calls() {
    // A million input tokens at Gemini 3.1 Pro's $2 per million is the whole $2 budget.
    let usage = Usage {
        input_tokens: 1_000_000,
        ..Usage::default()
    };
    let mock = Arc::new(MockProvider::new(|_| Ok(cat_answer())).with_usage(usage));
    let pro = AiSettings {
        vision_model: "gemini-3.1-pro-preview".to_owned(),
        ..enabled_settings()
    };
    let (director, sink) = start(pro, &mock);
    director.set_project(pet_condition(), eager_policy(), false);
    sink.wait_for_state("watching");
    director.offer_frame(gray_frame(64, 36, 80, 1));
    assert!(wait_until(|| sink.observations().len() == 1));
    director.offer_frame(gray_frame(64, 36, 200, 2));
    let paused = sink.wait_for_state("paused");
    assert_eq!(paused.message, "hourly budget of $2.00 reached; resumes in 60 min");
    assert!((paused.hour_cost_usd - 2.0).abs() < 1e-9 && (paused.cost_usd - 2.0).abs() < 1e-9);
    assert_eq!(paused.input_tokens, 1_000_000);
    assert_eq!(mock.calls(), 1);
    assert!(sink.resets().is_empty(), "a pause is not a stop");
    director.shutdown();
}

#[test]
fn a_permanent_failure_stops_calls_and_resets_until_the_settings_change() {
    let broken = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&broken);
    let mock = Arc::new(MockProvider::new(move |_| {
        if flag.load(Ordering::SeqCst) {
            Err(AiError::Auth(
                "API key not valid. Please pass a valid API key.".to_owned(),
            ))
        } else {
            Ok(cat_answer())
        }
    }));
    let (director, sink) = start_watching(&mock);
    director.offer_frame(gray_frame(64, 36, 80, 1));
    assert!(wait_until(|| sink.observations().len() == 1));
    broken.store(true, Ordering::SeqCst);
    director.offer_frame(gray_frame(64, 36, 80, 2));
    let failed = sink.wait_for_state("failed");
    assert_eq!(
        failed.message,
        "the AI provider rejected the API key: API key not valid. Please pass a valid API key."
    );
    assert_eq!(sink.resets(), std::slice::from_ref(&failed.message));
    assert!(!director.wants_frame(1_000_000));
    director.offer_frame(gray_frame(64, 36, 80, 3));
    thread::sleep(Duration::from_millis(100));
    assert_eq!(mock.calls(), 2);

    broken.store(false, Ordering::SeqCst);
    director.set_settings(AiSettings {
        api_key: "another-test-key-98765".to_owned(),
        ..enabled_settings()
    });
    sink.wait_for_state("watching");
    director.offer_frame(gray_frame(64, 36, 80, 4));
    assert!(wait_until(|| sink.observations().len() >= 2));
    // The first look after recovering may use the recent frame offered while stopped.
    assert!(sink.observations()[1].1 >= 3);
    director.shutdown();
}

#[test]
fn turning_scene_understanding_off_resets_conditions() {
    let mock = Arc::new(MockProvider::new(|_| Ok(cat_answer())));
    let (director, sink) = start_watching(&mock);
    director.offer_frame(gray_frame(64, 36, 80, 1));
    assert!(wait_until(|| sink.observations().len() == 1));
    director.set_settings(AiSettings {
        enabled: false,
        ..enabled_settings()
    });
    sink.wait_for_state("off");
    assert_eq!(sink.resets(), ["scene understanding is off in the AI settings"]);
    assert!(!director.wants_frame(1_000_000));

    director.set_settings(enabled_settings());
    sink.wait_for_state("watching");
    let project_off = AiPolicy {
        enabled: false,
        ..eager_policy()
    };
    director.set_project(pet_condition(), project_off, false);
    sink.wait_for_state("off");
    assert_eq!(sink.resets()[1], "scene understanding is off for this project");
    director.shutdown();
}

#[test]
fn a_refusal_counts_as_a_call_without_failing() {
    let mock = Arc::new(MockProvider::new(|_| Err(AiError::Refused("declined".to_owned()))));
    let (director, sink) = start_watching(&mock);
    director.offer_frame(gray_frame(64, 36, 80, 1));
    assert!(wait_until(|| sink
        .last_status()
        .is_some_and(|status| status.calls == 1)));
    let status = sink.wait_for_state("watching");
    assert_eq!(status.message, "the model declined to judge this image");
    assert!(sink.observations().is_empty() && sink.resets().is_empty());
    director.shutdown();
}

#[test]
fn repeated_transient_failures_report_failed_while_retrying() {
    let mock = Arc::new(MockProvider::new(|_| {
        Err(AiError::Unavailable("overloaded".to_owned()))
    }));
    let (director, sink) = start_watching(&mock);
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut captured_ms = 0;
    while sink.last_status().is_none_or(|status| status.state != "failed") && Instant::now() < deadline {
        captured_ms += 1;
        director.offer_frame(gray_frame(64, 36, 80, captured_ms));
        thread::sleep(Duration::from_millis(50));
    }
    let failed = sink.last_status().unwrap();
    assert_eq!(failed.state, "failed");
    assert_eq!(
        failed.message,
        "retrying in 8 s: the AI provider is unavailable: overloaded"
    );
    assert_eq!(mock.calls(), 3);
    let messages: Vec<String> = sink.statuses().into_iter().map(|status| status.message).collect();
    assert!(
        messages.iter().any(|m| m.starts_with("retrying in 2 s:")),
        "{messages:?}"
    );
    assert!(
        sink.resets().is_empty(),
        "transient failures keep scene understanding running"
    );
    assert!(director.wants_frame(u64::MAX / 2));
    director.shutdown();
}

#[test]
fn an_answer_to_a_question_that_changed_meanwhile_is_dropped() {
    let (release, wait) = mpsc::channel::<()>();
    let wait = Mutex::new(wait);
    let mock = Arc::new(MockProvider::new(move |_| {
        let _ = wait.lock().unwrap().recv_timeout(Duration::from_secs(5));
        Ok(cat_answer())
    }));
    let (director, sink) = start_watching(&mock);
    director.offer_frame(gray_frame(64, 36, 80, 1));
    assert!(wait_until(|| mock.calls() == 1));
    director.set_settings(AiSettings {
        enabled: false,
        ..enabled_settings()
    });
    release.send(()).unwrap();
    sink.wait_for_state("off");
    assert!(sink.observations().is_empty());
    assert_eq!(sink.resets().len(), 1);
    director.shutdown();
}

#[test]
fn a_bug_in_the_director_thread_is_reported_and_resets() {
    let mock = Arc::new(MockProvider::new(|_| panic!("a bug while answering")));
    let (director, sink) = start_watching(&mock);
    director.offer_frame(gray_frame(64, 36, 80, 1));
    let failed = sink.wait_for_state("failed");
    assert!(failed.message.contains("stopped unexpectedly"), "{}", failed.message);
    assert_eq!(sink.resets(), std::slice::from_ref(&failed.message));
    assert!(!director.wants_frame(1_000_000));
    director.shutdown();
}
