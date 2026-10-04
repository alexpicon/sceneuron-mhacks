// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the requests of the Gemini `generateContent` client against a scripted server on
//! 127.0.0.1: the path, header, and body it sends, model names that cannot go into a URL, the
//! schema rewritten for Gemini and where it went, logged, and thinking per effort and model.

mod common;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use common::TEST_KEY;
use common::gemini::{IMAGE, PATH, keys, request, settings, success};
use common::logs::logged;
use common::server::MockServer;
use sceneuron_ai::{AiError, ModelRequest, connect, plan_schema};
use serde_json::{Value, json};

#[test]
fn requests_carry_the_key_in_a_header_and_the_body_gemini_expects() {
    let server = MockServer::start(vec![success()]);
    let provider = connect(&settings(&server)).unwrap();
    assert_eq!(provider.kind(), "gemini");
    provider.complete(&request()).unwrap();
    let sent = &server.requests()[0];
    assert_eq!((sent.method.as_str(), sent.path.as_str()), ("POST", PATH));
    assert_eq!(sent.header("x-goog-api-key"), Some(TEST_KEY));
    assert_eq!(sent.header("content-type"), Some("application/json"));
    for absent in ["authorization", "x-api-key"] {
        assert_eq!(sent.header(absent), None, "{absent}");
    }
    assert!(
        !sent.path.contains(TEST_KEY) && !sent.path.contains('?'),
        "{}",
        sent.path
    );
    let body = sent.json();
    assert_eq!(
        body["systemInstruction"],
        json!({"parts": [{"text": "You judge camera frames."}]})
    );
    assert_eq!(body["contents"][0]["role"], "user");
    assert_eq!(
        body["contents"][0]["parts"],
        json!([{"inlineData": {"mimeType": "image/jpeg", "data": STANDARD.encode(IMAGE)}},
            {"text": "Is a cat visible?"}]),
        "the image comes before the text"
    );
    let config = &body["generationConfig"];
    assert_eq!(config["maxOutputTokens"], 4000);
    assert_eq!(config["thinkingConfig"], json!({"thinkingLevel": "LOW"}));
    assert_eq!(
        config["responseFormat"],
        json!({"text": {"mimeType": "APPLICATION_JSON", "schema": request().schema}})
    );
    for absent in [
        "temperature",
        "topP",
        "topK",
        "candidateCount",
        "responseMimeType",
        "responseJsonSchema",
    ] {
        assert!(config.get(absent).is_none(), "{absent} must not be sent");
        assert!(body.get(absent).is_none(), "{absent} must not be sent");
    }
}

#[test]
fn a_models_prefix_is_dropped_and_names_unfit_for_a_url_are_refused_before_any_request() {
    let server = MockServer::start(vec![success()]);
    let provider = connect(&settings(&server)).unwrap();
    let prefixed = ModelRequest {
        model: "models/gemini-3.8-flash".to_owned(),
        ..request()
    };
    provider.complete(&prefixed).unwrap();
    for model in [
        "",
        "models/",
        "gemini/../x",
        "gemini-3.8-flash?alt=sse",
        "gemini 3.8",
        "models/models/x",
        "a#b",
    ] {
        let unfit = ModelRequest {
            model: model.to_owned(),
            ..request()
        };
        let error = provider.complete(&unfit).unwrap_err();
        assert!(matches!(error, AiError::Config(_)), "{model:?}: {error:?}");
    }
    let sent = server.requests();
    assert_eq!(sent.len(), 1, "only the prefixed model was asked");
    assert_eq!(sent[0].path, PATH);
}

#[test]
fn every_const_becomes_a_one_value_enum_and_the_plan_goes_as_structured_output() {
    logged();
    let server = MockServer::start(vec![success(), success()]);
    let provider = connect(&settings(&server)).unwrap();
    let plan = ModelRequest {
        schema: plan_schema(),
        schema_name: "edit_plan".to_owned(),
        ..request()
    };
    provider.complete(&plan).unwrap();
    let body = server.requests()[0].json();
    let format = &body["generationConfig"]["responseFormat"]["text"];
    assert_eq!(format["mimeType"], "APPLICATION_JSON");
    let sent = &format["schema"];
    assert!(!keys(sent).contains(&"const".to_owned()), "{sent}");
    let constants = constants(&plan_schema());
    assert!(
        constants.len() > 10,
        "plan_schema keeps const for OpenAI-compatible servers: {constants:?}"
    );
    let enums = one_value_enums(sent);
    for constant in &constants {
        assert!(enums.contains(constant), "{constant} is not a one-value enum");
    }
    assert_eq!(
        body["systemInstruction"]["parts"][0]["text"],
        "You judge camera frames."
    );
    let line = "the edit_plan schema goes as structured output";
    assert!(logged().iter().any(|logged| logged == line), "{line:?} was not logged");

    let tricky = ModelRequest {
        schema: json!({"type": "object", "additionalProperties": false, "required": ["const"],
            "properties": {"const": {"type": "string", "const": "x", "enum": ["x", "y"]}},
            "$defs": {"kind": {"anyOf": [{"const": 1}, {"type": "string", "enum": ["const"]}]}}}),
        ..request()
    };
    provider.complete(&tricky).unwrap();
    let sent = &server.requests()[1].json()["generationConfig"]["responseFormat"]["text"]["schema"];
    assert_eq!(
        sent["properties"]["const"],
        json!({"type": "string", "enum": ["x"]}),
        "a property may be called const, and a constant wins over the enum beside it"
    );
    assert_eq!(
        sent["$defs"]["kind"]["anyOf"],
        json!([{"enum": [1]}, {"type": "string", "enum": ["const"]}])
    );
}

/// Every `const` value in `schema`.
fn constants(schema: &Value) -> Vec<Value> {
    let mut found = Vec::new();
    walk(schema, &mut |object| found.extend(object.get("const").cloned()));
    found
}

/// The value of every `enum` of exactly one value in `schema`.
fn one_value_enums(schema: &Value) -> Vec<Value> {
    let mut found = Vec::new();
    walk(schema, &mut |object| {
        if let Some([value]) = object.get("enum").and_then(Value::as_array).map(Vec::as_slice) {
            found.push(value.clone());
        }
    });
    found
}

fn walk(value: &Value, visit: &mut dyn FnMut(&serde_json::Map<String, Value>)) {
    match value {
        Value::Object(object) => {
            visit(object);
            object.values().for_each(|value| walk(value, visit));
        }
        Value::Array(items) => items.iter().for_each(|item| walk(item, visit)),
        _ => {}
    }
}

#[test]
fn thinking_follows_the_effort_as_a_level_or_on_gemini_2_as_a_budget() {
    let cases = [
        ("gemini-3.8-flash", "low", Some(json!({"thinkingLevel": "LOW"}))),
        ("gemini-3.8-flash", "medium", Some(json!({"thinkingLevel": "MEDIUM"}))),
        ("gemini-3.1-pro-preview", "high", Some(json!({"thinkingLevel": "HIGH"}))),
        ("gemini-2.5-flash", "low", Some(json!({"thinkingBudget": 1024}))),
        ("gemini-2.5-flash", "medium", Some(json!({"thinkingBudget": 8192}))),
        (
            "models/gemini-2.5-flash-lite",
            "high",
            Some(json!({"thinkingBudget": 24576})),
        ),
        ("gemini-3.5-flash-lite", "", None),
        ("gemini-2.5-flash", "", None),
    ];
    let server = MockServer::start(cases.iter().map(|_| success()).collect());
    let provider = connect(&settings(&server)).unwrap();
    for (model, effort, _) in &cases {
        let asked = ModelRequest {
            model: (*model).to_owned(),
            effort: (*effort).to_owned(),
            ..request()
        };
        provider.complete(&asked).unwrap();
    }
    for ((model, effort, expected), sent) in cases.iter().zip(server.requests()) {
        let thinking = sent.json()["generationConfig"].get("thinkingConfig").cloned();
        assert_eq!(&thinking, expected, "{model} at {effort:?}");
    }
}
