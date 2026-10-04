// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Server settings and the errors that keep it from starting.

use std::fmt;
use std::io;
use std::time::Duration;

use crate::generate_token;

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const MAX_REQUEST_TIMEOUT: Duration = Duration::from_secs(60 * 60);

/// Where the server listens, the tokens that guard its API, and how long clients may
/// take to send a request.
#[derive(Clone, PartialEq, Eq)]
pub struct ServerConfig {
    /// Loopback port; 0 picks a free one.
    pub port: u16,
    /// The editor's token, which allows every request: 16-128 characters of A-Z, a-z,
    /// 0-9, `-`, `_`, `.`, or `~`.
    pub token: String,
    /// The overlay's token, which only reads state and events, so a browser source that
    /// stores it in a scene collection cannot change anything. Same format; it must differ
    /// from `token`.
    pub overlay_token: String,
    /// The token control surfaces such as a Stream Deck get by pairing: it raises cards,
    /// shows and hides elements, and reads the control view, and nothing else. Same format;
    /// it must differ from both other tokens, and it counts only in the `X-Sceneuron-Token`
    /// header, never in an address. Revoking it replaces it while the server runs.
    pub control_token: String,
    /// How long a client may take to send its request line, headers, and body before it
    /// is disconnected; more than zero and at most an hour.
    pub request_timeout: Duration,
}

impl Default for ServerConfig {
    /// A free port, three new random tokens, and a 10 second request timeout.
    ///
    /// # Panics
    ///
    /// Panics if the operating system cannot provide random bytes for the token.
    fn default() -> Self {
        Self {
            port: 0,
            token: generate_token(),
            overlay_token: generate_token(),
            control_token: generate_token(),
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
        }
    }
}

impl fmt::Debug for ServerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServerConfig")
            .field("port", &self.port)
            .field("token", &"<redacted>")
            .field("overlay_token", &"<redacted>")
            .field("control_token", &"<redacted>")
            .field("request_timeout", &self.request_timeout)
            .finish()
    }
}

/// Why the server could not start.
#[derive(Debug)]
pub enum ServerError {
    /// The token is too short or needs escaping in a URL.
    InvalidToken(String),
    /// The request timeout is zero or longer than an hour.
    InvalidTimeout(Duration),
    /// The port could not be opened.
    Bind(io::Error),
    /// The accept thread could not be started.
    Thread(io::Error),
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerError::InvalidToken(reason) => write!(f, "invalid access token: {reason}"),
            ServerError::InvalidTimeout(timeout) => {
                write!(
                    f,
                    "invalid request timeout {timeout:?}: it must be more than zero and at most an hour"
                )
            }
            ServerError::Bind(error) => write!(f, "cannot listen on 127.0.0.1: {error}"),
            ServerError::Thread(error) => write!(f, "cannot start the server thread: {error}"),
        }
    }
}

impl std::error::Error for ServerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ServerError::InvalidToken(_) | ServerError::InvalidTimeout(_) => None,
            ServerError::Bind(error) | ServerError::Thread(error) => Some(error),
        }
    }
}
