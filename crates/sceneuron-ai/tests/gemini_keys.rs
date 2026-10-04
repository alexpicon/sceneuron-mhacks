// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises how the Gemini client explains Google's refusals of a key, against a scripted server
//! on 127.0.0.1: a key Google does not take for the Gemini API, one whose API restrictions block
//! the Generative Language API, and one whose application restrictions leave out this computer
//! become a key problem that says what to do, never retried; other refusals keep Google's words.

mod common;

use common::TEST_KEY;
use common::gemini::{error, key_refused, request, settings};
use common::server::{MockServer, Reply};
use sceneuron_ai::{AiError, connect};

/// How Google answered an AI Studio auth key sent in `x-goog-api-key`, as it was seen live.
const TOKEN_TYPE_UNSUPPORTED: &str = r#"{"error":{"code":401,"message":"Request had invalid authentication credentials. Expected OAuth 2 access token, login cookie or other valid authentication credential. See https://developers.google.com/identity/sign-in/web/devconsole-project.","status":"UNAUTHENTICATED","details":[{"@type":"type.googleapis.com/google.rpc.ErrorInfo","reason":"ACCESS_TOKEN_TYPE_UNSUPPORTED","domain":"googleapis.com"}]}}"#;
/// How Google answers a key whose API restrictions leave out the Generative Language API.
const SERVICE_BLOCKED: &str = r#"{"error":{"code":403,"message":"Requests to this API generativelanguage.googleapis.com method google.ai.generativelanguage.v1beta.GenerativeService.GenerateContent are blocked.","status":"PERMISSION_DENIED","details":[{"@type":"type.googleapis.com/google.rpc.ErrorInfo","reason":"API_KEY_SERVICE_BLOCKED","domain":"googleapis.com","metadata":{"service":"generativelanguage.googleapis.com","methodName":"google.ai.generativelanguage.v1beta.GenerativeService.GenerateContent","consumer":"projects/123456789012"}},{"@type":"type.googleapis.com/google.rpc.LocalizedMessage","locale":"en-US","message":"Requests to this API generativelanguage.googleapis.com method google.ai.generativelanguage.v1beta.GenerativeService.GenerateContent are blocked."}]}}"#;
const BLOCKED: &str = "Requests to this API generativelanguage.googleapis.com method \
                       google.ai.generativelanguage.v1beta.GenerativeService.GenerateContent are blocked.";

fn raw(code: u16, body: &str) -> Reply {
    Reply::bytes(code, "application/json; charset=UTF-8", body.as_bytes().to_vec())
}

/// What the streamer is told for a key Google refuses for the Gemini API with `reason`.
fn refused_for_gemini(reason: &str) -> AiError {
    AiError::Auth(format!(
        "Google refused this key for the Gemini API ({reason}). Create a key in Google AI Studio \
         (https://aistudio.google.com/apikey) for a project with the Gemini API, or allow the Generative \
         Language API in the key's restrictions."
    ))
}

/// Asks once with a server that answers `reply`, and returns the error.
fn refusal(reply: Reply) -> AiError {
    let server = MockServer::start(vec![reply]);
    let error = connect(&settings(&server)).unwrap().complete(&request()).unwrap_err();
    assert_eq!(server.requests().len(), 1, "a refused key is never retried");
    error
}

#[test]
fn a_key_google_does_not_take_for_the_gemini_api_says_where_to_get_one() {
    let server = MockServer::start(vec![raw(401, TOKEN_TYPE_UNSUPPORTED)]);
    let error = connect(&settings(&server)).unwrap().complete(&request()).unwrap_err();
    assert_eq!(error, refused_for_gemini("ACCESS_TOKEN_TYPE_UNSUPPORTED"));
    assert!(error.is_permanent());
    let shown = error.to_string();
    assert!(
        shown.starts_with("the AI provider rejected the API key: Google refused this key"),
        "{shown}"
    );
    assert!(!shown.contains("OAuth 2") && !shown.contains(TEST_KEY), "{shown}");
    let sent = server.requests();
    assert_eq!(sent.len(), 1, "a refused key is never retried");
    assert_eq!(sent[0].header("x-goog-api-key"), Some(TEST_KEY));
    assert_eq!(sent[0].header("authorization"), None);
}

#[test]
fn a_key_whose_restrictions_block_the_generative_language_api_says_how_to_allow_it() {
    assert_eq!(
        refusal(raw(403, SERVICE_BLOCKED)),
        refused_for_gemini("API_KEY_SERVICE_BLOCKED")
    );
    let unauthenticated = key_refused(401, "UNAUTHENTICATED", "API_KEY_SERVICE_BLOCKED", BLOCKED);
    assert_eq!(refusal(unauthenticated), refused_for_gemini("API_KEY_SERVICE_BLOCKED"));
    let denied = key_refused(403, "PERMISSION_DENIED", "ACCESS_TOKEN_TYPE_UNSUPPORTED", "Denied.");
    assert_eq!(refusal(denied), refused_for_gemini("ACCESS_TOKEN_TYPE_UNSUPPORTED"));
}

#[test]
fn a_key_restricted_to_other_callers_says_to_lift_the_restriction() {
    for reason in [
        "API_KEY_IP_ADDRESS_BLOCKED",
        "API_KEY_HTTP_REFERRER_BLOCKED",
        "API_KEY_ANDROID_APP_BLOCKED",
        "API_KEY_IOS_APP_BLOCKED",
    ] {
        let message = format!("Requests from this caller are blocked ({reason}).");
        let error = refusal(key_refused(403, "PERMISSION_DENIED", reason, &message));
        let expected = format!(
            "Google refused this key from this computer ({reason}). Remove the key's application restrictions, \
             or create a key in Google AI Studio (https://aistudio.google.com/apikey)."
        );
        assert_eq!(error, AiError::Auth(expected));
    }
}

#[test]
fn other_refusals_keep_googles_words() {
    let bare = error(
        401,
        "UNAUTHENTICATED",
        "Request had invalid authentication credentials.",
    );
    assert_eq!(
        refusal(bare),
        AiError::Auth("Request had invalid authentication credentials.".to_owned())
    );
    let disabled = "Generative Language API has not been used in project 123456789012 before or it is disabled.";
    let error = refusal(key_refused(403, "PERMISSION_DENIED", "SERVICE_DISABLED", disabled));
    assert_eq!(
        error,
        AiError::Auth(format!("the key may not use this model or feature: {disabled}"))
    );
}
