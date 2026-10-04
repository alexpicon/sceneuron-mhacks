// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the Stream Deck step of "Set up OBS for me" and the pairing OBS sends when it starts:
//! with Sceneuron's Stream Deck plugin installed and nothing paired, the host opens a pairing link
//! whose fresh code trades for the control token, and the answer never carries the code; a
//! control surface that traded a code or made a request with the control token in its header
//! counts as paired, so no link goes out, until a revoke; without the plugin the step says how to
//! install it and leaves the editor's code alone; and a link the host cannot open says why.

mod common;

use std::sync::atomic::Ordering;

use common::{Req, TestServer};
use sceneuron_server::{SetupOutcome, stream_deck_pairing_link};
use serde_json::{Value, json};

const SETUP: &str = "/api/setup";

fn installed() -> TestServer {
    let server = TestServer::start();
    server.host.stream_deck_installed.store(true, Ordering::SeqCst);
    server
}

/// The Stream Deck step of a call to "Set up OBS for me".
fn set_up(server: &TestServer) -> Value {
    let reply = server.post_json(SETUP, "");
    assert_eq!(reply.status, 200, "{}", reply.text());
    reply.json()["stream_deck"].clone()
}

fn step(outcome: &str) -> Value {
    json!({ "outcome": outcome, "source": "Stream Deck", "reason": "" })
}

fn links(server: &TestServer) -> Vec<String> {
    server.host.stream_deck_links.lock().unwrap().clone()
}

/// The code in the newest pairing link, after checking that the link is the editor's, for this port.
fn code_in_last_link(server: &TestServer) -> String {
    let link = links(server).pop().expect("no pairing link was opened");
    let prefix = format!(
        "streamdeck://plugins/message/ai.chaski.sceneuron/pair?port={}&code=",
        server.port
    );
    let code = link
        .strip_prefix(&prefix)
        .and_then(|rest| rest.strip_suffix("&streamdeck=hidden"))
        .unwrap_or_else(|| panic!("not a pairing link for this port: {link}"));
    assert!(
        code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit()),
        "{link}"
    );
    code.to_owned()
}

fn trade(server: &TestServer, code: &str) -> Value {
    let reply = Req::new("POST", "/api/control/pair")
        .body(json!({ "code": code }).to_string())
        .send(server.port);
    assert_eq!(reply.status, 200, "{}", reply.text());
    reply.json()
}

#[test]
fn the_link_is_the_one_the_editor_opens() {
    assert_eq!(
        stream_deck_pairing_link(4466, "012345"),
        "streamdeck://plugins/message/ai.chaski.sceneuron/pair?port=4466&code=012345&streamdeck=hidden"
    );
}

#[test]
fn with_the_plugin_installed_and_nothing_paired_a_link_pairs_it_and_the_next_click_finds_it() {
    let server = installed();
    let reply = server.post_json(SETUP, "");
    assert_eq!(reply.json()["stream_deck"], step("added"));
    let code = code_in_last_link(&server);
    assert!(!reply.text().contains(&code), "the answer carries the code");
    assert_eq!(trade(&server, &code), json!({ "token": server.control_token }));
    assert_eq!(server.get_json("/api/control/pairing"), json!({ "pairings": 1 }));

    assert_eq!(set_up(&server), step("present"));
    assert_eq!(links(&server).len(), 1, "no second link");
}

#[test]
fn a_request_with_the_control_token_in_its_header_counts_as_paired() {
    let server = installed();
    let query = format!("/api/control/state?token={}", server.control_token);
    assert_eq!(Req::new("GET", &query).send(server.port).status, 401);
    let wrong = server.with_token("control-token-that-is-wrong-0123", "GET", "/api/control/state");
    assert_eq!(wrong.send(server.port).status, 401);
    assert_eq!(set_up(&server), step("added"), "neither request counts");

    assert_eq!(
        server.control("GET", "/api/control/state").send(server.port).status,
        200
    );
    assert_eq!(set_up(&server), step("present"));
    assert_eq!(links(&server).len(), 1);
}

#[test]
fn revoking_ends_the_pairing_so_the_next_click_sends_a_new_link() {
    let server = installed();
    set_up(&server);
    let first = code_in_last_link(&server);
    trade(&server, &first);
    assert_eq!(server.api("POST", "/api/control/revoke").send(server.port).status, 200);

    assert_eq!(set_up(&server), step("added"));
    let token = server.host.saved_control_token().unwrap();
    assert_eq!(trade(&server, &code_in_last_link(&server)), json!({ "token": token }));
    assert_eq!(set_up(&server), step("present"));
    assert_eq!(links(&server).len(), 2);
}

#[test]
fn without_the_plugin_the_step_says_how_to_install_it_and_leaves_the_editors_code() {
    let server = TestServer::start();
    let issued = server.api("POST", "/api/control/pairing").send(server.port).json();
    let skipped = set_up(&server);
    assert_eq!(skipped["outcome"], "skipped");
    assert_eq!(skipped["source"], "");
    let reason = skipped["reason"].as_str().unwrap();
    assert!(
        reason.contains("not installed") && reason.contains("double-click ai.chaski.sceneuron.streamDeckPlugin"),
        "{reason}"
    );
    assert!(links(&server).is_empty());
    let code = issued["code"].as_str().unwrap();
    assert_eq!(trade(&server, code), json!({ "token": server.control_token }));
    assert_eq!(
        set_up(&server),
        step("present"),
        "paired another way, the plugin is not needed"
    );
}

#[test]
fn a_link_the_host_cannot_open_is_skipped_with_the_reason() {
    let server = installed();
    server.host.refuse_stream_deck_link.store(true, Ordering::SeqCst);
    let skipped = set_up(&server);
    assert_eq!(skipped["outcome"], "skipped");
    let reason = skipped["reason"].as_str().unwrap();
    assert_eq!(
        reason,
        "the pairing link could not be sent to the Stream Deck app: no application can open streamdeck links"
    );
}

#[test]
fn obs_pairs_at_start_through_the_server_as_the_click_does() {
    let server = installed();
    let running = server.server.as_ref().unwrap();
    let sent = running.pair_stream_deck();
    assert_eq!(sent.outcome, SetupOutcome::Added);
    trade(&server, &code_in_last_link(&server));
    assert_eq!(running.pair_stream_deck().outcome, SetupOutcome::Present);
    assert_eq!(links(&server).len(), 1);
}
