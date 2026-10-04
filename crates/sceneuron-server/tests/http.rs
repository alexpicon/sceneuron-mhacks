// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the HTTP layer over real loopback connections: pages, routing errors (404,
//! 405), response headers, server configuration, and server lifecycle. Assets have their
//! own tests in `assets.rs`.

mod common;

use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use common::{Req, TestServer, start_server, wait_until};
use sceneuron_core::default_project;
use sceneuron_server::{Hub, HubConfig, ServerConfig, ServerError, generate_token};

const POLICY: &str = "default-src 'none'; script-src 'self'; style-src 'self'; font-src 'self'; \
                      connect-src 'self'; media-src 'self'; img-src data: blob:; base-uri 'none'; \
                      form-action 'none'; frame-ancestors 'none'";

#[test]
fn root_redirects_to_the_editor_keeping_the_query() {
    let server = TestServer::start();
    let reply = Req::new("GET", "/").send(server.port);
    assert_eq!(reply.status, 302);
    assert_eq!(reply.header("Location"), Some("/editor"));
    let reply = Req::new("GET", "/?token=abc&x=1").send(server.port);
    assert_eq!(reply.status, 302);
    assert_eq!(reply.header("Location"), Some("/editor?token=abc&x=1"));
}

#[test]
fn pages_are_served_with_a_content_security_policy() {
    let server = TestServer::start();
    let pages = [
        ("/editor", include_str!("../../../web/editor.html")),
        ("/overlay", include_str!("../../../web/overlay.html")),
    ];
    for (path, body) in pages {
        let reply = Req::new("GET", path).send(server.port);
        assert_eq!(reply.status, 200, "{path}");
        assert_eq!(reply.text(), body, "{path}");
        assert_eq!(reply.header("Content-Type"), Some("text/html; charset=utf-8"));
        assert_eq!(reply.header("Content-Security-Policy"), Some(POLICY));
        assert_eq!(reply.header("X-Content-Type-Options"), Some("nosniff"));
        assert_eq!(reply.header("Cache-Control"), Some("no-store"));
        assert_eq!(reply.header("Referrer-Policy"), Some("no-referrer"));
    }
}

#[test]
fn unknown_paths_are_not_found() {
    let server = TestServer::start();
    for path in [
        "/nope",
        "/editor/",
        "/assets/missing.js",
        "/assets/",
        "/api",
        "/web/editor.html",
    ] {
        let reply = Req::new("GET", path).send(server.port);
        assert_eq!(reply.status, 404, "{path}");
        assert_eq!(reply.header("Content-Type"), Some("application/json"));
        assert_eq!(reply.error(), "not found");
    }
    let reply = server.api("GET", "/api/nope").send(server.port);
    assert_eq!(reply.status, 404);
}

#[test]
fn wrong_methods_are_rejected_with_the_allowed_ones() {
    let server = TestServer::start();
    let cases = [
        ("POST", "/editor", "GET"),
        ("DELETE", "/", "GET"),
        ("PUT", "/assets/render.js", "GET"),
        ("POST", "/api/state", "GET"),
        ("DELETE", "/api/project", "GET, PUT"),
        ("GET", "/api/override", "POST"),
        ("PUT", "/api/release", "POST"),
        ("GET", "/api/effect", "POST"),
        ("GET", "/api/automation", "POST"),
        ("GET", "/api/simulate", "POST"),
        ("POST", "/api/sources", "GET"),
        ("GET", "/api/sources/attach", "POST"),
        ("GET", "/api/overlay", "POST"),
        ("POST", "/api/events", "GET"),
        ("OPTIONS", "/api/state", "GET"),
    ];
    for (method, path, allowed) in cases {
        let reply = server.api(method, path).send(server.port);
        assert_eq!(reply.status, 405, "{method} {path}");
        assert_eq!(reply.header("Allow"), Some(allowed), "{method} {path}");
        assert_eq!(reply.error(), "method not allowed");
    }
}

#[test]
fn urls_name_the_loopback_port_and_token() {
    let hub = Hub::start(default_project(), HubConfig::default()).unwrap();
    let token = generate_token();
    let overlay_token = generate_token();
    let config = ServerConfig {
        port: 0,
        token: token.clone(),
        overlay_token: overlay_token.clone(),
        ..ServerConfig::default()
    };
    let server = start_server(hub, config).unwrap();
    let port = server.port();
    assert_ne!(port, 0);
    assert_eq!(
        server.editor_url(),
        format!("http://127.0.0.1:{port}/editor#token={token}")
    );
    assert_eq!(
        server.overlay_url(),
        format!("http://127.0.0.1:{port}/overlay#token={overlay_token}")
    );
}

#[test]
fn start_rejects_weak_tokens_and_busy_ports() {
    let hub = Hub::start(default_project(), HubConfig::default()).unwrap();
    for token in [
        "",
        "short",
        "has spaces in the token",
        "has&ampersand-1234567",
        &"x".repeat(129),
    ] {
        let config = ServerConfig {
            port: 0,
            token: token.to_owned(),
            ..ServerConfig::default()
        };
        let error = start_server(Arc::clone(&hub), config).err().unwrap();
        assert!(matches!(error, ServerError::InvalidToken(_)), "{token:?}");
    }
    let config = ServerConfig {
        port: 0,
        token: "Custom-token_1.2~ok".to_owned(),
        ..ServerConfig::default()
    };
    let first = start_server(Arc::clone(&hub), config).unwrap();
    let busy = ServerConfig {
        port: first.port(),
        token: generate_token(),
        ..ServerConfig::default()
    };
    let error = start_server(hub, busy).err().unwrap();
    assert!(matches!(error, ServerError::Bind(_)), "{error}");
    assert!(error.to_string().contains("127.0.0.1"));
}

#[test]
fn the_default_config_is_usable_and_timeouts_are_bounded() {
    let config = ServerConfig::default();
    assert_eq!(config.port, 0);
    assert_eq!(config.token.len(), 32);
    assert_ne!(ServerConfig::default().token, config.token);
    assert_eq!(config.request_timeout, Duration::from_secs(10));
    let hub = Hub::start(default_project(), HubConfig::default()).unwrap();
    for request_timeout in [Duration::ZERO, Duration::from_secs(3601), Duration::MAX] {
        let config = ServerConfig {
            request_timeout,
            ..ServerConfig::default()
        };
        let error = start_server(Arc::clone(&hub), config).err().unwrap();
        assert!(matches!(error, ServerError::InvalidTimeout(_)), "{request_timeout:?}");
    }
    let longest = ServerConfig {
        request_timeout: Duration::from_secs(3600),
        ..ServerConfig::default()
    };
    assert!(start_server(hub, longest).is_ok());
}

#[test]
fn stop_closes_the_port_and_drop_stops_too() {
    let mut server = TestServer::start();
    let port = server.port;
    assert!(server.stop() < Duration::from_secs(1));
    assert!(wait_until(Duration::from_secs(2), || TcpStream::connect((
        "127.0.0.1",
        port
    ))
    .is_err()));

    let hub = Hub::start(default_project(), HubConfig::default()).unwrap();
    let config = ServerConfig {
        port: 0,
        token: generate_token(),
        ..ServerConfig::default()
    };
    let dropped = start_server(hub, config).unwrap();
    let port = dropped.port();
    drop(dropped);
    assert!(wait_until(Duration::from_secs(2), || TcpStream::connect((
        "127.0.0.1",
        port
    ))
    .is_err()));
}

#[test]
fn http_1_0_requests_are_answered() {
    let server = TestServer::start();
    let reply = server.api("GET", "/api/state").version("1.0").send(server.port);
    assert_eq!(reply.status, 200);
    assert!(reply.json()["revision"].is_u64());
}
