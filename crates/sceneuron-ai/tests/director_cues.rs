// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises cued conditions in the director, with a scripted vision model and Jev: the tongue-out
//! reaction waits for an open mouth, so a closed mouth costs no frame, look, or question; an open
//! mouth held for the dwell time brings one look at a frame taken from then on; a short opening
//! does not; the reaction turns off when the mouth closes after the model saw it; and Jev judges
//! only the conditions without a cue.

mod common;

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use common::server::{MockServer, Reply};
use common::sink::{RecordingSink, wait_until};
use common::{TEST_KEY, condition, gray_frame};
use sceneuron_ai::{AiSettings, Director, MockProvider, ModelRequest};
use sceneuron_core::{AiPolicy, Condition, TONGUE_OUT, Verdict, reactions};
use serde_json::{Value, json};

/// A model that sees a tongue whenever it is asked about one.
fn vision() -> Arc<MockProvider> {
    Arc::new(MockProvider::new(|request: &ModelRequest| {
        let verdicts = if request.text.contains(TONGUE_OUT) {
            json!([{"condition": TONGUE_OUT, "reason": "The tongue sticks out.", "holds": true,
                    "confidence": 0.95, "detail": ""}])
        } else {
            json!([])
        };
        Ok(json!({"caption": "", "verdicts": verdicts}))
    }))
}

/// Only reasons start a look: no refresh and no look for a changed picture.
fn policy() -> AiPolicy {
    AiPolicy {
        min_interval_ms: 0,
        refresh_ms: 3_600_000,
        change_threshold: 1.0,
        ..AiPolicy::default()
    }
}

fn jev_says(probability: f64) -> MockServer {
    let reply = Reply::json(
        200,
        json!({"model": "jev-1.13.0", "answers": {"q0": {"type": "noul", "noul": probability}},
               "usage": {"input_tokens": 100, "output_tokens": 0}}),
    );
    MockServer::start(vec![reply; 40])
}

/// A director watching `conditions` and the tongue-out reaction, with a Jev key.
fn start(conditions: Vec<Condition>, jev: &MockServer) -> (Director, Arc<RecordingSink>, Arc<MockProvider>) {
    let settings = AiSettings {
        enabled: true,
        consent: true,
        api_key: TEST_KEY.to_owned(),
        jev_api_key: "jev-test-key-0123456789".to_owned(),
        jev_base_url: jev.url(),
        ..AiSettings::default()
    };
    let vision = vision();
    let sink = Arc::new(RecordingSink::default());
    let director = Director::spawn(settings, vision.connector(), Arc::clone(&sink) as _);
    let reaction = &reactions()[0];
    let mut watched = conditions;
    watched.push(reaction.condition.clone());
    director.set_project_with_cues(watched, vec![reaction.cue()], policy(), false);
    (director, sink, vision)
}

fn facts(mouth: &str) -> Vec<String> {
    vec!["face in view (0.98)".to_owned(), mouth.to_owned()]
}

/// Reports `mouth` at each of `times`, waiting a little between reports like a camera does.
fn show(director: &Director, mouth: &str, times: &[u64]) {
    for &at in times {
        director.hint(facts(mouth), false, at);
        thread::sleep(Duration::from_millis(30));
    }
}

fn quiet_for_a_while() {
    thread::sleep(Duration::from_millis(400));
}

fn tongue_verdicts(sink: &RecordingSink) -> Vec<(Verdict, u64)> {
    let all = sink.verdicts().into_iter();
    let flat = all.flat_map(|(verdicts, at)| verdicts.into_iter().map(move |verdict| (verdict, at)));
    flat.filter(|(verdict, _)| verdict.condition == TONGUE_OUT).collect()
}

#[test]
fn a_closed_mouth_costs_no_frame_look_or_question() {
    let jev = jev_says(0.1);
    let (director, sink, vision) = start(Vec::new(), &jev);
    let status = sink.wait_for_state("watching");
    assert!(
        status
            .message
            .contains("waiting for on-device analysis to see mouth open"),
        "{status:?}"
    );
    show(&director, "mouth closed (0.01)", &[1_000, 1_200, 1_400, 1_600]);
    assert!(!director.wants_frame(1_700), "a frame was wanted with nothing to ask");
    director.offer_frame(gray_frame(32, 18, 90, 1_700));
    quiet_for_a_while();
    assert_eq!(vision.calls(), 0);
    assert!(jev.requests().is_empty(), "Jev was asked about a cued condition");
    director.shutdown();
}

#[test]
fn an_open_mouth_held_brings_one_look_at_a_frame_taken_from_then_on() {
    let jev = jev_says(0.1);
    let (director, sink, vision) = start(Vec::new(), &jev);
    sink.wait_for_state("watching");
    show(&director, "mouth open (0.21)", &[1_000, 1_200]);
    assert!(!director.wants_frame(1_250), "armed before the dwell time");
    show(&director, "mouth open (0.24)", &[1_400]);
    assert!(wait_until(|| director.wants_frame(1_410)), "no frame wanted once armed");
    director.offer_frame(gray_frame(32, 18, 90, 1_300));
    quiet_for_a_while();
    assert_eq!(vision.calls(), 0, "a frame from before the cue was judged");
    director.offer_frame(gray_frame(32, 18, 90, 1_450));
    assert!(wait_until(|| sink.observations().len() == 1));
    let (observation, captured_ms) = sink.observations()[0].clone();
    assert_eq!(captured_ms, 1_450);
    assert!(observation.verdicts[0].holds);
    let request = &vision.requests()[0];
    assert!(request.text.contains("Someone in view is sticking their tongue out"));
    assert!(request.text.contains("mouth open (0.24)"), "the hint reaches the model");
    quiet_for_a_while();
    assert_eq!(vision.calls(), 1);
    director.shutdown();
}

#[test]
fn a_short_opening_brings_no_look() {
    let jev = jev_says(0.1);
    let (director, sink, vision) = start(Vec::new(), &jev);
    sink.wait_for_state("watching");
    show(&director, "mouth open (0.12)", &[1_000, 1_200]);
    show(&director, "mouth closed (0.03)", &[1_300]);
    show(&director, "mouth open (0.10)", &[1_500, 1_700]);
    director.offer_frame(gray_frame(32, 18, 90, 1_750));
    quiet_for_a_while();
    assert_eq!(vision.calls(), 0);
    director.shutdown();
}

#[test]
fn closing_the_mouth_turns_the_reaction_off_once_the_model_saw_it() {
    let jev = jev_says(0.1);
    let (director, sink, vision) = start(Vec::new(), &jev);
    sink.wait_for_state("watching");
    show(&director, "mouth closed (0.02)", &[500]);
    assert!(tongue_verdicts(&sink).is_empty(), "nothing to turn off yet");
    show(&director, "mouth open (0.3)", &[1_000, 1_400]);
    director.offer_frame(gray_frame(32, 18, 90, 1_450));
    assert!(wait_until(|| vision.calls() == 1));
    assert!(wait_until(|| sink.observations().len() == 1));
    show(&director, "mouth closed (0.02)", &[2_000]);
    assert!(wait_until(|| !tongue_verdicts(&sink).is_empty()));
    let (verdict, at) = tongue_verdicts(&sink)[0].clone();
    assert_eq!(at, 2_000);
    assert!(!verdict.holds && verdict.confidence == 1.0, "{verdict:?}");
    assert_eq!(verdict.reason, "on-device analysis no longer sees mouth open");
    show(&director, "mouth open (0.3)", &[2_500]);
    show(&director, "mouth closed (0.02)", &[2_600]);
    quiet_for_a_while();
    assert_eq!(
        tongue_verdicts(&sink).len(),
        1,
        "closing again without a look turns nothing off"
    );
    director.shutdown();
}

#[test]
fn jev_judges_only_the_conditions_without_a_cue() {
    let jev = jev_says(0.92);
    let thumbs = condition("thumbs-up", "The person gives a thumbs up.", "");
    let (director, sink, _vision) = start(vec![thumbs], &jev);
    show(&director, "mouth open (0.3)", &[1_000, 1_200, 1_400]);
    assert!(wait_until(|| !jev.requests().is_empty()));
    for request in jev.requests() {
        let questions: Value = request.json()["questions"].clone();
        let asked: Vec<&str> = questions
            .as_object()
            .unwrap()
            .values()
            .filter_map(|question| question["instructions"].as_str())
            .collect();
        assert_eq!(asked, ["Is this true right now? The person gives a thumbs up."]);
    }
    assert!(wait_until(|| sink.verdicts().iter().any(|(verdicts, _)| verdicts
        .iter()
        .any(|v| v.condition == "thumbs-up" && v.holds))));
    assert!(
        tongue_verdicts(&sink).iter().all(|(verdict, _)| !verdict.holds),
        "Jev turned the reaction on"
    );
    director.shutdown();
}

#[test]
fn other_conditions_are_judged_without_the_reaction_until_the_mouth_opens() {
    let jev = jev_says(0.1);
    let pet = condition("pet", "A cat or dog is clearly visible.", "");
    let (director, sink, vision) = start(vec![pet], &jev);
    sink.wait_for_state("watching");
    show(&director, "mouth closed (0.02)", &[900]);
    director.offer_frame(gray_frame(32, 18, 90, 950));
    assert!(wait_until(|| vision.calls() == 1), "the first look");
    let first = &vision.requests()[0];
    assert!(first.text.contains("A cat or dog") && !first.text.contains(TONGUE_OUT));
    show(&director, "mouth open (0.3)", &[1_000, 1_400]);
    director.offer_frame(gray_frame(32, 18, 90, 1_450));
    assert!(wait_until(|| vision.calls() == 2), "no look when the mouth opened");
    let second = &vision.requests()[1];
    assert!(second.text.contains("A cat or dog") && second.text.contains(TONGUE_OUT));
    director.shutdown();
}
