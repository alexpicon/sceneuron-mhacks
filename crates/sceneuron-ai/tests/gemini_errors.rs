// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises how the Gemini client handles failures against a scripted server on 127.0.0.1:
//! Gemini's own answers to an unknown key, an unserved project, and spent credits; retries for
//! rate limits and overloads, after the server's delay when it gives one; other permanent errors;
//! the call deadline; and the fallbacks, logged, when the endpoint refuses `responseFormat` or a
//! schema. The key never shows in an error.

mod common;

use std::time::{Duration, Instant};

use common::TEST_KEY;
use common::gemini::{error, invalid_key, request, settings, success};
use common::logs::logged;
use common::server::MockServer;
use sceneuron_ai::{AiError, AiSettings, ModelRequest, ProviderKind, connect};
use serde_json::json;

#[test]
fn an_unknown_key_answered_with_400_is_rejected_for_good_and_never_echoed() {
    let echoed = error(400, "INVALID_ARGUMENT", &format!("API key not valid: {TEST_KEY}"));
    let server = MockServer::start(vec![invalid_key(), echoed]);
    let provider = connect(&settings(&server)).unwrap();
    let error = provider.complete(&request()).unwrap_err();
    assert_eq!(
        error,
        AiError::Auth("API key not valid. Please pass a valid API key.".to_owned())
    );
    assert!(error.is_permanent());
    let error = provider.complete(&request()).unwrap_err();
    assert!(matches!(error, AiError::Auth(_)), "{error:?}");
    let shown = error.to_string();
    assert!(!shown.contains(TEST_KEY) && shown.contains("[redacted]"), "{shown}");
    assert_eq!(server.requests().len(), 2, "neither is retried");
}

#[test]
fn an_unserved_project_and_spent_credits_are_setup_problems_and_not_retried() {
    let unserved = error(
        400,
        "FAILED_PRECONDITION",
        "Gemini API free tier is not available in your country. Please enable billing on your project.",
    );
    let spent = error(402, "RESOURCE_EXHAUSTED", "Your prepayment credits are depleted.");
    let server = MockServer::start(vec![unserved, spent]);
    let provider = connect(&settings(&server)).unwrap();
    for detail in ["free tier is not available", "credits are depleted"] {
        let error = provider.complete(&request()).unwrap_err();
        assert!(
            matches!(error, AiError::Config(ref message) if message.contains(detail)),
            "{error:?}"
        );
    }
    assert_eq!(server.requests().len(), 2, "each setup problem was asked once");
}

#[test]
fn rate_limits_and_overloads_are_retried_twice_then_reported() {
    let limited = error(429, "RESOURCE_EXHAUSTED", "Quota exceeded.");
    let overloaded = || error(503, "UNAVAILABLE", "The model is overloaded.");
    let server = MockServer::start(vec![limited, overloaded(), success()]);
    let reply = connect(&settings(&server)).unwrap().complete(&request()).unwrap();
    assert_eq!(reply.json["answer"], 42);
    assert_eq!(server.requests().len(), 3);

    let server = MockServer::start(vec![overloaded(), overloaded(), overloaded(), success()]);
    let error = connect(&settings(&server)).unwrap().complete(&request()).unwrap_err();
    assert!(
        matches!(error, AiError::Unavailable(ref message) if message.contains("overloaded")) && !error.is_permanent(),
        "{error:?}"
    );
    assert_eq!(server.requests().len(), 3);
}

#[test]
fn rate_limits_are_retried_after_the_servers_delay() {
    let limited = error(429, "RESOURCE_EXHAUSTED", "Quota exceeded.").header("retry-after", "1");
    let server = MockServer::start(vec![limited, success()]);
    let started = Instant::now();
    let reply = connect(&settings(&server)).unwrap().complete(&request()).unwrap();
    assert_eq!(reply.json["answer"], 42);
    assert!(started.elapsed() >= Duration::from_secs(1));
    assert_eq!(server.requests().len(), 2);
}

#[test]
fn other_client_errors_are_permanent_and_not_retried() {
    for (code, status, expected) in [
        (400, "INVALID_ARGUMENT", "BadRequest"),
        (403, "PERMISSION_DENIED", "Auth"),
        (404, "NOT_FOUND", "BadRequest"),
    ] {
        let server = MockServer::start(vec![error(code, status, "no such thing")]);
        let error = connect(&settings(&server)).unwrap().complete(&request()).unwrap_err();
        assert!(
            error.is_permanent() && format!("{error:?}").starts_with(expected),
            "{code}: {error:?}"
        );
        assert!(error.to_string().contains("no such thing"));
        assert_eq!(server.requests().len(), 1);
    }
}

#[test]
fn the_request_timeout_bounds_the_whole_call() {
    let server = MockServer::start(vec![success().delayed(Duration::from_millis(1500))]);
    let quick = ModelRequest {
        timeout: Duration::from_millis(300),
        ..request()
    };
    let started = Instant::now();
    let error = connect(&settings(&server)).unwrap().complete(&quick).unwrap_err();
    assert!(matches!(error, AiError::Unavailable(_)), "{error:?}");
    assert!(started.elapsed() < Duration::from_millis(1200));
}

#[test]
fn a_refused_response_format_gives_way_to_the_older_fields_for_good() {
    for refusal in [
        "Invalid JSON payload received. Unknown name \"responseFormat\" at 'generation_config': Cannot find field.",
        "Invalid value at 'generation_config.response_format.text.mime_type'",
    ] {
        let server = MockServer::start(vec![error(400, "INVALID_ARGUMENT", refusal), success(), success()]);
        let provider = connect(&settings(&server)).unwrap();
        provider.complete(&request()).unwrap();
        provider.complete(&request()).unwrap();
        let sent = server.requests();
        assert_eq!(sent.len(), 3, "{refusal}");
        assert!(sent[0].json()["generationConfig"].get("responseFormat").is_some());
        for later in &sent[1..] {
            let config = &later.json()["generationConfig"];
            assert!(config.get("responseFormat").is_none(), "{refusal}");
            assert_eq!(config["responseMimeType"], "application/json");
            assert_eq!(config["responseJsonSchema"], request().schema);
            assert_eq!(config["thinkingConfig"], json!({"thinkingLevel": "LOW"}));
        }
    }
}

#[test]
fn a_refused_schema_goes_in_the_instructions_from_then_on() {
    let refused = error(
        400,
        "INVALID_ARGUMENT",
        "The specified schema has too many states for serving.",
    );
    logged();
    let server = MockServer::start(vec![refused, success(), success(), success()]);
    let provider = connect(&settings(&server)).unwrap();
    // Other tests log the shared request()'s schema too, so this one has a name of its own.
    let states = ModelRequest {
        schema_name: "too_many_states".to_owned(),
        ..request()
    };
    provider.complete(&states).unwrap();
    provider.complete(&states).unwrap();
    let other = ModelRequest {
        schema_name: "other".to_owned(),
        ..request()
    };
    provider.complete(&other).unwrap();
    let sent: Vec<_> = server.requests().iter().map(|recorded| recorded.json()).collect();
    assert_eq!(sent.len(), 4, "only the first request failed");
    for instructed in &sent[1..3] {
        let config = &instructed["generationConfig"];
        for absent in ["responseFormat", "responseMimeType", "responseJsonSchema"] {
            assert!(config.get(absent).is_none(), "{absent}");
        }
        assert_eq!(config["thinkingConfig"], json!({"thinkingLevel": "LOW"}));
        let system = instructed["systemInstruction"]["parts"][0]["text"].as_str().unwrap();
        assert!(
            system.starts_with("You judge camera frames.") && system.contains("\"additionalProperties\""),
            "{system}"
        );
    }
    assert!(
        sent[3]["generationConfig"].get("responseFormat").is_some(),
        "other schemas stay structured"
    );
    let line = "the too_many_states schema goes in the instructions, because the endpoint refused it as \
                structured output";
    assert!(logged().iter().any(|logged| logged == line), "{line:?} was not logged");
}

#[test]
fn a_schema_refused_inside_response_format_tries_the_older_fields_then_the_instructions_alone() {
    let format = error(
        400,
        "INVALID_ARGUMENT",
        "Invalid value at 'generation_config.response_format.text.schema'",
    );
    let older = error(
        400,
        "INVALID_ARGUMENT",
        "generation_config.response_json_schema: the schema is too deep",
    );
    let server = MockServer::start(vec![format, older, success(), success()]);
    let provider = connect(&settings(&server)).unwrap();
    let reply = provider.complete(&request()).unwrap();
    assert_eq!(reply.json["answer"], 42);
    let other = ModelRequest {
        schema_name: "other".to_owned(),
        ..request()
    };
    provider.complete(&other).unwrap();
    let sent: Vec<_> = server.requests().iter().map(|recorded| recorded.json()).collect();
    assert_eq!(sent.len(), 4);
    assert_eq!(sent[1]["generationConfig"]["responseJsonSchema"], request().schema);
    assert!(sent[1]["generationConfig"].get("responseFormat").is_none());
    let last = &sent[2];
    assert!(last["generationConfig"].get("responseJsonSchema").is_none());
    let system = last["systemInstruction"]["parts"][0]["text"].as_str().unwrap();
    assert!(system.contains("\"answer\""), "{system}");
    assert!(
        sent[3]["generationConfig"].get("responseFormat").is_some(),
        "other schemas stay in responseFormat"
    );
}

#[test]
fn an_unreachable_server_is_unavailable_without_the_key() {
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let settings = AiSettings {
        provider: ProviderKind::Gemini,
        base_url: format!("http://127.0.0.1:{port}"),
        api_key: TEST_KEY.to_owned(),
        ..AiSettings::default()
    };
    let quick = ModelRequest {
        timeout: Duration::from_millis(400),
        ..request()
    };
    let error = connect(&settings).unwrap().complete(&quick).unwrap_err();
    assert!(matches!(error, AiError::Unavailable(_)), "{error:?}");
    assert!(!error.to_string().contains(TEST_KEY));
}
