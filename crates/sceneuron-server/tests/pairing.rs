// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises pairing and revoking: codes from the editor traded once for the control token,
//! refusal of web pages, the attempt limit, the count of traded codes the editor watches, and
//! revoking, which replaces the token through the host, keeps it when saving fails, and drops
//! the pairing code. Code expiry is covered by the unit tests next to the codes.

mod common;

use std::sync::atomic::Ordering;

use common::{Reply, Req, TestServer};
use serde_json::json;

fn issue_code(server: &TestServer) -> String {
    let reply = server.api("POST", "/api/control/pairing").send(server.port);
    assert_eq!(reply.status, 200, "{}", reply.text());
    let body = reply.json();
    assert_eq!(body["expires_in_s"], 120);
    let code = body["code"].as_str().unwrap().to_owned();
    assert!(
        code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit()),
        "{code}"
    );
    code
}

fn pair(server: &TestServer, code: &str) -> Reply {
    pair_with(server, Req::new("POST", "/api/control/pair"), code)
}

fn pair_with(server: &TestServer, request: Req, code: &str) -> Reply {
    request.body(json!({ "code": code }).to_string()).send(server.port)
}

fn wrong(code: &str) -> String {
    let last = if code.ends_with('0') { '1' } else { '0' };
    format!("{}{last}", &code[..5])
}

fn refused(reply: &Reply) {
    assert_eq!(reply.status, 403, "{}", reply.text());
    assert!(!reply.error().is_empty());
}

fn no_cors(reply: &Reply) {
    assert!(
        reply
            .headers
            .iter()
            .all(|(name, _)| !name.to_ascii_lowercase().starts_with("access-control-")),
        "{:?}",
        reply.headers
    );
}

#[test]
fn a_code_is_traded_once_for_the_control_token() {
    let server = TestServer::start();
    let code = issue_code(&server);
    let paired = pair(&server, &code);
    assert_eq!(paired.status, 200, "{}", paired.text());
    no_cors(&paired);
    assert_eq!(paired.json(), json!({ "token": server.control_token }));
    let again = pair(&server, &code);
    refused(&again);
    no_cors(&again);
    let state = server.control("GET", "/api/control/state").send(server.port);
    assert_eq!(state.status, 200);
}

#[test]
fn a_new_code_replaces_the_old_one() {
    let server = TestServer::start();
    let old = issue_code(&server);
    let new = loop {
        let code = issue_code(&server);
        if code != old {
            break code;
        }
    };
    refused(&pair(&server, &old));
    assert_eq!(pair(&server, &new).status, 200);
}

#[test]
fn web_pages_cannot_pair() {
    let server = TestServer::start();
    let code = issue_code(&server);
    let page_requests = [
        ("Origin", "http://evil.example"),
        ("Origin", "null"),
        ("Origin", &format!("http://127.0.0.1:{}", server.port)),
        ("Sec-Fetch-Site", "cross-site"),
        ("Sec-Fetch-Site", "same-site"),
        ("Sec-Fetch-Site", "same-origin"),
    ];
    for (name, value) in page_requests {
        let reply = pair_with(
            &server,
            Req::new("POST", "/api/control/pair").header(name, value),
            &code,
        );
        refused(&reply);
        no_cors(&reply);
        assert!(reply.error().contains("web pages"), "{name}: {}", reply.error());
    }
    let mixed = Req::new("POST", "/api/control/pair")
        .header("Sec-Fetch-Site", "none")
        .header("Sec-Fetch-Site", "cross-site");
    refused(&pair_with(&server, mixed, &code));
    let typed = Req::new("POST", "/api/control/pair").header("Sec-Fetch-Site", "none");
    let paired = pair_with(&server, typed, &code);
    assert_eq!(
        paired.status,
        200,
        "refused pages did not use up the code: {}",
        paired.text()
    );
}

#[test]
fn the_fifth_wrong_code_burns_the_code() {
    let server = TestServer::start();
    let code = issue_code(&server);
    for _ in 0..4 {
        refused(&pair(&server, &wrong(&code)));
    }
    assert_eq!(pair(&server, &code).status, 200, "four wrong codes leave it");
    let code = issue_code(&server);
    for _ in 0..5 {
        refused(&pair(&server, &wrong(&code)));
    }
    refused(&pair(&server, &code));
    let fresh = issue_code(&server);
    assert_eq!(pair(&server, &fresh).status, 200);
}

#[test]
fn malformed_bodies_are_not_guesses() {
    let server = TestServer::start();
    let code = issue_code(&server);
    for body in [
        "",
        "{}",
        r#"{"code": 123456}"#,
        r#"{"code": "1", "token": "x"}"#,
        "code=1",
    ] {
        let reply = Req::new("POST", "/api/control/pair").body(body).send(server.port);
        assert_eq!(reply.status, 400, "{body}: {}", reply.text());
    }
    assert_eq!(pair(&server, &code).status, 200);
}

#[test]
fn without_a_code_nothing_pairs() {
    let server = TestServer::start();
    for guess in ["000000", "123456", ""] {
        refused(&pair(&server, guess));
    }
    let get = Req::new("GET", "/api/control/pair").send(server.port);
    assert_eq!(get.status, 405);
}

#[test]
fn only_the_editor_token_issues_codes_and_revokes() {
    let server = TestServer::start();
    for target in ["/api/control/pairing", "/api/control/revoke"] {
        assert_eq!(server.control("POST", target).send(server.port).status, 403, "{target}");
        assert_eq!(Req::new("POST", target).send(server.port).status, 401, "{target}");
    }
    assert!(server.host.saved_control_token().is_none());
}

#[test]
fn the_editor_token_alone_sees_how_many_codes_were_traded() {
    let server = TestServer::start();
    let pairings = || server.api("GET", "/api/control/pairing").send(server.port).json()["pairings"].clone();
    assert_eq!(pairings(), json!(0));
    let code = issue_code(&server);
    refused(&pair(&server, &wrong(&code)));
    assert_eq!(pairings(), json!(0), "a wrong code paired nothing");
    assert_eq!(pair(&server, &code).status, 200);
    refused(&pair(&server, &code));
    assert_eq!(pairings(), json!(1), "only the trade counts");
    let next = server.api("POST", "/api/control/pairing").send(server.port).json();
    assert_eq!(next["pairings"], 1, "a new code says how many paired before it");
    assert_eq!(
        server.control("GET", "/api/control/pairing").send(server.port).status,
        403
    );
    assert_eq!(Req::new("GET", "/api/control/pairing").send(server.port).status, 401);
}

#[test]
fn revoking_replaces_the_control_token_through_the_host() {
    let server = TestServer::start();
    let code = issue_code(&server);
    let reply = server.api("POST", "/api/control/revoke").send(server.port);
    assert_eq!(reply.status, 200, "{}", reply.text());
    assert_eq!(reply.json(), json!({ "ok": true }));
    let new_token = server
        .host
        .saved_control_token()
        .expect("the host was not asked to keep it");
    assert_ne!(new_token, server.control_token);
    assert_ne!(new_token, server.token);
    assert_eq!(
        server.control("GET", "/api/control/state").send(server.port).status,
        401
    );
    let state = server
        .with_token(&new_token, "GET", "/api/control/state")
        .send(server.port);
    assert_eq!(state.status, 200);
    assert_eq!(
        server.api("GET", "/api/state").send(server.port).status,
        200,
        "the editor token stays"
    );
    refused(&pair(&server, &code));
    let paired = pair(&server, &issue_code(&server));
    assert_eq!(paired.json(), json!({ "token": new_token }));

    let nonce = "00112233445566778899aabbccddeeff";
    let hello = Req::new("GET", &format!("/api/hello?nonce={nonce}"))
        .send(server.port)
        .json();
    let message = format!("{nonce}:{}", server.port);
    assert_eq!(hello["control"], sceneuron_server::proof(&new_token, &message));
}

#[test]
fn a_revoke_that_cannot_be_saved_keeps_the_old_token() {
    let server = TestServer::start();
    let code = issue_code(&server);
    server.host.refuse_control_token.store(true, Ordering::SeqCst);
    let reply = server.api("POST", "/api/control/revoke").send(server.port);
    assert_eq!(reply.status, 500, "{}", reply.text());
    assert!(reply.error().contains("the disk is full"), "{}", reply.error());
    assert_eq!(
        server.control("GET", "/api/control/state").send(server.port).status,
        200
    );
    assert_eq!(pair(&server, &code).json(), json!({ "token": server.control_token }));
    server.host.refuse_control_token.store(false, Ordering::SeqCst);
    assert_eq!(server.api("POST", "/api/control/revoke").send(server.port).status, 200);
    assert_eq!(
        server.control("GET", "/api/control/state").send(server.port).status,
        401
    );
}
