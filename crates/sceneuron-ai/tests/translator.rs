// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the subtitle translator thread behind an assistant, with a scripted provider: each
//! segment translated once and reported by id, its spend in the AI status, nothing sent while
//! subtitles are off or the budget is spent, failures given in Sceneuron's own words, lines
//! skipped when the provider falls behind, and a line queued before its targets changed sent only
//! for the targets still set, or not at all.

mod common;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use common::director::connector;
use common::sink::{RecordingSink, wait_until};
use common::{TEST_KEY, temp_dir};
use sceneuron_ai::{AiError, AiSettingsUpdate, Assistant, Credentials, MockProvider, ModelRequest, Usage};
use sceneuron_core::{AiPolicy, TranslationJob};
use serde_json::{Value, json};

const CLOSED: &str = "http://127.0.0.1:9";

/// An assistant with `update` applied on Gemini, whose Gemini 3.5 Flash-Lite prices and key
/// message these tests count.
fn open(mock: &Arc<MockProvider>, update: Value) -> (Assistant, Arc<RecordingSink>) {
    let sink = Arc::new(RecordingSink::default());
    let path = temp_dir("translator").join("ai.json");
    let (assistant, warning) = Assistant::with_elevenlabs_url(
        path,
        connector(mock),
        Arc::clone(&sink) as _,
        Credentials::default(),
        CLOSED,
    );
    assert_eq!(warning, None);
    let update: AiSettingsUpdate = serde_json::from_value(update).unwrap();
    assistant.update_settings(update).unwrap();
    (assistant, sink)
}

fn on() -> Value {
    json!({"api_key": TEST_KEY, "subtitles": {"consent": true, "enabled": true, "targets": ["es"]}})
}

fn job(id: u64, text: &str) -> TranslationJob {
    TranslationJob {
        id,
        text: text.to_owned(),
        language: "en".to_owned(),
        targets: vec!["es".to_owned()],
        context: Vec::new(),
    }
}

fn spanish(text: &str) -> Value {
    json!({"translations": {"es": format!("ES {text}")}})
}

/// Answers each line with "ES " and its words, and reports 400 input and 60 output tokens.
fn translating() -> Arc<MockProvider> {
    let usage = Usage {
        input_tokens: 400,
        output_tokens: 60,
        ..Usage::default()
    };
    let respond = |request: &ModelRequest| {
        let input: Value = serde_json::from_str(&request.text).unwrap();
        Ok(spanish(input["line"].as_str().unwrap()))
    };
    Arc::new(MockProvider::new(respond).with_usage(usage))
}

fn es(text: &str) -> Result<BTreeMap<String, String>, String> {
    Ok(BTreeMap::from([("es".to_owned(), text.to_owned())]))
}

#[test]
fn each_segment_is_translated_once_and_its_spend_counts() {
    let mock = translating();
    let (assistant, sink) = open(&mock, on());
    assistant.translate(job(7, "Good morning."));
    assert!(wait_until(|| !sink.translations().is_empty()));
    assert_eq!(sink.translations(), [(7, es("ES Good morning."))]);
    let sent = mock.requests();
    assert_eq!((sent.len(), sent[0].model.as_str()), (1, "gemini-3.5-flash-lite"));
    let counted = || sink.last_status().is_some_and(|status| status.translation_calls == 1);
    assert!(wait_until(counted), "{:?}", sink.last_status());
    let status = sink.last_status().unwrap();
    assert_eq!(
        (status.translation_input_tokens, status.translation_output_tokens),
        (400, 60)
    );
    let expected = (400.0 * 0.30 + 60.0 * 2.50) / 1_000_000.0;
    assert!((status.translation_cost_usd - expected).abs() < 1e-12, "{status:?}");
    assert!((status.cost_usd - expected).abs() < 1e-12 && (status.hour_cost_usd - expected).abs() < 1e-12);
    assert_eq!(status.calls, 0, "translations are not camera looks");
    assistant.shutdown();
}

#[test]
fn nothing_is_sent_while_subtitles_are_off_or_the_budget_is_spent() {
    let mock = translating();
    let (assistant, sink) = open(&mock, json!({"api_key": TEST_KEY}));
    assistant.translate(job(1, "Off."));
    assert!(wait_until(|| !sink.translations().is_empty()));
    assert_eq!(sink.translations(), [(1, Err("subtitles are off".to_owned()))]);
    let spent = AiPolicy {
        max_cost_per_hour: 0.0,
        ..AiPolicy::default()
    };
    assistant.director().set_project(Vec::new(), spent, false);
    assistant
        .update_settings(serde_json::from_value(on()).unwrap())
        .unwrap();
    assistant.translate(job(2, "This one costs."));
    assert!(
        wait_until(|| assistant.director().over_budget()),
        "the spend reaches the budget"
    );
    assistant.translate(job(3, "Not sent."));
    assert!(wait_until(|| sink.translations().len() == 3));
    let last = sink.translations().pop().unwrap();
    assert_eq!(last, (3, Err("the hourly budget is spent".to_owned())));
    assert_eq!(mock.calls(), 1, "only the line before the budget was spent went out");
    assistant.shutdown();
}

#[test]
fn failures_are_told_in_sceneurons_own_words() {
    let quoting = Arc::new(MockProvider::new(|request: &ModelRequest| {
        Err(AiError::BadRequest(format!("cannot translate {}", request.text)))
    }));
    let (assistant, sink) = open(&quoting, on());
    assistant.translate(job(1, "a secret plan"));
    assert!(wait_until(|| !sink.translations().is_empty()));
    let why = sink.translations()[0].1.clone().unwrap_err();
    assert_eq!(why, "the AI provider rejected the request; check the translation model");
    assistant.shutdown();

    let unusable = Arc::new(MockProvider::new(|_: &ModelRequest| Ok(json!({"translations": {}}))));
    let (assistant, sink) = open(&unusable, on());
    assistant.translate(job(2, "another line"));
    assert!(wait_until(|| !sink.translations().is_empty()));
    assert_eq!(
        sink.translations(),
        [(2, Err("the model's answer could not be used".to_owned()))]
    );
    assistant.shutdown();

    let no_key = json!({"subtitles": {"consent": true, "enabled": true, "targets": ["es"]}});
    let (assistant, sink) = open(&translating(), no_key);
    assistant.translate(job(3, "no key"));
    assert!(wait_until(|| !sink.translations().is_empty()));
    assert_eq!(
        sink.translations(),
        [(3, Err("add a Gemini API key in the AI settings".to_owned()))]
    );
    assistant.shutdown();
}

#[test]
fn a_translator_that_falls_behind_skips_the_oldest_waiting_lines() {
    let slow = Arc::new(MockProvider::new(|request: &ModelRequest| {
        let input: Value = serde_json::from_str(&request.text).unwrap();
        let line = input["line"].as_str().unwrap().to_owned();
        if line == "first" {
            thread::sleep(Duration::from_millis(300));
        }
        Ok(spanish(&line))
    }));
    let (assistant, sink) = open(&slow, on());
    assistant.translate(job(1, "first"));
    assert!(wait_until(|| slow.calls() == 1), "the first line is in flight");
    for id in 2..=6 {
        assistant.translate(job(id, &format!("line {id}")));
    }
    assert!(wait_until(|| sink.translations().len() == 6));
    let skipped = Err("translations fell behind, so a line was skipped".to_owned());
    let expected = vec![
        (1, es("ES first")),
        (2, skipped),
        (3, es("ES line 3")),
        (4, es("ES line 4")),
        (5, es("ES line 5")),
        (6, es("ES line 6")),
    ];
    assert_eq!(sink.translations(), expected);
    assistant.shutdown();
}

#[test]
fn a_line_queued_before_its_targets_were_removed_is_not_sent() {
    let mock = translating();
    let (assistant, sink) = open(&mock, on());
    let none: AiSettingsUpdate = serde_json::from_value(json!({"subtitles": {"targets": []}})).unwrap();
    assistant.update_settings(none).unwrap();
    assistant.translate(job(1, "Said before the change."));
    assert!(wait_until(|| !sink.translations().is_empty()));
    assert_eq!(
        sink.translations(),
        [(1, Err("no languages to translate into".to_owned()))]
    );
    assert_eq!(mock.calls(), 0, "no text leaves without a language to translate into");
    assistant.shutdown();
}

#[test]
fn a_line_goes_out_only_for_the_targets_still_wanted() {
    let every_target = Arc::new(MockProvider::new(|request: &ModelRequest| {
        let input: Value = serde_json::from_str(&request.text).unwrap();
        let line = input["line"].as_str().unwrap();
        let codes = input["targets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|target| &target["code"]);
        let answers: serde_json::Map<String, Value> = codes
            .map(|code| (code.as_str().unwrap().to_owned(), json!(format!("{code} {line}"))))
            .collect();
        Ok(json!({ "translations": answers }))
    }));
    let both = json!({"api_key": TEST_KEY, "subtitles": {"consent": true, "enabled": true, "targets": ["es", "fr"]}});
    let (assistant, sink) = open(&every_target, both);
    let only_french: AiSettingsUpdate = serde_json::from_value(json!({"subtitles": {"targets": ["fr"]}})).unwrap();
    assistant.update_settings(only_french).unwrap();
    let queued = TranslationJob {
        targets: vec!["ES".to_owned(), "FR".to_owned()],
        ..job(2, "Bonjour.")
    };
    assistant.translate(queued);
    assert!(wait_until(|| !sink.translations().is_empty()));
    let sent: Value = serde_json::from_str(&every_target.requests()[0].text).unwrap();
    assert_eq!(
        sent["targets"],
        json!([{"code": "FR", "language": "French", "constructed": false}]),
        "matched without regard to case"
    );
    let french = BTreeMap::from([("FR".to_owned(), "\"FR\" Bonjour.".to_owned())]);
    assert_eq!(sink.translations(), [(2, Ok(french))]);
    assistant.shutdown();
}
