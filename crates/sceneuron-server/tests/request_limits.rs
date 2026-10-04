// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises request parsing over real loopback connections: the head size and header
//! count limits (431), Transfer-Encoding (501), malformed requests (400), HTTP versions,
//! Content-Length framing, `Expect: 100-continue`, bare LF line endings, and one
//! response per connection.

mod common;

use std::io::{Read, Write};

use common::{Reply, TestServer, connect, send_raw};

const MAX_HEAD: usize = 16 * 1024;

/// A raw request with the server's Host header and token followed by `extra` lines.
fn request(server: &TestServer, request_line: &str, extra: &str, body: &str) -> Vec<u8> {
    let head = format!(
        "{request_line}\r\nHost: 127.0.0.1:{}\r\nX-Sceneuron-Token: {}\r\n{extra}\r\n",
        server.port, server.token
    );
    [head.as_bytes(), body.as_bytes()].concat()
}

fn assert_refused(reply: &Reply, status: u16, context: &str) {
    assert_eq!(reply.status, status, "{context}: {}", reply.text());
    assert_eq!(reply.header("Connection"), Some("close"), "{context}");
    assert_eq!(reply.header("Content-Type"), Some("application/json"), "{context}");
    assert!(!reply.error().is_empty(), "{context}");
}

/// A GET of the state whose head is exactly `size` bytes long.
fn head_of_size(server: &TestServer, size: usize) -> Vec<u8> {
    let bare = request(server, "GET /api/state HTTP/1.1", "X-Pad: \r\n", "");
    let pad = "p".repeat(size - bare.len());
    request(server, "GET /api/state HTTP/1.1", &format!("X-Pad: {pad}\r\n"), "")
}

#[test]
fn heads_over_16_kib_are_refused() {
    let server = TestServer::start();
    let at_limit = head_of_size(&server, MAX_HEAD);
    assert_eq!(at_limit.len(), MAX_HEAD);
    assert_eq!(send_raw(server.port, &at_limit).status, 200);
    for size in [MAX_HEAD + 1, 4 * MAX_HEAD] {
        let reply = send_raw(server.port, &head_of_size(&server, size));
        assert_refused(&reply, 431, &format!("{size} bytes"));
        assert!(reply.error().contains("16 KiB"));
    }
    let long_target = format!("GET /api/state?{} HTTP/1.1", "x".repeat(MAX_HEAD));
    assert_refused(
        &send_raw(server.port, &request(&server, &long_target, "", "")),
        431,
        "target",
    );
}

#[test]
fn more_than_100_header_fields_are_refused() {
    let server = TestServer::start();
    let fields = |count: usize| {
        (0..count)
            .map(|index| format!("X-Field-{index}: {index}\r\n"))
            .collect::<String>()
    };
    // Host and the token are two of the fields.
    let at_limit = request(&server, "GET /api/state HTTP/1.1", &fields(98), "");
    assert_eq!(send_raw(server.port, &at_limit).status, 200);
    let over = request(&server, "GET /api/state HTTP/1.1", &fields(99), "");
    let reply = send_raw(server.port, &over);
    assert_refused(&reply, 431, "101 fields");
    assert!(reply.error().contains("100 header fields"));
}

#[test]
fn transfer_encoding_is_not_implemented() {
    let server = TestServer::start();
    let chunked = "11\r\n{\"enabled\":false}\r\n0\r\n\r\n";
    let framings = [
        "Transfer-Encoding: chunked\r\n",
        "Transfer-Encoding: gzip, chunked\r\n",
        "transfer-encoding: identity\r\n",
        "Transfer-Encoding: chunked\r\nContent-Length: 27\r\n",
    ];
    for framing in framings {
        let raw = request(&server, "POST /api/automation HTTP/1.1", framing, chunked);
        assert_refused(&send_raw(server.port, &raw), 501, framing);
    }
    assert!(server.hub.snapshot().automation_enabled, "a refused body was applied");
}

#[test]
fn malformed_requests_are_bad_requests() {
    let server = TestServer::start();
    let lines = [
        ("GET /api/state", ""),
        ("GET  /api/state HTTP/1.1", ""),
        ("GET /api/state HTTP/1.1 extra", ""),
        ("GET /api/state  HTTP/1.1", ""),
        ("G(T /api/state HTTP/1.1", ""),
        ("GET /api/st\u{1}ate HTTP/1.1", ""),
        ("GET /caf\u{e9} HTTP/1.1", ""),
        ("GET /api/state HTTP/1.1x", ""),
        ("GET /api/state http/1.1", ""),
        ("GET /api/state HTTP/1", ""),
        ("", ""),
        ("GET /api/state HTTP/1.1", "No colon here\r\n"),
        ("GET /api/state HTTP/1.1", "X-Spaced : value\r\n"),
        ("GET /api/state HTTP/1.1", ": no name\r\n"),
        ("GET /api/state HTTP/1.1", "X-Folded: one\r\n two\r\n"),
        ("GET /api/state HTTP/1.1", "X-Null: a\u{0}b\r\n"),
        ("GET /api/state HTTP/1.1", "X-Cr: a\rb\r\n"),
        ("GET /api/state HTTP/1.1", "Content-Length: abc\r\n"),
        ("GET /api/state HTTP/1.1", "Content-Length: -1\r\n"),
        ("GET /api/state HTTP/1.1", "Content-Length: +5\r\n"),
        ("GET /api/state HTTP/1.1", "Content-Length: 1 2\r\n"),
        ("GET /api/state HTTP/1.1", "Content-Length:\r\n"),
        ("GET /api/state HTTP/1.1", "Content-Length: 5\r\nContent-Length: 6\r\n"),
        ("GET /api/state HTTP/1.1", "Content-Length: 5, 6\r\n"),
    ];
    for (line, extra) in lines {
        let reply = send_raw(server.port, &request(&server, line, extra, ""));
        assert_refused(&reply, 400, &format!("{line:?} {extra:?}"));
    }
    let mut unfinished = connect(server.port);
    unfinished.write_all(b"GET /api/state HTTP/1.1\r\nHost: x").unwrap();
    unfinished.shutdown(std::net::Shutdown::Write).unwrap();
    let mut reply = String::new();
    unfinished.read_to_string(&mut reply).unwrap();
    assert!(reply.starts_with("HTTP/1.1 400 "), "{reply}");
}

#[test]
fn only_http_1_is_served() {
    let server = TestServer::start();
    for version in ["HTTP/2.0", "HTTP/0.9", "HTTP/3.1"] {
        let raw = request(&server, &format!("GET /api/state {version}"), "", "");
        assert_refused(&send_raw(server.port, &raw), 505, version);
    }
    for version in ["HTTP/1.0", "HTTP/1.1", "HTTP/1.2"] {
        let raw = request(&server, &format!("GET /api/state {version}"), "", "");
        assert_eq!(send_raw(server.port, &raw).status, 200, "{version}");
    }
}

#[test]
fn content_length_frames_the_body() {
    let server = TestServer::start();
    let body = r#"{"enabled":false}"#;
    let unframed = request(&server, "POST /api/automation HTTP/1.1", "", body);
    let reply = send_raw(server.port, &unframed);
    assert_eq!(reply.status, 400, "without Content-Length the body is empty");
    assert!(reply.error().contains("EOF"), "{}", reply.error());
    assert!(server.hub.snapshot().automation_enabled);
    for framing in [
        "Content-Length: 17\r\nContent-Length: 17\r\n",
        "Content-Length: 17, 17\r\n",
        "Content-Length: 0017\r\n",
        "content-length:17\r\n",
    ] {
        let raw = request(&server, "POST /api/automation HTTP/1.1", framing, body);
        assert_eq!(send_raw(server.port, &raw).status, 200, "{framing}");
    }
    assert!(!server.hub.snapshot().automation_enabled);
}

#[test]
fn a_client_expecting_100_continue_gets_it_before_sending_the_body() {
    let server = TestServer::start();
    let body = r#"{"presence":"away"}"#;
    let extra = format!("Expect: 100-continue\r\nContent-Length: {}\r\n", body.len());
    let mut stream = connect(server.port);
    stream
        .write_all(&request(&server, "POST /api/simulate HTTP/1.1", &extra, ""))
        .unwrap();
    let mut interim = [0u8; 25];
    stream.read_exact(&mut interim).unwrap();
    assert_eq!(&interim, b"HTTP/1.1 100 Continue\r\n\r\n");
    stream.write_all(body.as_bytes()).unwrap();
    let mut reply = String::new();
    stream.read_to_string(&mut reply).unwrap();
    assert!(reply.starts_with("HTTP/1.1 200 OK\r\n"), "{reply}");
    assert_eq!(server.hub.snapshot().presence.state, sceneuron_core::Presence::Away);
}

#[test]
fn bare_lf_line_endings_are_accepted() {
    let server = TestServer::start();
    let raw = format!(
        "GET /api/state HTTP/1.1\nHost: 127.0.0.1:{}\nX-Sceneuron-Token: {}\n\n",
        server.port, server.token
    );
    assert_eq!(send_raw(server.port, raw.as_bytes()).status, 200);
}

#[test]
fn each_connection_gets_one_response_and_is_closed() {
    let server = TestServer::start();
    let one = request(&server, "GET /api/state HTTP/1.1", "", "");
    let pipelined = [one.clone(), one].concat();
    let mut stream = connect(server.port);
    stream.write_all(&pipelined).unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let text = String::from_utf8_lossy(&raw);
    assert_eq!(text.matches("HTTP/1.1 ").count(), 1, "{text}");
    let reply = send_raw(server.port, &request(&server, "GET /api/state HTTP/1.1", "", ""));
    assert_eq!(reply.header("Connection"), Some("close"));
    assert!(reply.header("Date").is_some_and(|date| date.ends_with(" GMT")));
    let length: usize = reply.header("Content-Length").unwrap().parse().unwrap();
    assert_eq!(length, reply.body.len());
}
