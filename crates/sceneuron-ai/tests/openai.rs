// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the OpenAI-compatible client against a scripted server on 127.0.0.1: the bearer
//! header, data-URL images, schema output and its plain-JSON fallback, and tolerant parsing of
//! answers wrapped in prose or code fences.

mod common;

use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use common::server::{MockServer, Reply};
use sceneuron_ai::{AiError, AiSettings, ModelRequest, ProviderKind, connect};
use serde_json::json;

const IMAGE: [u8; 4] = [0xFF, 0xD8, 9, 9];

fn settings(server: &MockServer, key: &str) -> AiSettings {
    AiSettings {
        provider: ProviderKind::OpenAiCompatible,
        base_url: format!("{}/v1", server.url()),
        vision_model: "llama3.2-vision".to_owned(),
        api_key: key.to_owned(),
        ..AiSettings::default()
    }
}

fn request(images: Vec<Vec<u8>>) -> ModelRequest {
    ModelRequest {
        model: "llama3.2-vision".to_owned(),
        effort: "low".to_owned(),
        system: "You judge camera frames.".to_owned(),
        text: "Is a cat visible?".to_owned(),
        images,
        schema: json!({"type": "object", "properties": {"cat": {"type": "boolean"}},
            "required": ["cat"], "additionalProperties": false}),
        schema_name: "cat_check".to_owned(),
        max_tokens: 4000,
        timeout: Duration::from_secs(20),
    }
}

fn answer(content: serde_json::Value) -> Reply {
    Reply::json(
        200,
        json!({
            "model": "llama3.2-vision:11b",
            "choices": [{"index": 0, "finish_reason": "stop",
                "message": {"role": "assistant", "content": content}}],
            "usage": {"prompt_tokens": 300, "completion_tokens": 12}
        }),
    )
}

#[test]
fn the_bearer_header_is_sent_only_with_a_key() {
    let server = MockServer::start(vec![answer(json!("{\"cat\": true}")), answer(json!("{\"cat\": true}"))]);
    connect(&settings(&server, ""))
        .unwrap()
        .complete(&request(vec![]))
        .unwrap();
    connect(&settings(&server, "nvapi-test-key"))
        .unwrap()
        .complete(&request(vec![]))
        .unwrap();
    let sent = server.requests();
    assert_eq!(sent[0].header("authorization"), None);
    assert_eq!(sent[1].header("authorization"), Some("Bearer nvapi-test-key"));
    assert_eq!(sent[1].header("x-api-key"), None);
}

#[test]
fn requests_carry_a_data_url_image_and_a_strict_schema() {
    let server = MockServer::start(vec![answer(json!("{\"cat\": false}"))]);
    let reply = connect(&settings(&server, ""))
        .unwrap()
        .complete(&request(vec![IMAGE.to_vec()]))
        .unwrap();
    assert_eq!(reply.json, json!({"cat": false}));
    assert_eq!(reply.model, "llama3.2-vision:11b");
    assert_eq!((reply.usage.input_tokens, reply.usage.output_tokens), (300, 12));
    let sent = &server.requests()[0];
    assert_eq!(
        (sent.method.as_str(), sent.path.as_str()),
        ("POST", "/v1/chat/completions")
    );
    let body = sent.json();
    assert_eq!(body["model"], "llama3.2-vision");
    assert_eq!(body["max_tokens"], 4000);
    assert_eq!(
        body["messages"][0],
        json!({"role": "system", "content": "You judge camera frames."})
    );
    let content = &body["messages"][1]["content"];
    assert_eq!(content[0], json!({"type": "text", "text": "Is a cat visible?"}));
    let url = format!("data:image/jpeg;base64,{}", STANDARD.encode(IMAGE));
    assert_eq!(content[1], json!({"type": "image_url", "image_url": {"url": url}}));
    assert_eq!(
        body["response_format"],
        json!({"type": "json_schema", "json_schema": {"name": "cat_check", "schema": request(vec![]).schema,
            "strict": true}})
    );
}

#[test]
fn text_only_requests_send_plain_text_content() {
    let server = MockServer::start(vec![answer(json!("{\"cat\": true}"))]);
    connect(&settings(&server, ""))
        .unwrap()
        .complete(&request(vec![]))
        .unwrap();
    assert_eq!(
        server.requests()[0].json()["messages"][1]["content"],
        "Is a cat visible?"
    );
}

#[test]
fn answers_in_code_fences_prose_or_parts_are_found() {
    let fenced = answer(json!(
        "Here is the result:\n```json\n{\"cat\": true}\n```\nHope that helps."
    ));
    let braces_in_prose = answer(json!("I {think} so: {\"cat\": true, \"note\": \"a } brace\"}"));
    let parts = answer(json!([{"type": "text", "text": "{\"cat\":"}, {"type": "text", "text": " true}"}]));
    let server = MockServer::start(vec![fenced, braces_in_prose, parts]);
    let provider = connect(&settings(&server, "")).unwrap();
    assert_eq!(provider.complete(&request(vec![])).unwrap().json, json!({"cat": true}));
    assert_eq!(
        provider.complete(&request(vec![])).unwrap().json,
        json!({"cat": true, "note": "a } brace"})
    );
    assert_eq!(provider.complete(&request(vec![])).unwrap().json, json!({"cat": true}));
}

#[test]
fn servers_without_schema_output_get_a_json_object_request_from_then_on() {
    let unsupported = Reply::json(
        400,
        json!({"error": {"message": "response_format type json_schema is unsupported"}}),
    );
    let server = MockServer::start(vec![
        unsupported,
        answer(json!("{\"cat\": true}")),
        answer(json!("{\"cat\": true}")),
    ]);
    let provider = connect(&settings(&server, "")).unwrap();
    assert!(provider.complete(&request(vec![])).is_ok());
    assert!(provider.complete(&request(vec![])).is_ok());
    let sent = server.requests();
    assert_eq!(sent.len(), 3);
    for retry in &sent[1..] {
        let body = retry.json();
        assert_eq!(body["response_format"], json!({"type": "json_object"}));
        let system = body["messages"][0]["content"].as_str().unwrap();
        assert!(system.contains("JSON schema") && system.contains("\"cat\""), "{system}");
    }
}

#[test]
fn truncated_or_filtered_answers_are_not_used() {
    let truncated = Reply::json(
        200,
        json!({"choices": [{"finish_reason": "length", "message": {"content": "{\"cat\": tr"}}]}),
    );
    let filtered = Reply::json(
        200,
        json!({"choices": [{"finish_reason": "content_filter", "message": {"content": null}}]}),
    );
    let refused = Reply::json(
        200,
        json!({"choices": [{"finish_reason": "stop", "message": {"content": null, "refusal": "I can't help."}}]}),
    );
    let server = MockServer::start(vec![truncated, filtered, refused]);
    let provider = connect(&settings(&server, "")).unwrap();
    assert!(matches!(provider.complete(&request(vec![])), Err(AiError::Invalid(_))));
    assert!(matches!(provider.complete(&request(vec![])), Err(AiError::Refused(_))));
    assert_eq!(
        provider.complete(&request(vec![])),
        Err(AiError::Refused("I can't help.".to_owned()))
    );
}

#[test]
fn missing_usage_counts_as_zero() {
    let bare = Reply::json(200, json!({"choices": [{"message": {"content": "{\"cat\": true}"}}]}));
    let server = MockServer::start(vec![bare]);
    let reply = connect(&settings(&server, ""))
        .unwrap()
        .complete(&request(vec![]))
        .unwrap();
    assert_eq!((reply.usage.input_tokens, reply.usage.output_tokens), (0, 0));
    assert_eq!(reply.model, "llama3.2-vision");
}

#[test]
fn server_errors_are_retried_and_plain_error_bodies_are_read() {
    let down = Reply::json(503, json!({"error": "loading model"}));
    let server = MockServer::start(vec![down, answer(json!("{\"cat\": true}"))]);
    assert!(
        connect(&settings(&server, ""))
            .unwrap()
            .complete(&request(vec![]))
            .is_ok()
    );
    assert_eq!(server.requests().len(), 2);

    let missing = Reply::json(404, json!({"error": "model \"llava\" not found, try pulling it first"}));
    let server = MockServer::start(vec![missing]);
    let error = connect(&settings(&server, ""))
        .unwrap()
        .complete(&request(vec![]))
        .unwrap_err();
    assert!(matches!(error, AiError::BadRequest(_)) && error.is_permanent());
    assert!(error.to_string().contains("not found, try pulling it first"), "{error}");
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn an_unreachable_server_is_unavailable() {
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let settings = AiSettings {
        provider: ProviderKind::OpenAiCompatible,
        base_url: format!("http://127.0.0.1:{port}/v1"),
        ..AiSettings::default()
    };
    let quick = ModelRequest {
        timeout: Duration::from_millis(400),
        ..request(vec![])
    };
    let error = connect(&settings).unwrap().complete(&quick).unwrap_err();
    assert!(matches!(error, AiError::Unavailable(_)), "{error:?}");
}
