// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises how the Gemini `generateContent` client reads answers from a scripted server on
//! 127.0.0.1: the text without thoughts, token usage with cached and thinking tokens, the model
//! version and its price, blocked prompts, finish reasons, and a missing key.

mod common;

use common::gemini::{answer, finished, request, settings, success};
use common::server::{MockServer, Reply};
use sceneuron_ai::{AiError, AiSettings, ModelRequest, ProviderKind, Usage, connect, cost_usd, prices_for};
use serde_json::json;

#[test]
fn answers_skip_thoughts_and_count_cached_and_thinking_tokens() {
    let server = MockServer::start(vec![success()]);
    let reply = connect(&settings(&server)).unwrap().complete(&request()).unwrap();
    assert_eq!(
        reply.json,
        json!({"answer": 42}),
        "the answer's parts are joined in order, without the thought that drafts another"
    );
    assert_eq!(reply.model, "gemini-3.8-flash");
    let usage = reply.usage;
    assert_eq!(
        (usage.input_tokens, usage.cache_read_tokens, usage.output_tokens),
        (300, 1000, 117)
    );
    assert!(reply.latency_ms >= 0.0);
}

#[test]
fn the_reply_names_the_model_version_without_its_prefix_or_else_the_model_asked() {
    let text = json!({"content": {"parts": [{"text": "```json\n{\"answer\": 7}\n```"}]}, "finishReason": "STOP"});
    let versioned = Reply::json(
        200,
        json!({"candidates": [text], "modelVersion": "models/gemini-3.5-flash-lite",
            "usageMetadata": {"promptTokenCount": 5, "cachedContentTokenCount": 9}}),
    );
    let bare = Reply::json(
        200,
        json!({"candidates": [{"content": {"parts": [{"text": "{\"answer\": 8}"}]}}]}),
    );
    let server = MockServer::start(vec![versioned, bare]);
    let provider = connect(&settings(&server)).unwrap();
    let reply = provider.complete(&request()).unwrap();
    assert_eq!(
        (reply.json["answer"].as_i64(), reply.model.as_str()),
        (Some(7), "gemini-3.5-flash-lite")
    );
    assert_eq!(
        (reply.usage.input_tokens, reply.usage.cache_read_tokens),
        (0, 9),
        "a cached count above the prompt count never underflows"
    );
    let reply = provider.complete(&request()).unwrap();
    assert_eq!(
        (reply.json["answer"].as_i64(), reply.model.as_str()),
        (Some(8), "gemini-3.8-flash")
    );
    assert_eq!(reply.usage, Usage::default());
}

#[test]
fn a_blocked_prompt_is_refused_before_any_candidate_is_read() {
    let blocked = Reply::json(
        200,
        json!({"promptFeedback": {"blockReason": "PROHIBITED_CONTENT", "safetyRatings": []},
            "usageMetadata": {"promptTokenCount": 1300}}),
    );
    let server = MockServer::start(vec![blocked]);
    let error = connect(&settings(&server)).unwrap().complete(&request()).unwrap_err();
    assert_eq!(
        error,
        AiError::Refused("the request was blocked (PROHIBITED_CONTENT)".to_owned())
    );
    assert_eq!(server.requests().len(), 1, "a refusal is not retried");
}

#[test]
fn blocked_answers_are_refused_and_cut_or_broken_ones_are_invalid() {
    let cases = [
        (
            finished("SAFETY", Some("The answer was withheld.")),
            AiError::Refused("The answer was withheld.".to_owned()),
        ),
        (
            finished("RECITATION", None),
            AiError::Refused("the answer was blocked (RECITATION)".to_owned()),
        ),
        (
            finished("IMAGE_SAFETY", Some(" ")),
            AiError::Refused("the answer was blocked (IMAGE_SAFETY)".to_owned()),
        ),
        (
            finished("SPII", None),
            AiError::Refused("the answer was blocked (SPII)".to_owned()),
        ),
        (
            finished("MAX_TOKENS", None),
            AiError::Invalid("the answer was cut off at the token limit".to_owned()),
        ),
        (
            finished("MALFORMED_RESPONSE", None),
            AiError::Invalid("the model stopped without an answer (MALFORMED_RESPONSE)".to_owned()),
        ),
        (
            finished("OTHER", None),
            AiError::Invalid("the model stopped without an answer (OTHER)".to_owned()),
        ),
        (
            finished("MISSING_THOUGHT_SIGNATURE", None),
            AiError::Invalid("the model stopped without an answer (MISSING_THOUGHT_SIGNATURE)".to_owned()),
        ),
        (
            Reply::json(200, json!({"candidates": [], "usageMetadata": {}})),
            AiError::Invalid("the provider's response has no answer".to_owned()),
        ),
        (
            answer(json!({"content": {"parts": [{"text": "No cat."}]}, "finishReason": "STOP"})),
            AiError::Invalid("the answer did not contain a JSON object".to_owned()),
        ),
        (
            Reply::bytes(200, "text/plain", b"not json".to_vec()),
            AiError::Invalid("the provider's response is not JSON".to_owned()),
        ),
    ];
    let count = cases.len();
    let (replies, expected): (Vec<Reply>, Vec<AiError>) = cases.into_iter().unzip();
    let server = MockServer::start(replies);
    let provider = connect(&settings(&server)).unwrap();
    for expected in expected {
        assert_eq!(provider.complete(&request()), Err(expected));
    }
    assert_eq!(server.requests().len(), count, "none of these is retried");
}

#[test]
fn an_account_limited_for_policy_violations_is_a_key_problem_that_retrying_cannot_fix() {
    let server = MockServer::start(vec![finished("PUP_LIMITED_DISABLED", None)]);
    let error = connect(&settings(&server)).unwrap().complete(&request()).unwrap_err();
    assert_eq!(
        error,
        AiError::Auth("Google has limited this Gemini account for Prohibited Use Policy violations".to_owned())
    );
    assert!(error.is_permanent());
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn a_version_id_without_a_price_is_reported_as_the_priced_model_asked_for() {
    let version = |id: &str| {
        let text = json!({"content": {"parts": [{"text": "{\"answer\": 1}"}]}, "finishReason": "STOP"});
        Reply::json(
            200,
            json!({"candidates": [text], "modelVersion": id,
                "usageMetadata": {"promptTokenCount": 2000, "candidatesTokenCount": 100}}),
        )
    };
    let server = MockServer::start(vec![version("gemini-3.8-flash-001"), version("gemini-9-flash-001")]);
    let settings = settings(&server);
    let provider = connect(&settings).unwrap();
    let reply = provider.complete(&request()).unwrap();
    assert_eq!(reply.model, "gemini-3.8-flash");
    let prices = prices_for(&settings, &reply.model).expect("the model asked for has a list price");
    assert!(cost_usd(&reply.usage, &prices) > 0.0);
    let unpriced = ModelRequest {
        model: "gemini-9-flash".to_owned(),
        ..request()
    };
    let reply = provider.complete(&unpriced).unwrap();
    assert_eq!(
        reply.model, "gemini-9-flash-001",
        "with neither priced, the version Gemini reports is named"
    );
}

#[test]
fn a_missing_key_is_a_setup_problem() {
    let settings = AiSettings {
        provider: ProviderKind::Gemini,
        ..AiSettings::default()
    };
    let Err(AiError::Config(message)) = connect(&settings) else {
        panic!("connecting without a key must fail");
    };
    assert_eq!(message, "add a Gemini API key in the AI settings");
}
