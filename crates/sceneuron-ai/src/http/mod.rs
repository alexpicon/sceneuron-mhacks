// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! HTTPS transport shared by the providers: one connection pool each, a deadline per call, and
//! bounded retries for rate limits and outages. The API key never appears in an error.

mod retry;

use std::time::Duration;

use serde_json::Value;

use crate::provider::AiError;
pub(crate) use retry::with_retries;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_BODY_BYTES: u64 = 8 * 1024 * 1024;
const MAX_MESSAGE_CHARS: usize = 300;

/// The status, body, and retry hint of one HTTP exchange.
pub(crate) struct HttpReply<B = String> {
    pub status: u16,
    pub body: B,
    pub retry_after: Option<Duration>,
}

/// An error plus the server's hint on when to try again.
pub(crate) struct Failure {
    pub error: AiError,
    pub retry_after: Option<Duration>,
}

impl From<AiError> for Failure {
    fn from(error: AiError) -> Failure {
        Failure {
            error,
            retry_after: None,
        }
    }
}

/// A connection pool for one provider and the secret its errors must never echo.
pub(crate) struct Http {
    agent: ureq::Agent,
    secret: String,
}

impl Http {
    pub fn new(base_url: &str, secret: &str) -> Http {
        let mut config = ureq::Agent::config_builder()
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .http_status_as_error(false)
            .max_redirects(0)
            .user_agent(concat!("sceneuron/", env!("CARGO_PKG_VERSION")));
        // A model server on this computer must never be reached through a proxy from the
        // environment, which could not reach it anyway.
        if is_loopback(base_url) {
            config = config.proxy(None);
        }
        Http {
            agent: config.build().into(),
            secret: secret.to_owned(),
        }
    }

    /// Sends one JSON POST and returns whatever status the server answered.
    pub fn post(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &[u8],
        timeout: Duration,
    ) -> Result<HttpReply, AiError> {
        let reply = self.post_bytes(url, headers, body, timeout)?;
        let text = String::from_utf8(reply.body).map_err(|_| AiError::Invalid("the answer is not text".to_owned()))?;
        Ok(HttpReply {
            status: reply.status,
            body: text,
            retry_after: reply.retry_after,
        })
    }

    /// Sends one JSON POST whose answer may be binary, such as audio.
    pub fn post_bytes(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &[u8],
        timeout: Duration,
    ) -> Result<HttpReply<Vec<u8>>, AiError> {
        let mut request = self.agent.post(url).header("content-type", "application/json");
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let result = request.config().timeout_global(Some(timeout)).build().send(body);
        self.read_reply(result, url)
    }

    /// Sends one GET and returns whatever status the server answered.
    pub fn get_bytes(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        timeout: Duration,
    ) -> Result<HttpReply<Vec<u8>>, AiError> {
        let mut request = self.agent.get(url);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let result = request.config().timeout_global(Some(timeout)).build().call();
        self.read_reply(result, url)
    }

    fn read_reply(
        &self,
        result: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
        url: &str,
    ) -> Result<HttpReply<Vec<u8>>, AiError> {
        let mut response = result.map_err(|error| self.transport_error(error, url))?;
        let status = response.status().as_u16();
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(parse_retry_after);
        let body = response
            .body_mut()
            .with_config()
            .limit(MAX_BODY_BYTES)
            .read_to_vec()
            .map_err(|error| self.transport_error(error, url))?;
        Ok(HttpReply {
            status,
            body,
            retry_after,
        })
    }

    /// The error an unsuccessful status stands for, with the server's message.
    pub fn status_error(&self, reply: &HttpReply) -> Failure {
        self.error_for(reply.status, &reply.body, reply.retry_after)
    }

    /// [`Http::status_error`] for a reply read as bytes.
    pub fn bytes_error(&self, reply: &HttpReply<Vec<u8>>) -> Failure {
        self.error_for(reply.status, &String::from_utf8_lossy(&reply.body), reply.retry_after)
    }

    fn error_for(&self, status: u16, body: &str, retry_after: Option<Duration>) -> Failure {
        // Redacted before the message is cut, so a key that straddles the cut cannot leave its
        // first part behind.
        let message = self.redact(&error_message(&self.redact(body)));
        let error = match status {
            401 => AiError::Auth(message),
            403 => AiError::Auth(format!("the key may not use this model or feature: {message}")),
            404 => AiError::BadRequest(format!("not found; check the model name and base URL: {message}")),
            413 => AiError::BadRequest(format!("the request is too large: {message}")),
            408 => AiError::Unavailable(message),
            429 => AiError::RateLimited(message),
            300..=399 => AiError::BadRequest(format!(
                "the server redirected the request; check the base URL ({message})"
            )),
            500.. => AiError::Unavailable(message),
            _ => AiError::BadRequest(message),
        };
        Failure { error, retry_after }
    }

    /// Replaces the secret wherever a server echoed it back.
    pub fn redact(&self, text: &str) -> String {
        if self.secret.is_empty() {
            text.to_owned()
        } else {
            text.replace(&self.secret, "[redacted]")
        }
    }

    fn transport_error(&self, error: ureq::Error, url: &str) -> AiError {
        let host = url
            .split("://")
            .nth(1)
            .and_then(|rest| rest.split('/').next())
            .unwrap_or(url);
        match error {
            ureq::Error::Timeout(_) => AiError::Unavailable("the request timed out".to_owned()),
            ureq::Error::HostNotFound => AiError::Unavailable(format!("cannot find the host {host}")),
            ureq::Error::ConnectionFailed => AiError::Unavailable(format!("cannot connect to {host}")),
            ureq::Error::BodyExceedsLimit(_) => AiError::Invalid("the answer is too large".to_owned()),
            ureq::Error::BadUri(_) | ureq::Error::Http(_) => {
                AiError::Config("the base URL or a request header is not valid".to_owned())
            }
            ureq::Error::Io(error) => AiError::Unavailable(self.redact(&format!("cannot reach {host}: {error}"))),
            other => AiError::Unavailable(self.redact(&format!("cannot reach {host}: {other}"))),
        }
    }
}

/// Reads `retry-after` in seconds; the HTTP-date form is ignored.
fn parse_retry_after(value: &str) -> Option<Duration> {
    let seconds: f64 = value.trim().parse().ok()?;
    (seconds.is_finite() && seconds >= 0.0).then(|| Duration::from_secs_f64(seconds.min(86_400.0)))
}

/// A readable message from an error body of either API family, or the start of the body.
pub(crate) fn error_message(body: &str) -> String {
    let parsed: Option<Value> = serde_json::from_str(body).ok();
    let field = |value: &Value| value.as_str().map(str::to_owned);
    let message = parsed.as_ref().and_then(|json| {
        json.pointer("/error/message")
            .and_then(field)
            .or_else(|| json.pointer("/detail/message").and_then(field))
            .or_else(|| json.get("error").and_then(field))
            .or_else(|| json.get("message").and_then(field))
            .or_else(|| json.get("detail").and_then(field))
    });
    let text = message.unwrap_or_else(|| body.to_owned());
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.is_empty() {
        "no details given".to_owned()
    } else {
        flat.chars().take(MAX_MESSAGE_CHARS).collect()
    }
}

/// Whether a base URL points at this computer.
pub(crate) fn is_loopback(base_url: &str) -> bool {
    let rest = base_url.split("://").nth(1).unwrap_or(base_url);
    let authority = rest.split('/').next().unwrap_or_default();
    let host = match authority.strip_prefix('[') {
        Some(bracketed) => bracketed.split(']').next().unwrap_or_default(),
        None => authority.split(':').next().unwrap_or_default(),
    };
    host.eq_ignore_ascii_case("localhost") || host.parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

/// Whether an error message mentions any of `words`, ignoring case.
pub(crate) fn mentions(error: &AiError, words: &[&str]) -> bool {
    let message = error.message().to_ascii_lowercase();
    words.iter().any(|word| message.contains(word))
}
