// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Request heads: the request line, the header fields, and the body framing, parsed
//! within the server's limits. Lines may end with CRLF or with a bare LF.

use crate::http::Reply;

/// Longest request line plus header fields, including the empty line that ends them.
pub(crate) const MAX_HEAD: usize = 16 * 1024;

/// Most header fields one request may carry.
const MAX_HEADERS: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Version {
    Http10,
    Http11,
}

/// A parsed request line and its header fields. It has no `Debug`, because its fields
/// can hold the access token.
pub(crate) struct Head {
    pub(crate) method: String,
    /// The request target as sent, such as `/api/state?token=...`.
    pub(crate) target: String,
    pub(crate) version: Version,
    fields: Vec<(String, String)>,
    /// The declared body length; a request without `Content-Length` has an empty body.
    pub(crate) content_length: u64,
}

impl Head {
    /// Values of every header field with this name, ignoring case.
    pub(crate) fn header_values(&self, name: &str) -> impl Iterator<Item = &str> {
        self.fields
            .iter()
            .filter(move |(field, _)| field.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Whether the client waits for `100 Continue` before it sends the body.
    pub(crate) fn expects_continue(&self) -> bool {
        self.version == Version::Http11
            && self
                .header_values("Expect")
                .any(|value| value.eq_ignore_ascii_case("100-continue"))
    }
}

/// Finds the end of a head while its bytes arrive, without scanning any byte twice.
#[derive(Debug, Default)]
pub(crate) struct HeadEnd {
    scanned: usize,
    line_start: usize,
}

impl HeadEnd {
    /// The length of the head, once `received` holds the empty line that ends it.
    pub(crate) fn find(&mut self, received: &[u8]) -> Option<usize> {
        while self.scanned < received.len() {
            let index = self.scanned;
            self.scanned += 1;
            if received[index] != b'\n' {
                continue;
            }
            let line = &received[self.line_start..index];
            if line.is_empty() || line == b"\r" {
                return Some(index + 1);
            }
            self.line_start = index + 1;
        }
        None
    }
}

pub(crate) fn too_large() -> Reply {
    Reply::error(431, &format!("request heads are limited to {} KiB", MAX_HEAD / 1024))
}

/// Parses a complete head, as delimited by [`HeadEnd`].
pub(crate) fn parse(head: &[u8]) -> Result<Head, Reply> {
    let mut lines = head
        .split(|&byte| byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line));
    let (method, target, version) = request_line(lines.next().unwrap_or_default())?;
    let mut fields = Vec::new();
    for line in lines.take_while(|line| !line.is_empty()) {
        if fields.len() == MAX_HEADERS {
            let message = format!("requests are limited to {MAX_HEADERS} header fields");
            return Err(Reply::error(431, &message));
        }
        fields.push(field(line)?);
    }
    let mut head = Head {
        method,
        target,
        version,
        fields,
        content_length: 0,
    };
    if head.header_values("Transfer-Encoding").next().is_some() {
        let message = "Transfer-Encoding is not supported; send the body with Content-Length";
        return Err(Reply::error(501, message));
    }
    head.content_length = content_length(&head)?;
    Ok(head)
}

fn request_line(line: &[u8]) -> Result<(String, String, Version), Reply> {
    let malformed = || Reply::error(400, "the request line is malformed");
    let mut parts = line.split(|&byte| byte == b' ');
    let (Some(method), Some(target), Some(version), None) = (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(malformed());
    };
    let target_ok = !target.is_empty() && target.iter().all(|byte| (b'!'..=b'~').contains(byte));
    if !is_token(method) || !target_ok {
        return Err(malformed());
    }
    let version = match version {
        [b'H', b'T', b'T', b'P', b'/', major, b'.', minor] if major.is_ascii_digit() && minor.is_ascii_digit() => {
            match (major, minor) {
                (b'1', b'0') => Version::Http10,
                // RFC 9110 reads a later minor version as the latest one the server knows.
                (b'1', _) => Version::Http11,
                _ => return Err(Reply::error(505, "only HTTP/1.0 and HTTP/1.1 are supported")),
            }
        }
        _ => return Err(malformed()),
    };
    Ok((ascii(method), ascii(target), version))
}

/// A `name: value` line. Whitespace before the colon and folded lines are refused, as
/// RFC 9112 asks of servers.
fn field(line: &[u8]) -> Result<(String, String), Reply> {
    let malformed = || Reply::error(400, "a header field is malformed");
    let colon = line.iter().position(|&byte| byte == b':').ok_or_else(malformed)?;
    let name = &line[..colon];
    let value = trim_blanks(&line[colon + 1..]);
    let control = |&byte: &u8| byte != b'\t' && (byte < b' ' || byte == 0x7f);
    if !is_token(name) || value.iter().any(control) {
        return Err(malformed());
    }
    Ok((ascii(name), String::from_utf8_lossy(value).into_owned()))
}

/// Every `Content-Length` value must be the same run of digits. A number too large for
/// `u64` is still a valid declaration, and it is refused later as too large.
fn content_length(head: &Head) -> Result<u64, Reply> {
    let invalid = || Reply::error(400, "the Content-Length header is invalid");
    let mut length = None;
    for value in head.header_values("Content-Length").flat_map(|value| value.split(',')) {
        let value = value.trim_matches([' ', '\t']);
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(invalid());
        }
        let parsed = value.parse().unwrap_or(u64::MAX);
        if length.is_some_and(|known| known != parsed) {
            return Err(invalid());
        }
        length = Some(parsed);
    }
    Ok(length.unwrap_or(0))
}

/// An RFC 9110 token, the syntax of methods and field names.
fn is_token(bytes: &[u8]) -> bool {
    let token_char = |byte: &u8| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(byte);
    !bytes.is_empty() && bytes.iter().all(token_char)
}

fn trim_blanks(mut bytes: &[u8]) -> &[u8] {
    while let [b' ' | b'\t', rest @ ..] = bytes {
        bytes = rest;
    }
    while let [rest @ .., b' ' | b'\t'] = bytes {
        bytes = rest;
    }
    bytes
}

/// Bytes already checked to be printable ASCII.
fn ascii(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}
