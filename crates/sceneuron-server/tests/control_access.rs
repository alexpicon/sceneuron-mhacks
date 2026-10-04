// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises who may use the control API: the control token's explicit route allowlist and
//! its 403 everywhere else, the editor token reaching every control route, the control token
//! counting only in the header, its hello proof, and its checks at startup.

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{Reply, Req, TestServer, server_with_decks};
use sceneuron_core::default_project;
use sceneuron_server::{
    Hub, HubConfig, NullAssistant, NullHost, NullRoom, Server, ServerConfig, ServerError, generate_token,
};

const NONCE: &str = "00112233445566778899aabbccddeeff";

/// Every request that needs a token, with a body the API accepts.
const ROUTES: &[(&str, &str, &str)] = &[
    ("GET", "/api/state", ""),
    ("GET", "/api/project", ""),
    ("PUT", "/api/project", "{}"),
    ("POST", "/api/override", r#"{"element":"brb","props":{"text":"Back"}}"#),
    ("POST", "/api/release", "{}"),
    ("POST", "/api/effect", r#"{"element":"brb","effect":"pop"}"#),
    ("POST", "/api/flood", r#"{"flood":"tongue"}"#),
    ("POST", "/api/automation", r#"{"enabled":true}"#),
    ("POST", "/api/simulate", r#"{"presence":"away"}"#),
    ("GET", "/api/sources", ""),
    ("POST", "/api/sources/attach", r#"{"uuid":"cam-1"}"#),
    ("GET", "/api/camera/preview", ""),
    ("GET", "/api/camera/analysis", ""),
    ("GET", "/api/audio-sources", ""),
    ("POST", "/api/audio-sources/attach", r#"{"uuid":"mic-1"}"#),
    ("POST", "/api/overlay", ""),
    ("GET", "/api/overlay-url", ""),
    ("GET", "/api/ai", ""),
    ("PUT", "/api/ai", "{}"),
    ("POST", "/api/ai/plan", r#"{"request":"wave"}"#),
    ("POST", "/api/ai/analyze", ""),
    ("GET", "/api/events", ""),
    ("GET", "/api/packs", ""),
    ("POST", "/api/card", r#"{"deck":"poker","card":"p5","action":"toggle"}"#),
    ("POST", "/api/control/element", r#"{"element":"brb","action":"toggle"}"#),
    ("GET", "/api/control/state", ""),
    ("GET", "/api/control/pairing", ""),
    ("POST", "/api/control/pairing", ""),
    ("POST", "/api/control/revoke", ""),
    ("GET", "/api/voice", ""),
    ("POST", "/api/voice/talk", r#"{"down":false}"#),
    ("GET", "/api/voice/guide", ""),
    ("POST", "/api/voice/command", r#"{"text":"wrap it up"}"#),
];

/// The requests a control token may make, besides its event stream.
const CONTROL_ROUTES: &[&str] = &[
    "/api/release",
    "/api/effect",
    "/api/flood",
    "/api/automation",
    "/api/card",
    "/api/control/element",
    "/api/control/state",
    "/api/voice/talk",
];

fn send(server: &TestServer, request: Req, body: &str) -> Reply {
    let request = if body.is_empty() { request } else { request.body(body) };
    request.send(server.port)
}

fn overlay_token(server: &TestServer) -> String {
    let url = server.server.as_ref().unwrap().overlay_url();
    url.split("token=").nth(1).unwrap().to_owned()
}

#[test]
fn the_control_token_reaches_only_the_control_requests() {
    let server = server_with_decks();
    for (method, target, body) in ROUTES {
        let reply = send(&server, server.control(method, target), body);
        if CONTROL_ROUTES.contains(target) {
            assert_eq!(reply.status, 200, "{method} {target}: {}", reply.text());
        } else {
            assert_eq!(reply.status, 403, "{method} {target}: {}", reply.text());
            assert_eq!(
                reply.error(),
                "the control token only controls cards, elements, effects, floods, automation, and push to talk"
            );
        }
    }
    let mut events = server.control_events(&server.control_token);
    assert_eq!(events.status, 200);
    assert!(events.next_control(Duration::from_secs(3)).is_some());
}

#[test]
fn tokenless_requests_refuse_the_control_token_too() {
    let server = TestServer::start();
    let hello = server
        .control("GET", &format!("/api/hello?nonce={NONCE}"))
        .send(server.port);
    assert_eq!(hello.status, 403);
    let pair = server.control("POST", "/api/control/pair").body(r#"{"code":"123456"}"#);
    assert_eq!(pair.send(server.port).status, 403);
    for path in ["/", "/editor", "/overlay", "/assets/render.js"] {
        let reply = server.control("GET", path).send(server.port);
        assert!(matches!(reply.status, 200 | 302), "{path}: {}", reply.status);
    }
}

#[test]
fn the_editor_token_reaches_every_control_request() {
    let server = server_with_decks();
    for target in CONTROL_ROUTES {
        let (method, _, body) = ROUTES.iter().find(|(_, path, _)| path == target).unwrap();
        let reply = send(&server, server.api(method, target), body);
        assert_eq!(reply.status, 200, "{method} {target}: {}", reply.text());
    }
    let mut events = server.control_events(&server.token);
    assert!(events.next_control(Duration::from_secs(3)).is_some());
}

#[test]
fn the_overlay_token_and_no_token_cannot_control() {
    let server = server_with_decks();
    let overlay = overlay_token(&server);
    for (method, target, body) in ROUTES
        .iter()
        .filter(|(_, target, _)| target.contains("control") || *target == "/api/card")
    {
        let reply = send(&server, server.with_token(&overlay, method, target), body);
        assert_eq!(reply.status, 403, "{method} {target}: {}", reply.text());
        let reply = send(&server, Req::new(method, target), body);
        assert_eq!(reply.status, 401, "{method} {target}: {}", reply.text());
    }
    assert_eq!(server.control_events(&overlay).status, 403);
}

#[test]
fn the_control_token_counts_only_in_the_header() {
    let server = server_with_decks();
    let control = &server.control_token;
    let refused = [
        Req::new("GET", &format!("/api/control/state?token={control}")),
        Req::new("GET", &format!("/api/control/events?token={control}")),
        Req::new("POST", &format!("/api/card?token={control}")).body(r#"{"deck":"poker","action":"next"}"#),
        server.with_token(&generate_token(), "GET", &format!("/api/control/state?token={control}")),
    ];
    for request in refused {
        let reply = request.send(server.port);
        assert_eq!(reply.status, 401, "{}", reply.text());
        assert_eq!(reply.error(), "a valid access token is required");
    }
    assert_eq!(server.hub.snapshot().decks[1].raised, None, "no request got through");
    let header = server.control("GET", "/api/control/state").send(server.port);
    assert_eq!(header.status, 200);
    let lowercase = Req::new("GET", "/api/control/state").header("x-sceneuron-token", control);
    assert_eq!(lowercase.send(server.port).status, 200);
    let editor = Req::new("GET", &format!("/api/control/state?token={}", server.token));
    assert_eq!(
        editor.send(server.port).status,
        200,
        "the editor token still works in the query"
    );
}

#[test]
fn hello_also_proves_the_control_token() {
    let server = TestServer::start();
    let reply = Req::new("GET", &format!("/api/hello?nonce={NONCE}")).send(server.port);
    assert_eq!(reply.status, 200, "{}", reply.text());
    let message = format!("{NONCE}:{}", server.port);
    let body = reply.json();
    assert_eq!(
        body["control"],
        sceneuron_server::proof(&server.control_token, &message)
    );
    assert_eq!(body["editor"], sceneuron_server::proof(&server.token, &message));
    assert_ne!(body["control"], body["editor"]);
}

#[test]
fn startup_rejects_a_bad_or_reused_control_token() {
    let hub = Hub::start(default_project(), HubConfig::default()).unwrap();
    let base = ServerConfig::default();
    let configs = [
        ServerConfig {
            control_token: base.token.clone(),
            ..base.clone()
        },
        ServerConfig {
            control_token: base.overlay_token.clone(),
            ..base.clone()
        },
        ServerConfig {
            control_token: "short".to_owned(),
            ..base.clone()
        },
        ServerConfig {
            control_token: "has spaces in it, too many".to_owned(),
            ..base
        },
    ];
    for config in configs {
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
fn server_config_debug_output_hides_the_control_token() {
    let config = ServerConfig::default();
    let debug = format!("{config:?}");
    assert!(!debug.contains(&config.control_token));
    assert!(debug.contains("control_token"));
}
