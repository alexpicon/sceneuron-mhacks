// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the server's access controls: the Host header allowlist against DNS
//! rebinding, the access token in the header or the query, token generation and
//! comparison, and keeping the token out of debug output.

mod common;

use std::collections::HashSet;

use common::{Req, TestServer};
use sceneuron_server::{ServerConfig, generate_token, tokens_match};

#[test]
fn the_api_requires_the_token() {
    let server = TestServer::start();
    let port = server.port;
    let token = &server.token;
    let wrong = generate_token();
    let refused = [
        Req::new("GET", "/api/state"),
        Req::new("GET", &format!("/api/state?token={wrong}")),
        Req::new("GET", "/api/state?token="),
        Req::new("GET", &format!("/api/state?token={}", &token[..31])),
        Req::new("GET", &format!("/api/state?token={token}x")),
        Req::new("GET", "/api/state").header("X-Sceneuron-Token", &wrong),
        Req::new("GET", &format!("/api/state?token={token}")).header("X-Sceneuron-Token", &wrong),
        Req::new("GET", &format!("/api/state?other={token}")),
        Req::new("GET", "/api/nope"),
        Req::new("GET", "/api/events"),
        Req::new("POST", "/api/simulate").body(r#"{"presence":"away"}"#),
    ];
    for request in refused {
        let reply = request.send(port);
        assert_eq!(reply.status, 401, "{}", reply.text());
        assert_eq!(reply.error(), "a valid access token is required");
        assert_eq!(reply.header("Content-Type"), Some("application/json"));
        assert_eq!(reply.header("Cache-Control"), Some("no-store"));
    }
    assert_eq!(server.hub.snapshot().presence.state, sceneuron_core::Presence::Unknown);

    let accepted = [
        Req::new("GET", &format!("/api/state?token={token}")),
        Req::new("GET", &format!("/api/state?x=1&token={token}&y=2")),
        Req::new(
            "GET",
            &format!("/api/state?token=%{:x}{}", token.as_bytes()[0], &token[1..]),
        ),
        Req::new("GET", "/api/state").header("X-Sceneuron-Token", token),
        Req::new("GET", "/api/state").header("x-sceneuron-token", token),
    ];
    for request in accepted {
        let reply = request.send(port);
        assert_eq!(reply.status, 200, "{}", reply.text());
    }
}

#[test]
fn pages_and_assets_need_no_token() {
    let server = TestServer::start();
    for path in [
        "/editor",
        "/overlay",
        "/assets/render.js",
        "/assets/editor/css/base.css",
    ] {
        assert_eq!(Req::new("GET", path).send(server.port).status, 200, "{path}");
    }
}

#[test]
fn only_loopback_host_headers_for_this_port_are_accepted() {
    let server = TestServer::start();
    let port = server.port;
    let other_port = if port == 65535 { 1 } else { port + 1 };
    let forbidden = [
        Some(format!("evil.example:{port}")),
        Some("evil.example".to_owned()),
        Some("127.0.0.1".to_owned()),
        Some("localhost".to_owned()),
        Some(format!("127.0.0.1:{other_port}")),
        Some(format!("0.0.0.0:{port}")),
        Some(format!("[::1]:{other_port}")),
        Some(format!("127.0.0.1.evil.example:{port}")),
        Some(String::new()),
        None,
    ];
    for host in forbidden {
        for path in ["/editor", "/api/state", "/nope"] {
            let reply = server.api("GET", path).host(host.as_deref()).send(port);
            assert_eq!(reply.status, 403, "{host:?} {path}");
            assert!(reply.error().contains("Host header"));
        }
    }
    let duplicated = server
        .api("GET", "/api/state")
        .header("Host", &format!("evil.example:{port}"))
        .send(port);
    assert_eq!(duplicated.status, 403);
    for host in [format!("127.0.0.1:{port}"), format!("localhost:{port}")] {
        let reply = server.api("GET", "/api/state").host(Some(&host)).send(port);
        assert_eq!(reply.status, 200, "{host}");
    }
}

#[test]
fn generated_tokens_are_long_random_lowercase_hex() {
    let tokens: HashSet<String> = (0..64).map(|_| generate_token()).collect();
    assert_eq!(tokens.len(), 64);
    for token in &tokens {
        assert_eq!(token.len(), 32);
        assert!(
            token.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "{token}"
        );
    }
}

#[test]
fn tokens_match_only_when_identical() {
    let token = generate_token();
    assert!(tokens_match(&token, &token.clone()));
    assert!(tokens_match("", ""));
    assert!(!tokens_match(&token, ""));
    assert!(!tokens_match("", &token));
    assert!(!tokens_match(&token, &token[..31]));
    assert!(!tokens_match(&token, &format!("{token}0")));
    assert!(!tokens_match(&token, &token.to_uppercase()));
    let mut flipped = token.clone().into_bytes();
    flipped[31] = if flipped[31] == b'0' { b'1' } else { b'0' };
    assert!(!tokens_match(&token, std::str::from_utf8(&flipped).unwrap()));
}

#[test]
fn server_config_debug_output_hides_the_token() {
    let token = generate_token();
    let config = ServerConfig {
        port: 4466,
        token: token.clone(),
        ..ServerConfig::default()
    };
    let debug = format!("{config:?}");
    assert!(!debug.contains(&token));
    assert!(debug.contains("4466"));
}
