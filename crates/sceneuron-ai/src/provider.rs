// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The interface every model provider implements: requests, replies, token usage, and errors.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::gemini::GeminiProvider;
use crate::openai::OpenAiProvider;
use crate::provider_kind::ProviderKind;
use crate::settings::AiSettings;

/// One question for a model: instructions, the question, optional camera images, and the JSON
/// shape the answer must take.
#[derive(Clone, PartialEq)]
pub struct ModelRequest {
    pub model: String,
    /// Reasoning depth: `low`, `medium`, or `high`; empty sets none, for a model that takes no
    /// effort setting.
    pub effort: String,
    pub system: String,
    pub text: String,
    /// JPEG images, shown to the model together with the text.
    pub images: Vec<Vec<u8>>,
    /// JSON schema the answer must match.
    pub schema: Value,
    /// Short name of the schema, for providers that label structured output.
    pub schema_name: String,
    /// Upper bound on generated tokens, thinking included.
    pub max_tokens: u32,
    /// Deadline for the whole call, retries included.
    pub timeout: Duration,
}

impl fmt::Debug for ModelRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let image_bytes: Vec<usize> = self.images.iter().map(Vec::len).collect();
        f.debug_struct("ModelRequest")
            .field("model", &self.model)
            .field("effort", &self.effort)
            .field("system_chars", &self.system.len())
            .field("text", &self.text)
            .field("image_bytes", &image_bytes)
            .field("schema_name", &self.schema_name)
            .field("max_tokens", &self.max_tokens)
            .field("timeout", &self.timeout)
            .finish()
    }
}

/// Tokens one call consumed, as the provider reported them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Usage {
    /// Input tokens billed at the full rate (not read from the prompt cache).
    pub input_tokens: u64,
    /// Generated tokens, thinking included.
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
}

impl Usage {
    /// Adds another call's tokens to these.
    pub fn add(&mut self, other: &Usage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_read_tokens += other.cache_read_tokens;
    }
}

/// A successful answer: the JSON value, what it cost in tokens, and who produced it.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelReply {
    pub json: Value,
    pub usage: Usage,
    /// The model that produced the answer, which can differ from the one asked when the
    /// provider routes a declined request to a fallback model.
    pub model: String,
    pub latency_ms: f64,
}

/// Why a model call failed. Messages never contain the API key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiError {
    /// The settings or the input cannot work, such as a missing API key.
    Config(String),
    /// The provider rejected the API key or its permissions.
    Auth(String),
    /// The provider rejected the request itself, such as an unknown model.
    BadRequest(String),
    /// The provider asked to slow down.
    RateLimited(String),
    /// The provider is overloaded or unreachable, or the call timed out.
    Unavailable(String),
    /// The model declined to answer.
    Refused(String),
    /// The answer was cut off or could not be used.
    Invalid(String),
}

impl AiError {
    /// Whether retrying cannot help until the settings change.
    pub fn is_permanent(&self) -> bool {
        matches!(self, AiError::Config(_) | AiError::Auth(_) | AiError::BadRequest(_))
    }

    /// The detail message without the category prefix.
    pub fn message(&self) -> &str {
        match self {
            AiError::Config(message)
            | AiError::Auth(message)
            | AiError::BadRequest(message)
            | AiError::RateLimited(message)
            | AiError::Unavailable(message)
            | AiError::Refused(message)
            | AiError::Invalid(message) => message,
        }
    }
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AiError::Config(message) => f.write_str(message),
            AiError::Auth(message) => write!(f, "the AI provider rejected the API key: {message}"),
            AiError::BadRequest(message) => write!(f, "the AI provider rejected the request: {message}"),
            AiError::RateLimited(message) => write!(f, "the AI provider asked to slow down: {message}"),
            AiError::Unavailable(message) => write!(f, "the AI provider is unavailable: {message}"),
            AiError::Refused(message) => write!(f, "the model declined: {message}"),
            AiError::Invalid(message) => write!(f, "the model's answer could not be used: {message}"),
        }
    }
}

impl std::error::Error for AiError {}

/// A model service that answers [`ModelRequest`]s with JSON.
pub trait Provider: Send + Sync {
    /// Asks the model and waits for the answer, at most until the request's timeout.
    fn complete(&self, request: &ModelRequest) -> Result<ModelReply, AiError>;

    /// A short name for the status display, such as `gemini`.
    fn kind(&self) -> &'static str;
}

/// Builds a provider from settings; tests and demos substitute their own.
pub type Connector = Arc<dyn Fn(&AiSettings) -> Result<Arc<dyn Provider>, AiError> + Send + Sync>;

/// Builds the provider the settings select.
pub fn connect(settings: &AiSettings) -> Result<Arc<dyn Provider>, AiError> {
    match settings.provider {
        ProviderKind::Gemini => Ok(Arc::new(GeminiProvider::new(settings)?)),
        ProviderKind::OpenAiCompatible => Ok(Arc::new(OpenAiProvider::new(settings))),
    }
}

/// The connector that talks to real providers.
pub fn default_connector() -> Connector {
    Arc::new(connect)
}
