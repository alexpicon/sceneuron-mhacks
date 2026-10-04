// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Responses with the server's security headers, and request target helpers.

use std::borrow::Cow;
use std::fmt::Write;

use serde::Serialize;

use crate::assets::{Asset, PAGE_POLICY};
use crate::{date, to_json};

const JSON: &str = "application/json";

/// A complete response that is not a stream.
#[derive(Debug)]
pub(crate) struct Reply {
    status: u16,
    content_type: Option<&'static str>,
    body: Cow<'static, [u8]>,
    headers: Vec<(&'static str, Cow<'static, str>)>,
}

impl Reply {
    pub(crate) fn json_text(status: u16, body: String) -> Self {
        Self {
            status,
            content_type: Some(JSON),
            body: Cow::Owned(body.into_bytes()),
            headers: Vec::new(),
        }
    }

    pub(crate) fn json<T: Serialize + ?Sized>(status: u16, value: &T) -> Self {
        Self::json_text(status, to_json(value))
    }

    /// An error as `{"error": message}`.
    pub(crate) fn error(status: u16, message: &str) -> Self {
        Self::json(status, &serde_json::json!({ "error": message }))
    }

    /// Binary content, such as audio.
    pub(crate) fn bytes(content_type: &'static str, body: Vec<u8>) -> Self {
        Self {
            status: 200,
            content_type: Some(content_type),
            body: Cow::Owned(body),
            headers: Vec::new(),
        }
    }

    pub(crate) fn asset(asset: &'static Asset) -> Self {
        let reply = Self {
            status: 200,
            content_type: Some(asset.content_type),
            body: Cow::Borrowed(asset.body),
            headers: Vec::new(),
        };
        if asset.page {
            reply.with_header("Content-Security-Policy", PAGE_POLICY)
        } else {
            reply
        }
    }

    pub(crate) fn redirect(location: String) -> Self {
        Self {
            status: 302,
            content_type: None,
            body: Cow::Borrowed(b""),
            headers: vec![("Location", Cow::Owned(location))],
        }
    }

    pub(crate) fn with_header(mut self, name: &'static str, value: impl Into<Cow<'static, str>>) -> Self {
        self.headers.push((name, value.into()));
        self
    }

    /// The complete response. Every response closes its connection.
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut head = format!("HTTP/1.1 {} {}\r\n", self.status, reason(self.status));
        let common = [
            ("Cache-Control", "no-store"),
            ("X-Content-Type-Options", "nosniff"),
            ("Referrer-Policy", "no-referrer"),
        ];
        let content_type = self.content_type.map(|value| ("Content-Type", value));
        let extra = self.headers.iter().map(|(name, value)| (*name, value.as_ref()));
        for (name, value) in common.into_iter().chain(content_type).chain(extra) {
            // A line break in a value would let it add header fields of its own.
            if value.bytes().all(|byte| byte == b'\t' || (b' '..=b'~').contains(&byte)) {
                let _ = write!(head, "{name}: {value}\r\n");
            }
        }
        let _ = write!(
            head,
            "Content-Length: {}\r\nConnection: close\r\nDate: {}\r\n\r\n",
            self.body.len(),
            date::now()
        );
        let mut bytes = head.into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

/// The reason phrase of each status this server sends.
fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        202 => "Accepted",
        302 => "Found",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        413 => "Content Too Large",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        505 => "HTTP Version Not Supported",
        _ => "",
    }
}

/// Splits a request target into its path and query string.
pub(crate) fn split_target(target: &str) -> (&str, &str) {
    target.split_once('?').unwrap_or((target, ""))
}

/// The percent-decoded value of the first query parameter called `name`.
pub(crate) fn query_param(query: &str, name: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if key == name { percent_decode(value) } else { None }
    })
}

fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = text
                .get(index + 1..index + 3)
                .filter(|hex| hex.bytes().all(|byte| byte.is_ascii_hexdigit()))?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}
