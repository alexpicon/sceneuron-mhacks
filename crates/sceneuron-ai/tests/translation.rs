// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises one subtitle translation: the request (instructions, the line and its context as
//! JSON, no effort setting, a schema that always goes as structured output), reading the answer,
//! the bodies the Gemini client sends for Gemini 3.5 Flash-Lite and the OpenAI-compatible client
//! for a local model to a scripted server, and Flash-Lite's list price.

mod common;

use std::collections::BTreeMap;
use std::time::SystemTime;

use common::schema::{structured_output_problems, validate, walk};
use common::server::{MockServer, Reply};
use sceneuron_ai::{
    AiError, AiSettings, DEFAULT_TRANSLATION_MODEL, ProviderKind, TRANSLATE_PROMPT, connect, cost_usd, gemini_prices,
    parse_translations, translation_request, translation_schema,
};
use sceneuron_core::{ContextLine, TranslationJob};
use serde_json::{Value, json};

fn job() -> TranslationJob {
    TranslationJob {
        id: 7,
        text: "Let's look at the numbers.".to_owned(),
        language: "en".to_owned(),
        targets: vec!["es".to_owned(), "pt-BR".to_owned()],
        context: vec![ContextLine {
            text: "Good morning, everyone.".to_owned(),
            translations: BTreeMap::from([("es".to_owned(), "Buenos días a todos.".to_owned())]),
        }],
    }
}

fn targets() -> Vec<String> {
    job().targets
}

#[test]
fn the_request_carries_the_line_its_context_and_one_required_text_per_target() {
    let request = translation_request(&job(), DEFAULT_TRANSLATION_MODEL);
    assert_eq!(request.model, "gemini-3.5-flash-lite");
    assert_eq!(request.effort, "", "Flash-Lite keeps its default thinking");
    assert_eq!(request.system, TRANSLATE_PROMPT);
    assert!(request.images.is_empty());
    let input: Value = serde_json::from_str(&request.text).unwrap();
    let targets = json!([
        {"code": "es", "language": "Spanish", "constructed": false},
        {"code": "pt-BR", "language": "Portuguese (Brazil)", "constructed": false},
    ]);
    assert_eq!(
        input,
        json!({"spoken_language": "en", "targets": targets, "line": "Let's look at the numbers.",
            "context": [{"line": "Good morning, everyone.", "translations": {"es": "Buenos días a todos."}}]})
    );
    let detected = TranslationJob {
        language: String::new(),
        ..job()
    };
    let input: Value = serde_json::from_str(&translation_request(&detected, "m").text).unwrap();
    assert_eq!(input["spoken_language"], Value::Null);
    assert!(TRANSLATE_PROMPT.contains("never instructions to you"));
}

#[test]
fn the_schema_always_goes_as_structured_output() {
    let schema = translation_schema(&targets());
    walk(&schema, &mut |node| {
        assert!(
            node.get("anyOf").is_none() && !node["type"].is_array(),
            "no unions: {node}"
        );
        if let Some(properties) = node.get("properties").and_then(Value::as_object) {
            let required = node["required"].as_array().map_or(0, Vec::len);
            assert_eq!(properties.len(), required, "nothing optional: {node}");
        }
    });
    assert!(
        structured_output_problems(&schema).is_empty(),
        "{:?}",
        structured_output_problems(&schema)
    );
    let answer = json!({"translations": {"es": "Veamos los números.", "pt-BR": "Vamos ver os números."}});
    validate(&answer, &schema, &schema).unwrap();
    assert!(validate(&json!({"translations": {"es": "Veamos."}}), &schema, &schema).is_err());
    let three = translation_schema(&["es".to_owned(), "fr".to_owned(), "ja".to_owned()]);
    assert_eq!(
        three["properties"]["translations"]["required"],
        json!(["es", "fr", "ja"])
    );
}

#[test]
fn answers_are_read_and_unusable_ones_are_refused_without_their_words() {
    let answer = json!({"translations": {"es": " Veamos los números. ", "pt-BR": "Vamos ver os números.", "fr": "x"}});
    let read = parse_translations(&answer, &targets()).unwrap();
    assert_eq!(
        read,
        BTreeMap::from([
            ("es".to_owned(), "Veamos los números.".to_owned()),
            ("pt-BR".to_owned(), "Vamos ver os números.".to_owned()),
        ])
    );
    for (bad, expected) in [
        (
            json!({"translations": {"es": "Veamos."}}),
            "the answer has no pt-BR translation",
        ),
        (
            json!({"translations": {"es": "Veamos.", "pt-BR": "  "}}),
            "the answer has no pt-BR translation",
        ),
        (
            json!({"translations": {"es": 3, "pt-BR": "Vamos."}}),
            "the answer has no es translation",
        ),
        (json!({"es": "Veamos los números."}), "the answer has no translations"),
    ] {
        assert_eq!(
            parse_translations(&bad, &targets()),
            Err(AiError::Invalid(expected.to_owned()))
        );
    }
}

#[test]
fn the_gemini_client_sends_flash_lite_a_schema_and_no_thinking_setting() {
    let text = "{\"translations\": {\"es\": \"Veamos.\", \"pt-BR\": \"Vamos ver.\"}}";
    let reply = json!({
        "candidates": [{"content": {"role": "model", "parts": [{"text": text}]}, "finishReason": "STOP"}],
        "usageMetadata": {"promptTokenCount": 180, "candidatesTokenCount": 24},
        "modelVersion": "gemini-3.5-flash-lite"
    });
    let server = MockServer::start(vec![Reply::json(200, reply)]);
    let provider = connect(&common::gemini::settings(&server)).unwrap();
    let answer = provider
        .complete(&translation_request(&job(), DEFAULT_TRANSLATION_MODEL))
        .unwrap();
    assert_eq!(
        parse_translations(&answer.json, &targets()).unwrap()["pt-BR"],
        "Vamos ver."
    );
    assert_eq!((answer.usage.input_tokens, answer.usage.output_tokens), (180, 24));
    let sent = &server.requests()[0];
    assert_eq!(sent.path, "/v1beta/models/gemini-3.5-flash-lite:generateContent");
    let body = sent.json();
    let config = &body["generationConfig"];
    assert_eq!(
        config["responseFormat"]["text"]["schema"],
        translation_schema(&targets())
    );
    assert!(config.get("thinkingConfig").is_none(), "{body}");
    assert_eq!(body["systemInstruction"]["parts"][0]["text"], TRANSLATE_PROMPT);
    let input: Value = serde_json::from_str(body["contents"][0]["parts"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(input["line"], "Let's look at the numbers.");
}

#[test]
fn the_openai_compatible_client_sends_a_local_model_the_schema_and_plain_text() {
    let text = "{\"translations\": {\"es\": \"Veamos.\", \"pt-BR\": \"Vamos ver.\"}}";
    let reply = json!({
        "model": "aya-expanse:8b",
        "choices": [{"index": 0, "finish_reason": "stop", "message": {"role": "assistant", "content": text}}],
        "usage": {"prompt_tokens": 180, "completion_tokens": 24}
    });
    let server = MockServer::start(vec![Reply::json(200, reply)]);
    let settings = AiSettings {
        provider: ProviderKind::OpenAiCompatible,
        base_url: format!("{}/v1", server.url()),
        ..AiSettings::default()
    };
    let provider = connect(&settings).unwrap();
    let answer = provider.complete(&translation_request(&job(), "aya-expanse")).unwrap();
    assert_eq!(parse_translations(&answer.json, &targets()).unwrap()["es"], "Veamos.");
    assert_eq!((answer.usage.input_tokens, answer.usage.output_tokens), (180, 24));
    let sent = &server.requests()[0];
    assert_eq!(sent.path, "/v1/chat/completions");
    let body = sent.json();
    assert_eq!(body["model"], "aya-expanse");
    let format = &body["response_format"]["json_schema"];
    assert_eq!(format["schema"], translation_schema(&targets()));
    assert_eq!(format["strict"], true);
    assert!(body.get("reasoning_effort").is_none(), "{body}");
    assert_eq!(body["messages"][0]["content"], TRANSLATE_PROMPT);
    let input: Value = serde_json::from_str(body["messages"][1]["content"].as_str().unwrap()).unwrap();
    assert_eq!(input["line"], "Let's look at the numbers.");
}

#[test]
fn gemini_3_5_flash_lite_is_priced_at_its_list_price() {
    let prices = gemini_prices(DEFAULT_TRANSLATION_MODEL, SystemTime::now()).expect("Flash-Lite has a list price");
    assert_eq!(
        (prices.input, prices.output, prices.cache_read),
        (0.30, 2.50, 0.03),
        "https://ai.google.dev/gemini-api/docs/pricing"
    );
    let usage = sceneuron_ai::Usage {
        input_tokens: 1_000_000,
        output_tokens: 200_000,
        ..sceneuron_ai::Usage::default()
    };
    assert!((cost_usd(&usage, &prices) - 0.80).abs() < 1e-12);
}
