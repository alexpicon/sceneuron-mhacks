// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises words for the AI against a scripted planner: they need a key and an hourly budget not
//! yet spent, come back as actions with the look and subtitle changes done here, retry once when a
//! project edit does not apply, say why a model refused, and count toward the spend.
//! assistant_host_actions.rs covers what the assistant does itself.

mod common;

use std::sync::Arc;

use common::assistant::{answering, festive, open, with_key};
use common::sink::wait_until;
use common::temp_dir;
use sceneuron_ai::{AiError, MockProvider, ModelRequest};
use sceneuron_core::{AiPolicy, Flood, VoiceAction, VoiceReply, VoiceStep, VoiceTask, default_project};
use serde_json::json;

fn ask(text: &str) -> VoiceTask {
    VoiceTask::Ask { text: text.to_owned() }
}

#[test]
fn words_for_the_ai_need_a_key_and_come_back_as_actions() {
    let mock = answering(vec![festive()]);
    let (assistant, _) = open(&temp_dir("voice-ask").join("ai.json"), &mock);
    let project = default_project();
    let refused = assistant.carry_out(&project, &ask("Sceneuron, celebrate")).unwrap_err();
    assert_eq!(
        refused,
        "no command matched, and the AI that takes any other is not set up: add a Gemini API key in the AI settings"
    );
    assert_eq!(mock.calls(), 0);
    assistant.update_settings(with_key()).unwrap();
    let reply = assistant.carry_out(&project, &ask("Sceneuron, celebrate")).unwrap();
    let VoiceReply { understood, steps } = reply;
    assert_eq!(understood, "celebrate in Spanish");
    assert_eq!(steps.len(), 3);
    assert_eq!(
        steps[0],
        VoiceStep::Act(VoiceAction::React {
            reaction: Flood::ShipIt
        })
    );
    assert_eq!(
        steps[1],
        VoiceStep::Done(
            "subtitles will translate into Spanish (es) first; to show them, confirm in the AI tab that microphone \
             audio may be sent to ElevenLabs for subtitles"
                .to_owned()
        ),
        "done here"
    );
    let VoiceStep::Act(VoiceAction::EditProject { summary, .. }) = &steps[2] else {
        panic!("a project edit for the engine, got {:?}", steps[2]);
    };
    assert_eq!(summary, "Hides the name label.");
    let request: ModelRequest = mock.requests().remove(0);
    assert_eq!(
        (request.model.as_str(), request.effort.as_str()),
        ("gemini-3.1-pro-preview", "low")
    );
    assert!(request.text.contains("<words>\nSceneuron, celebrate\n</words>"));
    assistant.shutdown();
}

#[test]
fn an_edit_that_does_not_apply_is_sent_back_once() {
    let broken = json!({"understood": "remove the ghost", "actions": [
        {"action": "edit_project", "summary": "Removes it.", "operations": [{"op": "remove_element", "id": "ghost"}]}
    ]});
    let mock = answering(vec![broken.clone(), festive()]);
    let (assistant, _) = open(&temp_dir("voice-retry").join("ai.json"), &mock);
    assistant.update_settings(with_key()).unwrap();
    let reply = assistant
        .carry_out(&default_project(), &ask("remove the ghost"))
        .unwrap();
    assert_eq!(reply.understood, "celebrate in Spanish");
    let requests = mock.requests();
    assert_eq!(requests.len(), 2);
    let reason = "Reason: operation 1: unknown element \"ghost\"";
    assert!(requests[1].text.contains(reason), "{}", requests[1].text);
    let stubborn = answering(vec![broken]);
    let (again, _) = open(&temp_dir("voice-retry-again").join("ai.json"), &stubborn);
    again.update_settings(with_key()).unwrap();
    assert_eq!(
        again
            .carry_out(&default_project(), &ask("remove the ghost"))
            .unwrap_err(),
        "the AI's answer could not be used: operation 1: unknown element \"ghost\""
    );
    assert_eq!(stubborn.calls(), 2);
    assistant.shutdown();
    again.shutdown();
}

#[test]
fn refusals_and_a_spent_budget_say_why_and_calls_count_toward_the_spend() {
    let declined = Arc::new(MockProvider::new(|_: &ModelRequest| {
        Err(AiError::Refused("that is not something I can do".to_owned()))
    }));
    let (assistant, sink) = open(&temp_dir("voice-refused").join("ai.json"), &declined);
    assistant.update_settings(with_key()).unwrap();
    assert_eq!(
        assistant
            .carry_out(&default_project(), &ask("hack the planet"))
            .unwrap_err(),
        "the model declined: that is not something I can do"
    );
    assistant.shutdown();
    let mock = answering(vec![festive()]);
    let (assistant, sink_spent) = open(&temp_dir("voice-budget").join("ai.json"), &mock);
    assistant.update_settings(with_key()).unwrap();
    let policy = AiPolicy {
        max_cost_per_hour: 0.0,
        ..AiPolicy::default()
    };
    assistant.director().set_project(Vec::new(), policy, false);
    assistant.carry_out(&default_project(), &ask("celebrate")).unwrap();
    assert!(wait_until(|| sink_spent
        .last_status()
        .is_some_and(|status| status.calls == 1)));
    assert!(wait_until(|| assistant.director().over_budget()));
    assert_eq!(
        assistant.carry_out(&default_project(), &ask("celebrate")).unwrap_err(),
        "the hourly budget is spent, so the AI takes no voice commands until it allows more"
    );
    assert_eq!(mock.calls(), 1);
    assert!(
        sink.statuses().iter().all(|status| status.calls == 0),
        "a refusal costs nothing"
    );
    assistant.shutdown();
}
