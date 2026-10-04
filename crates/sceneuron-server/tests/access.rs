// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises who may reach the API: the read-only overlay token, one-time editor links,
//! the IPv6 loopback listener that keeps `localhost` from being squatted, and token checks
//! at startup.

mod common;

use std::io::{Read, Write};
use std::net::{Ipv6Addr, TcpListener, TcpStream};
use std::sync::Arc;

use common::{Reply, Req, TestServer};
use sceneuron_core::default_project;
use sceneuron_server::{Hub, HubConfig, NullAssistant, NullHost, NullRoom, Server, ServerConfig, ServerError};

fn overlay_token(server: &TestServer) -> String {
    let url = server.server.as_ref().unwrap().overlay_url();
    url.split("token=").nth(1).unwrap().to_owned()
}

fn with_overlay_token(server: &TestServer, method: &str, target: &str) -> Reply {
    Req::new(method, target)
        .header("X-Sceneuron-Token", &overlay_token(server))
        .send(server.port)
}

#[test]
fn the_overlay_token_only_reads_state_and_events() {
    let server = TestServer::start();
    let token = overlay_token(&server);
    assert_ne!(token, server.token);
    assert_eq!(with_overlay_token(&server, "GET", "/api/state").status, 200);
    let mut events = common::EventStream::open(server.port, &format!("/api/events?token={token}"), "1.1");
    assert!(events.next_state(std::time::Duration::from_secs(3)).is_some());
    for (method, target) in [
        ("GET", "/api/project"),
        ("PUT", "/api/project"),
        ("POST", "/api/override"),
        ("POST", "/api/automation"),
        ("GET", "/api/sources"),
        ("GET", "/api/camera/preview"),
        ("GET", "/api/camera/analysis"),
        ("GET", "/api/audio-sources"),
        ("POST", "/api/audio-sources/attach"),
        ("GET", "/api/overlay-url"),
        ("GET", "/api/ai"),
    ] {
        let reply = with_overlay_token(&server, method, target);
        assert_eq!(reply.status, 403, "{method} {target}: {}", reply.text());
    }
}

#[test]
fn the_editor_learns_the_overlay_address_from_the_server() {
    let server = TestServer::start();
    let url = server.get_json("/api/overlay-url")["url"].as_str().unwrap().to_owned();
    assert_eq!(url, server.server.as_ref().unwrap().overlay_url());
    assert!(url.starts_with(&format!("http://127.0.0.1:{}/overlay#token=", server.port)));
    assert!(!url.contains(&server.token));
}

#[test]
fn an_editor_link_works_once_and_never_shows_the_token_itself() {
    let server = TestServer::start();
    let link = server.server.as_ref().unwrap().editor_link();
    assert!(!link.contains(&server.token));
    let target = link.split_once(&format!(":{}", server.port)).unwrap().1.to_owned();
    let first = Req::new("GET", &target).send(server.port);
    assert_eq!(first.status, 302);
    assert_eq!(
        first.header("Location"),
        Some(format!("/editor#token={}", server.token).as_str())
    );
    let again = Req::new("GET", &target).send(server.port);
    assert_eq!(again.status, 403, "{}", again.text());
    assert_eq!(Req::new("GET", "/editor?code=guess").send(server.port).status, 403);
}

fn ipv6_loopback_available() -> bool {
    TcpListener::bind((Ipv6Addr::LOCALHOST, 0)).is_ok()
}

fn get_over_ipv6(port: u16, host: &str, token: &str) -> u16 {
    let mut stream = TcpStream::connect((Ipv6Addr::LOCALHOST, port)).unwrap();
    let head = format!("GET /api/state HTTP/1.1\r\nHost: {host}\r\nX-Sceneuron-Token: {token}\r\n\r\n");
    stream.write_all(head.as_bytes()).unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    Reply::parse(&raw).status
}

#[test]
fn the_server_also_holds_ipv6_loopback_for_localhost() {
    if !ipv6_loopback_available() {
        return;
    }
    let server = TestServer::start();
    let port = server.port;
    assert_eq!(get_over_ipv6(port, &format!("localhost:{port}"), &server.token), 200);
    assert_eq!(get_over_ipv6(port, &format!("[::1]:{port}"), &server.token), 200);
    let local = Req::new("GET", "/api/state")
        .host(Some(&format!("localhost:{port}")))
        .header("X-Sceneuron-Token", &server.token);
    assert_eq!(local.send(port).status, 200);
}

#[test]
fn localhost_is_refused_when_another_program_holds_ipv6_loopback() {
    if !ipv6_loopback_available() {
        return;
    }
    let squatter = TcpListener::bind((Ipv6Addr::LOCALHOST, 0)).unwrap();
    let port = squatter.local_addr().unwrap().port();
    let hub = Hub::start(default_project(), HubConfig::default()).unwrap();
    let config = ServerConfig {
        port,
        ..ServerConfig::default()
    };
    let token = config.token.clone();
    let Ok(server) = Server::start(
        Arc::clone(&hub),
        Arc::new(NullHost),
        Arc::new(NullAssistant),
        Arc::new(NullRoom),
        config,
    ) else {
        // The IPv4 side of this port was taken by chance; nothing to check.
        return;
    };
    let request = |host: &str| {
        Req::new("GET", "/api/state")
            .host(Some(host))
            .header("X-Sceneuron-Token", &token)
            .send(port)
    };
    assert_eq!(request(&format!("localhost:{port}")).status, 403);
    assert_eq!(request(&format!("127.0.0.1:{port}")).status, 200);
    server.stop();
    hub.shutdown();
}

#[test]
fn startup_rejects_a_bad_or_reused_overlay_token() {
    let hub = Hub::start(default_project(), HubConfig::default()).unwrap();
    let same = ServerConfig::default();
    let same = ServerConfig {
        overlay_token: same.token.clone(),
        ..same
    };
    let short = ServerConfig {
        overlay_token: "short".to_owned(),
        ..ServerConfig::default()
    };
    for config in [same, short] {
        let started = Server::start(
            Arc::clone(&hub),
            Arc::new(NullHost),
            Arc::new(NullAssistant),
            Arc::new(NullRoom),
            config,
        );
        assert!(matches!(started, Err(ServerError::InvalidToken(_))));
    }
    hub.shutdown();
}

#[test]
fn hello_proves_both_tokens_for_this_port_without_a_token() {
    let server = TestServer::start();
    let nonce = "00112233445566778899aabbccddeeff";
    let reply = Req::new("GET", &format!("/api/hello?nonce={nonce}")).send(server.port);
    assert_eq!(reply.status, 200, "{}", reply.text());
    let body = reply.json();
    let message = format!("{nonce}:{}", server.port);
    assert_eq!(body["editor"], sceneuron_server::proof(&server.token, &message));
    assert_eq!(
        body["overlay"],
        sceneuron_server::proof(&overlay_token(&server), &message)
    );
    for bad in ["short", "zz112233445566778899aabbccddeeff", ""] {
        let reply = Req::new("GET", &format!("/api/hello?nonce={bad}")).send(server.port);
        assert_eq!(reply.status, 400, "{bad}");
    }
    let foreign = Req::new("GET", &format!("/api/hello?nonce={nonce}"))
        .host(Some("evil.example"))
        .send(server.port);
    assert_eq!(foreign.status, 403);
}

#[test]
fn proofs_are_hmac_sha256() {
    // RFC 4231, test case 2.
    assert_eq!(
        sceneuron_server::proof("Jefe", "what do ya want for nothing?"),
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    );
}
