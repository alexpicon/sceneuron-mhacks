// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Shared test helpers: a server on an ephemeral port, a raw HTTP/1.1 client, an event
//! stream reader, a fake host, assistant, and room, control API helpers, and temporary
//! directories under the cargo target dir.

#![allow(dead_code, unused_imports)]

mod assistant;
mod client;
mod control;
mod events;
mod host;
mod room;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use sceneuron_core::{Outcome, Project, PropsPatch, default_project};
use sceneuron_server::{Hub, HubConfig, NullAssistant, NullHost, NullRoom, Server, ServerConfig, ServerError};
use serde_json::Value;

pub use assistant::{FakeAssistant, waving_plan};
pub use client::{Reply, Req, connect, send_raw};
pub use control::{control_post, deck, keys, project_with_packs, server_with_decks, view_after};
pub use events::{EventStream, Item};
pub use host::{FAKE_JPEG, FAKE_MIC, FAKE_SOURCE, FakeHost};
pub use room::FakeRoom;

/// A hub and server on 127.0.0.1 with fresh tokens, a [`FakeHost`], a [`FakeAssistant`], and a
/// [`FakeRoom`].
pub struct TestServer {
    pub hub: Arc<Hub>,
    pub host: Arc<FakeHost>,
    pub assistant: Arc<FakeAssistant>,
    pub room: Arc<FakeRoom>,
    pub token: String,
    /// The control token the server started with.
    pub control_token: String,
    pub port: u16,
    pub server: Option<Server>,
}

impl TestServer {
    pub fn start() -> Self {
        Self::with(default_project(), HubConfig::default())
    }

    pub fn with(project: Project, config: HubConfig) -> Self {
        Self::with_server_config(project, config, ServerConfig::default())
    }

    /// A server that disconnects clients slower than `request_timeout`.
    pub fn with_request_timeout(request_timeout: Duration) -> Self {
        let config = ServerConfig {
            request_timeout,
            ..ServerConfig::default()
        };
        Self::with_server_config(default_project(), HubConfig::default(), config)
    }

    pub fn with_server_config(project: Project, config: HubConfig, server_config: ServerConfig) -> Self {
        let hub = Hub::start(project, config).unwrap();
        let host = Arc::new(FakeHost::default());
        let assistant = Arc::new(FakeAssistant::default());
        let room = Arc::new(FakeRoom::default());
        let token = server_config.token.clone();
        let control_token = server_config.control_token.clone();
        let server = Server::start(
            Arc::clone(&hub),
            host.clone(),
            assistant.clone(),
            room.clone(),
            server_config,
        )
        .unwrap();
        let port = server.port();
        Self {
            hub,
            host,
            assistant,
            room,
            token,
            control_token,
            port,
            server: Some(server),
        }
    }

    /// An authenticated API request.
    pub fn api(&self, method: &str, target: &str) -> Req {
        Req::new(method, target).header("X-Sceneuron-Token", &self.token)
    }

    pub fn get_json(&self, target: &str) -> Value {
        let reply = self.api("GET", target).send(self.port);
        assert_eq!(reply.status, 200, "{}", reply.text());
        reply.json()
    }

    pub fn post_json(&self, target: &str, body: &str) -> Reply {
        self.api("POST", target).body(body).send(self.port)
    }

    pub fn events(&self) -> EventStream {
        EventStream::open(self.port, &format!("/api/events?token={}", self.token), "1.1")
    }

    /// A request with `token` in the header, as control surfaces send it.
    pub fn with_token(&self, token: &str, method: &str, target: &str) -> Req {
        Req::new(method, target).header("X-Sceneuron-Token", token)
    }

    /// A request with the control token the server started with.
    pub fn control(&self, method: &str, target: &str) -> Req {
        self.with_token(&self.control_token, method, target)
    }

    /// The control event stream, opened with `token` in the header.
    pub fn control_events(&self, token: &str) -> EventStream {
        let request = Req::new("GET", "/api/control/events").header("X-Sceneuron-Token", token);
        EventStream::open_request(self.port, &request)
    }

    /// Connections the server holds open right now.
    pub fn open_connections(&self) -> usize {
        self.server.as_ref().expect("the server was stopped").open_connections()
    }

    /// Connections that sent their request head, which the server never closes to make room.
    pub fn started_connections(&self) -> usize {
        self.server
            .as_ref()
            .expect("the server was stopped")
            .started_connections()
    }

    /// Waits until the server holds exactly `count` connections; false on a timeout.
    pub fn connections_settle_at(&self, count: usize) -> bool {
        wait_until(Duration::from_secs(5), || self.open_connections() == count)
    }

    /// Stops the server and returns how long that took.
    pub fn stop(&mut self) -> Duration {
        let started = Instant::now();
        if let Some(server) = self.server.take() {
            server.stop();
        }
        started.elapsed()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.stop();
        self.hub.shutdown();
    }
}

/// A server for `hub` with `config` and no host, assistant, or room.
pub fn start_server(hub: Arc<Hub>, config: ServerConfig) -> Result<Server, ServerError> {
    Server::start(
        hub,
        Arc::new(NullHost),
        Arc::new(NullAssistant),
        Arc::new(NullRoom),
        config,
    )
}

/// A new empty directory under the cargo target dir.
pub fn temp_dir(name: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}-{}-{unique}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Polls `condition` until it holds or `timeout` passes.
pub fn wait_until(timeout: Duration, mut condition: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if condition() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

/// The element with this id from a snapshot.
pub fn element<'a>(snapshot: &'a Value, id: &str) -> &'a Value {
    snapshot["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|element| element["id"] == id)
        .unwrap_or_else(|| panic!("no element {id}"))
}

pub fn parse(message: &str) -> Value {
    serde_json::from_str(message).unwrap()
}

/// Holds a text override on the `brb` element of the default project.
pub fn hold_text(hub: &Hub, text: &str) -> Outcome {
    let patch = PropsPatch {
        text: Some(text.to_owned()),
        ..PropsPatch::default()
    };
    hub.try_update(|engine, now| engine.set_override("brb", &patch, now))
        .unwrap()
}

/// Every message currently queued, and whether the channel is disconnected.
pub fn drain(receiver: &Receiver<Arc<str>>) -> (Vec<Value>, bool) {
    let mut messages = Vec::new();
    loop {
        match receiver.try_recv() {
            Ok(message) => messages.push(parse(&message)),
            Err(TryRecvError::Empty) => return (messages, false),
            Err(TryRecvError::Disconnected) => return (messages, true),
        }
    }
}
