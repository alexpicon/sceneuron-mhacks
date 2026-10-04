// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Reads a server-sent event stream incrementally, with timeouts.

use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use serde_json::Value;

use super::client::{Req, connect, decode_chunks, find, parse_head};

/// One item read from an event stream.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Event {
        name: String,
        data: String,
    },
    Comment(String),
    /// The server ended the stream.
    End,
}

/// Reads an event stream incrementally.
pub struct EventStream {
    stream: TcpStream,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    raw: Vec<u8>,
    text: Vec<u8>,
    chunked: bool,
    ended: bool,
}

impl EventStream {
    pub fn open(port: u16, target: &str, version: &'static str) -> Self {
        Self::open_request(port, &Req::new("GET", target).version(version))
    }

    /// Sends `request` and reads the response head.
    pub fn open_request(port: u16, request: &Req) -> Self {
        let mut stream = connect(port);
        stream.write_all(request.head(port).as_bytes()).unwrap();
        let mut raw = Vec::new();
        let mut buffer = [0u8; 4096];
        let end = loop {
            if let Some(end) = find(&raw, b"\r\n\r\n") {
                break end;
            }
            let read = stream.read(&mut buffer).unwrap();
            assert!(read > 0, "connection closed before the response head");
            raw.extend_from_slice(&buffer[..read]);
        };
        let (status, headers) = parse_head(&raw[..end]);
        let chunked = headers
            .iter()
            .any(|(name, value)| name.eq_ignore_ascii_case("Transfer-Encoding") && value == "chunked");
        Self {
            stream,
            status,
            headers,
            raw: raw[end + 4..].to_vec(),
            text: Vec::new(),
            chunked,
            ended: false,
        }
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(field, _)| field.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// The next event or comment, `Some(Item::End)` once the stream ended, or `None`
    /// when nothing complete arrived within `timeout`.
    pub fn next(&mut self, timeout: Duration) -> Option<Item> {
        let deadline = Instant::now() + timeout;
        loop {
            self.decode();
            if let Some(item) = self.take_item() {
                return Some(item);
            }
            if self.ended {
                return Some(Item::End);
            }
            let now = Instant::now();
            if now >= deadline {
                return None;
            }
            self.stream.set_read_timeout(Some(deadline - now)).unwrap();
            let mut buffer = [0u8; 8192];
            match self.stream.read(&mut buffer) {
                Ok(0) => self.ended = true,
                Ok(read) => self.raw.extend_from_slice(&buffer[..read]),
                Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => return None,
                Err(_) => self.ended = true,
            }
        }
    }

    /// The next `state` event's snapshot, skipping comments.
    pub fn next_state(&mut self, timeout: Duration) -> Option<Value> {
        self.next_event("state", timeout)
    }

    /// The next `control` event's view, skipping comments.
    pub fn next_control(&mut self, timeout: Duration) -> Option<Value> {
        self.next_event("control", timeout)
    }

    /// The data of the next event, which must be called `expected`, skipping comments.
    fn next_event(&mut self, expected: &str, timeout: Duration) -> Option<Value> {
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.next(left)? {
                Item::Event { name, data } => {
                    assert_eq!(name, expected);
                    return Some(serde_json::from_str(&data).unwrap());
                }
                Item::Comment(_) => {}
                Item::End => return None,
            }
        }
    }

    /// Reads until the stream ends; true if it ended within `timeout`.
    pub fn ends_within(&mut self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.next(left) {
                Some(Item::End) => return true,
                Some(_) => {}
                None => return false,
            }
        }
    }

    fn decode(&mut self) {
        if self.chunked {
            if decode_chunks(&mut self.raw, &mut self.text) {
                self.ended = true;
            }
        } else {
            self.text.append(&mut self.raw);
        }
    }

    fn take_item(&mut self) -> Option<Item> {
        let end = find(&self.text, b"\n\n")?;
        let block: Vec<u8> = self.text.drain(..end + 2).collect();
        let block = String::from_utf8(block).unwrap();
        let mut name = String::from("message");
        let mut data = Vec::new();
        for line in block.trim_end_matches('\n').lines() {
            if let Some(comment) = line.strip_prefix(':') {
                return Some(Item::Comment(comment.trim().to_owned()));
            }
            let (field, value) = line.split_once(": ").unwrap_or((line, ""));
            match field {
                "event" => name = value.to_owned(),
                "data" => data.push(value.to_owned()),
                _ => {}
            }
        }
        Some(Item::Event {
            name,
            data: data.join("\n"),
        })
    }
}
