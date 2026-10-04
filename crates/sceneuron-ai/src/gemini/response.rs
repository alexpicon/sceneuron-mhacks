// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Reading a `generateContent` reply: blocked prompts, finish reasons, the answer without
//! thoughts, token usage, and the errors Gemini answers differently from other providers.

use serde_json::Value;

use crate::http::{Failure, Http, HttpReply};
use crate::json_text::extract_object;
use crate::provider::{AiError, Usage};

/// Finish reasons that mean the answer was blocked, besides every `IMAGE_*` one.
const BLOCKED: [&str; 7] = [
    "SAFETY",
    "RECITATION",
    "LANGUAGE",
    "BLOCKLIST",
    "PROHIBITED_CONTENT",
    "SPII",
    "ESCALATION",
];
/// Reasons Google gives for not taking a key as Gemini API credentials at all, or for a key
/// whose API restrictions leave out the Generative Language API.
const KEY_REFUSALS: [&str; 2] = ["ACCESS_TOKEN_TYPE_UNSUPPORTED", "API_KEY_SERVICE_BLOCKED"];
/// Reasons Google gives for a key whose application restrictions leave out this computer.
const KEY_RESTRICTIONS: [&str; 4] = [
    "API_KEY_HTTP_REFERRER_BLOCKED",
    "API_KEY_IP_ADDRESS_BLOCKED",
    "API_KEY_ANDROID_APP_BLOCKED",
    "API_KEY_IOS_APP_BLOCKED",
];
const AI_STUDIO_KEYS: &str = "https://aistudio.google.com/apikey";
const ERROR_INFO: &str = "type.googleapis.com/google.rpc.ErrorInfo";

/// The JSON answer, its token usage, and the model that wrote it.
pub(super) struct Answer {
    pub json: Value,
    pub usage: Usage,
    pub model: Option<String>,
}

/// Reads a successful response: a blocked prompt first, then the first candidate's finish
/// reason, and only then its text.
pub(super) fn parse_response(body: &str) -> Result<Answer, AiError> {
    let response: Value =
        serde_json::from_str(body).map_err(|_| AiError::Invalid("the provider's response is not JSON".to_owned()))?;
    if let Some(reason) = response.pointer("/promptFeedback/blockReason").and_then(Value::as_str) {
        return Err(AiError::Refused(format!("the request was blocked ({reason})")));
    }
    let candidate = response
        .pointer("/candidates/0")
        .ok_or_else(|| AiError::Invalid("the provider's response has no answer".to_owned()))?;
    check_finish(candidate)?;
    let text: String = candidate
        .pointer("/content/parts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|part| part.get("thought").and_then(Value::as_bool) != Some(true))
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect();
    let model = response
        .get("modelVersion")
        .and_then(Value::as_str)
        .map(|version| version.strip_prefix("models/").unwrap_or(version))
        .filter(|version| !version.is_empty())
        .map(str::to_owned);
    Ok(Answer {
        json: extract_object(&text)?,
        usage: usage(&response),
        model,
    })
}

fn check_finish(candidate: &Value) -> Result<(), AiError> {
    match candidate.get("finishReason").and_then(Value::as_str) {
        None | Some("STOP") => Ok(()),
        Some("MAX_TOKENS") => Err(AiError::Invalid("the answer was cut off at the token limit".to_owned())),
        Some(reason) if BLOCKED.contains(&reason) || reason.starts_with("IMAGE_") => {
            let message = candidate
                .get("finishMessage")
                .and_then(Value::as_str)
                .filter(|text| !text.trim().is_empty())
                .map_or_else(|| format!("the answer was blocked ({reason})"), str::to_owned);
            Err(AiError::Refused(message))
        }
        // Retrying cannot help: the account itself is limited.
        Some("PUP_LIMITED_DISABLED") => Err(AiError::Auth(
            "Google has limited this Gemini account for Prohibited Use Policy violations".to_owned(),
        )),
        Some(reason) => Err(AiError::Invalid(format!(
            "the model stopped without an answer ({reason})"
        ))),
    }
}

/// Tokens as billed: the prompt count includes cached tokens, which are billed at the cache
/// price instead, and thinking is billed as output.
fn usage(response: &Value) -> Usage {
    let tokens = |field: &str| {
        response
            .pointer(&format!("/usageMetadata/{field}"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    let cached = tokens("cachedContentTokenCount");
    Usage {
        input_tokens: tokens("promptTokenCount").saturating_sub(cached),
        output_tokens: tokens("candidatesTokenCount") + tokens("thoughtsTokenCount"),
        cache_read_tokens: cached,
    }
}

/// The error an unsuccessful status stands for. Gemini answers an unknown key with 400 rather
/// than 401, a key it will not take for this API or from this computer with 401 or 403 and a
/// reason, a project it will not serve (such as a free tier unavailable in the country) with
/// 400 `FAILED_PRECONDITION`, and used-up prepaid credits with 402, which must not be retried;
/// every other status maps as it does for any provider.
pub(super) fn status_error(http: &Http, reply: &HttpReply) -> Failure {
    let failure = http.status_error(reply);
    let message = failure.error.message().to_owned();
    let body = serde_json::from_str::<Value>(&reply.body).unwrap_or_default();
    let status = body.pointer("/error/status").and_then(Value::as_str);
    let reason = error_reason(&body).unwrap_or_default();
    let error = match reply.status {
        400 if reply.body.contains("API_KEY_INVALID") || reply.body.contains("API key not valid") => {
            AiError::Auth(message)
        }
        400 if status == Some("FAILED_PRECONDITION") => {
            AiError::Config(format!("Gemini will not serve the project behind this key: {message}"))
        }
        401 | 403 if KEY_REFUSALS.contains(&reason) => AiError::Auth(format!(
            "Google refused this key for the Gemini API ({reason}). Create a key in Google AI Studio \
             ({AI_STUDIO_KEYS}) for a project with the Gemini API, or allow the Generative Language API \
             in the key's restrictions."
        )),
        401 | 403 if KEY_RESTRICTIONS.contains(&reason) => AiError::Auth(format!(
            "Google refused this key from this computer ({reason}). Remove the key's application \
             restrictions, or create a key in Google AI Studio ({AI_STUDIO_KEYS})."
        )),
        402 => AiError::Config(format!(
            "the prepaid credits for this Gemini key are used up; add credits to its billing account: {message}"
        )),
        _ => return failure,
    };
    Failure::from(error)
}

/// The reason in the error's `google.rpc.ErrorInfo` detail, such as `API_KEY_SERVICE_BLOCKED`.
fn error_reason(body: &Value) -> Option<&str> {
    body.pointer("/error/details")
        .and_then(Value::as_array)?
        .iter()
        .filter(|detail| detail.get("@type").and_then(Value::as_str) == Some(ERROR_INFO))
        .find_map(|detail| detail.get("reason").and_then(Value::as_str))
}
