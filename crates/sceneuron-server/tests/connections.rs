// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises connection handling over real loopback connections: connections released
//! after every kind of request, the connection limit (503) and idle sockets making room,
//! the head and request timeouts for slow clients, clients that leave mid-request, event streams that disconnect, and stopping
//! with streams and idle connections open.

mod common;

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::time::{Duration, Instant};

use common::{Req, TestServer, connect, wait_until};

const WAIT: Duration = Duration::from_secs(3);

/// Reads until the server closes the connection; what arrived, and how long that took.
fn read_until_closed(stream: &mut TcpStream) -> (String, Duration) {
    let started = Instant::now();
    // A socket the server already reset may refuse new options; reading then ends at once.
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut received = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => received.extend_from_slice(&buffer[..read]),
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                panic!("the server kept the connection open")
            }
            Err(_) => break,
        }
    }
    (String::from_utf8_lossy(&received).into_owned(), started.elapsed())
}

/// Sends a request head that declares a body of `length` bytes.
fn declare_body(server: &TestServer, length: usize) -> TcpStream {
    let mut stream = connect(server.port);
    let head = format!(
        "POST /api/override HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nX-Sceneuron-Token: {}\r\nContent-Length: {length}\r\n\r\n",
        server.port, server.token
    );
    stream.write_all(head.as_bytes()).unwrap();
    stream
}

#[test]
fn every_kind_of_request_releases_its_connection() {
    let server = TestServer::start();
    let requests = [
        (server.api("GET", "/api/state"), 200),
        (server.api("GET", "/nope"), 404),
        (Req::new("GET", "/api/state"), 401),
        (Req::new("GET", "/editor").host(Some("evil.example")), 403),
        (server.api("POST", "/api/override").body(vec![b' '; 300 * 1024]), 413),
        (server.api("POST", "/api/override").body("{"), 400),
        (
            server.api("GET", "/api/state").header("X-Pad", &"p".repeat(20_000)),
            431,
        ),
        (
            server
                .api("POST", "/api/automation")
                .header("Transfer-Encoding", "chunked"),
            501,
        ),
    ];
    for (request, status) in requests {
        assert_eq!(request.send(server.port).status, status);
    }
    let mut events = server.events();
    assert!(events.next_state(WAIT).is_some());
    assert_eq!(server.open_connections(), 1);
    drop(events);
    assert!(
        server.connections_settle_at(0),
        "{} still open",
        server.open_connections()
    );
}

#[test]
fn connections_beyond_the_limit_of_active_streams_get_503_at_once() {
    let server = TestServer::start();
    // Event streams are capped at 32, so requests still sending their bodies fill the rest.
    let mut streams: Vec<_> = (0..32).map(|_| server.events()).collect();
    for stream in &mut streams {
        assert!(stream.next_state(WAIT).is_some());
    }
    let _uploads: Vec<TcpStream> = (0..32).map(|_| declare_body(&server, 1000)).collect();
    assert!(server.connections_settle_at(64));
    // Until the server has read an upload's head, that upload is idle and would be closed to
    // make room instead of the next connection getting a 503.
    assert!(wait_until(WAIT, || server.started_connections() == 64));
    let mut refused = connect(server.port);
    let (reply, took) = read_until_closed(&mut refused);
    assert!(reply.starts_with("HTTP/1.1 503 "), "{reply}");
    assert!(reply.contains("\r\nConnection: close\r\n"), "{reply}");
    assert!(reply.contains("too many connections"), "{reply}");
    assert!(took < Duration::from_secs(1), "took {took:?}");
    assert_eq!(server.open_connections(), 64);
    streams.truncate(28);
    assert!(server.connections_settle_at(60));
    assert_eq!(server.api("GET", "/api/state").send(server.port).status, 200);
}

#[test]
fn idle_sockets_make_room_for_real_requests() {
    let server = TestServer::start();
    let mut idle: Vec<TcpStream> = (0..64).map(|_| connect(server.port)).collect();
    assert!(server.connections_settle_at(64));
    assert_eq!(server.api("GET", "/api/state").send(server.port).status, 200);
    let (reply, _) = read_until_closed(&mut idle[0]);
    assert_eq!(reply, "", "the oldest idle socket is closed without an answer");
    let mut events = server.events();
    assert!(events.next_state(WAIT).is_some(), "an event stream also gets a place");
}

#[test]
fn sockets_that_send_nothing_are_closed_after_two_seconds() {
    let server = TestServer::start();
    let started = Instant::now();
    let mut silent = connect(server.port);
    let (reply, took) = read_until_closed(&mut silent);
    assert_eq!(reply, "");
    assert!(
        took >= Duration::from_millis(1900) && took < WAIT,
        "took {:?}",
        started.elapsed()
    );
}

#[test]
fn clients_slower_than_the_request_timeout_are_disconnected() {
    let timeout = Duration::from_millis(300);
    let server = TestServer::with_request_timeout(timeout);
    let started = Instant::now();
    let mut silent = connect(server.port);
    let mut partial = connect(server.port);
    partial
        .write_all(b"GET /api/state HTTP/1.1\r\nHost: 127.0.0.1")
        .unwrap();
    let mut slow_body = declare_body(&server, 100);
    slow_body.write_all(b"{\"element\"").unwrap();

    let (reply, _) = read_until_closed(&mut silent);
    assert_eq!(reply, "", "a client that sent nothing gets no answer");
    let took = started.elapsed();
    assert!(took >= timeout && took < WAIT, "took {took:?}");
    let (reply, _) = read_until_closed(&mut partial);
    assert!(reply.starts_with("HTTP/1.1 408 "), "{reply}");
    let (reply, _) = read_until_closed(&mut slow_body);
    assert!(reply.starts_with("HTTP/1.1 408 "), "{reply}");
    assert!(reply.contains("did not arrive in time"), "{reply}");
    assert!(server.connections_settle_at(0));
}

#[test]
fn trickling_does_not_extend_the_request_timeout() {
    let timeout = Duration::from_millis(300);
    let server = TestServer::with_request_timeout(timeout);
    let started = Instant::now();
    let mut trickling = connect(server.port);
    trickling.write_all(b"GET /api/state HTTP/1.1\r\n").unwrap();
    trickling.set_read_timeout(Some(Duration::from_millis(50))).unwrap();
    let mut buffer = [0u8; 4096];
    loop {
        assert!(started.elapsed() < WAIT, "the trickling client was never disconnected");
        let _ = trickling.write_all(b"X-Slow: 1\r\n");
        match trickling.read(&mut buffer) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(_) => break,
        }
    }
    assert!(started.elapsed() >= timeout, "took {:?}", started.elapsed());
    assert!(server.connections_settle_at(0));
}

#[test]
fn a_client_that_leaves_mid_body_frees_its_connection() {
    let server = TestServer::start();
    let mut gone = declare_body(&server, 1000);
    gone.write_all(b"{\"element\":").unwrap();
    assert!(server.connections_settle_at(1));
    let started = Instant::now();
    drop(gone);
    assert!(server.connections_settle_at(0));
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "took {:?}",
        started.elapsed()
    );

    let mut half_closed = declare_body(&server, 1000);
    half_closed.write_all(b"{\"element\":").unwrap();
    half_closed.shutdown(Shutdown::Write).unwrap();
    let (reply, _) = read_until_closed(&mut half_closed);
    assert!(reply.starts_with("HTTP/1.1 400 "), "{reply}");
    assert!(reply.contains("ended early"), "{reply}");
    drop(half_closed);
    assert!(server.connections_settle_at(0));
}

#[test]
fn a_disconnected_event_stream_frees_its_place_before_the_next_ping() {
    let server = TestServer::start();
    let mut events = server.events();
    assert!(events.next_state(WAIT).is_some());
    assert_eq!(server.open_connections(), 1);
    let started = Instant::now();
    drop(events);
    assert!(server.connections_settle_at(0));
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "took {:?}",
        started.elapsed()
    );
}

#[test]
fn stop_returns_quickly_with_streams_and_idle_connections_open() {
    let mut server = TestServer::start();
    let mut streams: Vec<_> = (0..3).map(|_| server.events()).collect();
    for events in &mut streams {
        assert!(events.next_state(WAIT).is_some());
    }
    let mut clients: Vec<TcpStream> = (0..4).map(|_| connect(server.port)).collect();
    let mut partial = connect(server.port);
    partial.write_all(b"GET /api/state HTTP/1.1\r\n").unwrap();
    clients.push(partial);
    clients.push(declare_body(&server, 500));
    assert!(server.connections_settle_at(3 + 6));
    let port = server.port;

    let took = server.stop();
    assert!(took < Duration::from_secs(1), "stop took {took:?}");
    for events in &mut streams {
        assert!(events.ends_within(WAIT), "a stream stayed open after stop");
    }
    for client in &mut clients {
        let (_, closed_after) = read_until_closed(client);
        assert!(closed_after < WAIT);
    }
    assert!(wait_until(WAIT, || TcpStream::connect(("127.0.0.1", port)).is_err()));
}
