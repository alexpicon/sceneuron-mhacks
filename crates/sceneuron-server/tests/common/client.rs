// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A raw HTTP/1.1 client over `TcpStream` that sends exactly the bytes a test asks for.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use serde_json::Value;

const IO_TIMEOUT: Duration = Duration::from_secs(10);

/// A raw request; it carries `Host: 127.0.0.1:{port}` unless told otherwise.
pub struct Req {
    method: String,
    target: String,
    host: Option<Option<String>>,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
    version: &'static str,
}

impl Req {
    pub fn new(method: &str, target: &str) -> Self {
        Self {
            method: method.to_owned(),
            target: target.to_owned(),
            host: None,
            headers: Vec::new(),
            body: None,
            version: "1.1",
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }

    /// Replaces the Host header; `None` omits it.
    pub fn host(mut self, host: Option<&str>) -> Self {
        self.host = Some(host.map(str::to_owned));
        self
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn version(mut self, version: &'static str) -> Self {
        self.version = version;
        self
    }

    pub(super) fn head(&self, port: u16) -> String {
        let mut head = format!("{} {} HTTP/{}\r\n", self.method, self.target, self.version);
        let default_host = format!("127.0.0.1:{port}");
        let host = match &self.host {
            None => Some(default_host),
            Some(host) => host.clone(),
        };
        if let Some(host) = host {
            head.push_str(&format!("Host: {host}\r\n"));
        }
        head.push_str("Connection: close\r\n");
        for (name, value) in &self.headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        if let Some(body) = &self.body
            && !self
                .headers
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case("Transfer-Encoding"))
        {
            head.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }
        head.push_str("\r\n");
        head
    }

    pub fn send(&self, port: u16) -> Reply {
        let mut stream = connect(port);
        stream.write_all(self.head(port).as_bytes()).unwrap();
        if let Some(body) = &self.body {
            stream.write_all(body).unwrap();
        }
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();
        Reply::parse(&raw)
    }
}

/// Sends `request` exactly as given and reads the whole response.
pub fn send_raw(port: u16, request: &[u8]) -> Reply {
    let mut stream = connect(port);
    stream.write_all(request).unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    Reply::parse(&raw)
}

pub fn connect(port: u16) -> TcpStream {
    let stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.set_read_timeout(Some(IO_TIMEOUT)).unwrap();
    stream.set_write_timeout(Some(IO_TIMEOUT)).unwrap();
    stream
}

#[derive(Debug)]
pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Reply {
    pub fn parse(raw: &[u8]) -> Self {
        let end = find(raw, b"\r\n\r\n").expect("response has no header end");
        let (status, headers) = parse_head(&raw[..end]);
        let rest = &raw[end + 4..];
        let chunked = headers
            .iter()
            .any(|(name, value)| name.eq_ignore_ascii_case("Transfer-Encoding") && value == "chunked");
        let body = if chunked {
            let mut decoded = Vec::new();
            let mut raw = rest.to_vec();
            assert!(decode_chunks(&mut raw, &mut decoded), "chunked body is incomplete");
            decoded
        } else {
            rest.to_vec()
        };
        Self { status, headers, body }
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(field, _)| field.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn text(&self) -> &str {
        std::str::from_utf8(&self.body).unwrap()
    }

    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or_else(|error| panic!("{error}: {}", self.text()))
    }

    /// The message of a `{"error": ...}` body.
    pub fn error(&self) -> String {
        self.json()["error"].as_str().expect("no error message").to_owned()
    }
}

pub(super) fn parse_head(head: &[u8]) -> (u16, Vec<(String, String)>) {
    let head = std::str::from_utf8(head).unwrap();
    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap();
    let status = status_line.split(' ').nth(1).unwrap().parse().unwrap();
    let headers = lines
        .map(|line| {
            let (name, value) = line.split_once(':').unwrap();
            (name.trim().to_owned(), value.trim().to_owned())
        })
        .collect();
    (status, headers)
}

pub(super) fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

/// Moves complete chunks from `raw` to `decoded`; returns true at the final chunk.
pub(super) fn decode_chunks(raw: &mut Vec<u8>, decoded: &mut Vec<u8>) -> bool {
    loop {
        let Some(line_end) = find(raw, b"\r\n") else {
            return false;
        };
        let size_text = std::str::from_utf8(&raw[..line_end]).unwrap();
        let size = usize::from_str_radix(size_text.trim(), 16).unwrap();
        let chunk_end = line_end + 2 + size;
        if raw.len() < chunk_end + 2 {
            return false;
        }
        decoded.extend_from_slice(&raw[line_end + 2..chunk_end]);
        raw.drain(..chunk_end + 2);
        if size == 0 {
            return true;
        }
    }
}
