// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the request body limit over real loopback connections: 413 for declared
//! bodies before any of them is read, bodies at the limit, clients that declare more
//! than they send, and rejected requests whose body never arrives.

mod common;

use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use common::{TestServer, connect};

const LIMIT: usize = 256 * 1024;

/// Sends a request head with these extra header lines and no body.
fn send_head(server: &TestServer, request_line: &str, headers: &str) -> TcpStream {
    let mut stream = connect(server.port);
    let head = format!(
        "{request_line}\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n{headers}\r\n",
        server.port
    );
    stream.write_all(head.as_bytes()).unwrap();
    stream
}

/// Sends only a request head that declares a body of `length` bytes.
fn declare_body(server: &TestServer, token: &str, length: u64) -> TcpStream {
    let headers = format!("X-Sceneuron-Token: {token}\r\nContent-Length: {length}\r\n");
    send_head(server, "POST /api/override HTTP/1.1", &headers)
}

#[test]
fn bodies_over_the_limit_are_rejected() {
    let server = TestServer::start();
    let oversized = vec![b' '; LIMIT + 1];
    let reply = server
        .api("POST", "/api/override")
        .body(oversized.clone())
        .send(server.port);
    assert_eq!(reply.status, 413, "{}", reply.text());
    assert!(reply.error().contains("256 KiB"));
    let reply = server.api("PUT", "/api/project").body(oversized).send(server.port);
    assert_eq!(reply.status, 413);

    let at_limit = vec![b' '; LIMIT];
    let reply = server.api("POST", "/api/override").body(at_limit).send(server.port);
    assert_eq!(reply.status, 400, "a body at the limit is read and parsed");
}

#[test]
fn clients_that_declare_more_than_they_send_are_harmless() {
    let server = TestServer::start();
    let declared = [
        100_000_000_000_000,
        u64::from(u32::MAX),
        17 * 1024 * 1024,
        4 * LIMIT as u64,
        2_000,
    ];
    for length in declared {
        for token in [server.token.as_str(), "wrong"] {
            let stream = declare_body(&server, token, length);
            stream.shutdown(Shutdown::Both).unwrap();
        }
    }
    thread::sleep(Duration::from_millis(200));
    let reply = server.api("GET", "/api/state").send(server.port);
    assert_eq!(reply.status, 200);
}

#[test]
fn an_unsent_body_on_a_rejected_request_does_not_stall_other_requests() {
    let server = TestServer::start();
    let token = format!("X-Sceneuron-Token: {}\r\n", server.token);
    let mut stalled = [
        declare_body(&server, "wrong", 2_000),
        send_head(
            &server,
            "GET /api/state HTTP/1.1",
            &format!("{token}Content-Length: 2000\r\n"),
        ),
        send_head(
            &server,
            "POST /api/simulate HTTP/1.1",
            "Expect: 100-continue\r\nContent-Length: 20\r\n",
        ),
    ];
    let started = Instant::now();
    let reply = server.api("GET", "/api/state").send(server.port);
    assert_eq!(reply.status, 200);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "took {:?}",
        started.elapsed()
    );
    for (stream, expected) in stalled
        .iter_mut()
        .zip([b"HTTP/1.1 401", b"HTTP/1.1 200", b"HTTP/1.1 401"])
    {
        let mut head = [0u8; 12];
        stream.read_exact(&mut head).unwrap();
        assert_eq!(&head, expected);
    }
}

#[test]
fn an_oversized_upload_is_refused_before_its_body_is_sent() {
    let server = TestServer::start();
    let mut slow = declare_body(&server, &server.token, 4 * LIMIT as u64);
    slow.write_all(b"{").unwrap();
    let started = Instant::now();
    let reply = server.api("GET", "/api/state").send(server.port);
    assert_eq!(reply.status, 200);
    assert!(started.elapsed() < Duration::from_secs(2));
    let mut reply = String::new();
    slow.read_to_string(&mut reply).unwrap();
    assert!(reply.starts_with("HTTP/1.1 413 "), "{reply}");
    assert!(reply.contains("\r\nConnection: close\r\n"), "{reply}");
}

#[test]
fn an_absurd_declared_length_is_refused_at_once_and_its_connection_freed() {
    let server = TestServer::start();
    for length in [1u64 << 40, u64::MAX] {
        let mut refused = declare_body(&server, &server.token, length);
        let started = Instant::now();
        let mut reply = String::new();
        refused.read_to_string(&mut reply).unwrap();
        assert!(reply.starts_with("HTTP/1.1 413 "), "{reply}");
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "took {:?}",
            started.elapsed()
        );
        drop(refused);
        assert!(
            server.connections_settle_at(0),
            "{} still open",
            server.open_connections()
        );
    }
    let beyond_u64 = format!(
        "X-Sceneuron-Token: {}\r\nContent-Length: 99999999999999999999999999\r\n",
        server.token
    );
    let mut refused = send_head(&server, "POST /api/override HTTP/1.1", &beyond_u64);
    let mut reply = String::new();
    refused.read_to_string(&mut reply).unwrap();
    assert!(reply.starts_with("HTTP/1.1 413 "), "{reply}");
}
