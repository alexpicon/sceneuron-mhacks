// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! An in-process stand-in for the Scribe realtime server, for tests without a key or a network.
//! It listens on 127.0.0.1, speaks the same messages, records what each client sent, and answers
//! with scripted transcripts. Nothing it does costs ElevenLabs credits.

mod serve;

use std::collections::BTreeSet;
use std::io::{self, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};

use serve::connection;

const PATH: &str = "/v1/speech-to-text/realtime";

/// What the fake server does.
#[derive(Clone, Debug, Default)]
pub struct FakeScript {
    /// The key it accepts; a session with any other key gets an `auth_error`.
    pub key: String,
    /// Final transcripts in order, one per segment, each after a partial with its first word.
    /// A commit ends a segment, and so does every `vad_bytes` of audio in a `vad` session.
    pub transcripts: Vec<String>,
    /// The language code it reports after each final transcript, as the real server does in a
    /// transcript with timestamps, to a session that asks for language detection; empty reports
    /// none.
    pub language: String,
    /// Audio per segment in `vad` sessions; 0 means one second.
    pub vad_bytes: usize,
    /// For each connection in order, an error sent instead of starting a session, as
    /// `(message_type, error)`.
    pub refusals: Vec<Option<(String, String)>>,
    /// For each connection in order, an HTTP status and body that refuse the WebSocket
    /// handshake itself.
    pub http_refusals: Vec<Option<(u16, String)>>,
    /// For each started session in order, how many audio messages it takes before the server
    /// closes it.
    pub close_after: Vec<Option<usize>>,
}

/// What one client did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FakeSession {
    pub path: String,
    /// Query parameters, decoded, in order.
    pub query: Vec<(String, String)>,
    /// The `xi-api-key` header.
    pub key: Option<String>,
    pub audio: Vec<u8>,
    pub audio_messages: usize,
    pub commits: usize,
    pub sample_rates: BTreeSet<u64>,
    /// Messages other than audio, as sent.
    pub other_messages: Vec<String>,
    pub ended: bool,
    /// The HTTP status that refused the handshake, if one did.
    pub http_refused: Option<u16>,
}

impl FakeSession {
    pub fn query_values(&self, name: &str) -> Vec<&str> {
        self.query
            .iter()
            .filter(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
            .collect()
    }
}

#[derive(Default)]
struct State {
    script: FakeScript,
    sessions: Vec<FakeSession>,
    started: usize,
    next_transcript: usize,
    connections: usize,
}

type Shared = Arc<Mutex<State>>;

fn lock(state: &Shared) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A running fake server; dropping it stops it.
pub struct FakeScribe {
    address: SocketAddr,
    tls: bool,
    state: Shared,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl FakeScribe {
    /// Serves plain `ws://`, which the client accepts only on this computer.
    pub fn start(script: FakeScript) -> io::Result<FakeScribe> {
        FakeScribe::serve(script, None)
    }

    /// Serves `wss://` with `tls`; see [`tls_config`].
    pub fn start_tls(script: FakeScript, tls: Arc<ServerConfig>) -> io::Result<FakeScribe> {
        FakeScribe::serve(script, Some(tls))
    }

    fn serve(script: FakeScript, tls: Option<Arc<ServerConfig>>) -> io::Result<FakeScribe> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let state: Shared = Arc::new(Mutex::new(State {
            script,
            ..State::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let secure = tls.is_some();
        let (shared, stopping) = (Arc::clone(&state), Arc::clone(&stop));
        let thread = thread::spawn(move || accept_loop(&listener, tls, &shared, &stopping));
        Ok(FakeScribe {
            address,
            tls: secure,
            state,
            stop,
            thread: Some(thread),
        })
    }

    /// The endpoint for a voice config.
    pub fn endpoint(&self) -> String {
        match self.tls {
            true => format!("wss://localhost:{}{PATH}", self.address.port()),
            false => format!("ws://127.0.0.1:{}{PATH}", self.address.port()),
        }
    }

    pub fn sessions(&self) -> Vec<FakeSession> {
        lock(&self.state).sessions.clone()
    }

    /// Waits up to `timeout` for `ready` to hold; the sessions as last seen, as `Err` on timeout.
    pub fn wait(
        &self,
        timeout: Duration,
        ready: impl Fn(&[FakeSession]) -> bool,
    ) -> Result<Vec<FakeSession>, Vec<FakeSession>> {
        let deadline = Instant::now() + timeout;
        loop {
            let sessions = self.sessions();
            if ready(&sessions) {
                return Ok(sessions);
            }
            if Instant::now() >= deadline {
                return Err(sessions);
            }
            thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for FakeScribe {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// A server TLS config from a DER certificate and its PKCS #8 DER key.
pub fn tls_config(certificate: &[u8], key: &[u8]) -> Result<Arc<ServerConfig>, String> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.to_vec()));
    ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .and_then(|builder| {
            builder
                .with_no_client_auth()
                .with_single_cert(vec![CertificateDer::from(certificate.to_vec())], key)
        })
        .map(Arc::new)
        .map_err(|error| error.to_string())
}

fn accept_loop(listener: &TcpListener, tls: Option<Arc<ServerConfig>>, state: &Shared, stop: &Arc<AtomicBool>) {
    let mut connections = Vec::new();
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((tcp, _)) => {
                let (tls, state, stop) = (tls.clone(), Arc::clone(state), Arc::clone(stop));
                connections.push(thread::spawn(move || {
                    let _ = connection(tcp, tls, &state, &stop);
                }));
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(2)),
            Err(_) => break,
        }
    }
    for connection in connections {
        let _ = connection.join();
    }
}

enum Stream {
    Plain(TcpStream),
    Tls(Box<StreamOwned<ServerConnection, TcpStream>>),
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Stream::Plain(tcp) => tcp.read(buf),
            Stream::Tls(tls) => tls.read(buf),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Stream::Plain(tcp) => tcp.write(buf),
            Stream::Tls(tls) => tls.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Stream::Plain(tcp) => tcp.flush(),
            Stream::Tls(tls) => tls.flush(),
        }
    }
}
