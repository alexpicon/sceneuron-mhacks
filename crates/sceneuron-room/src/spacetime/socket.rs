// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Opening the WebSocket to a database: `GET /v1/database/{name}/subscribe` with the JSON text
//! subprotocol, over TCP for `http` and `ws` servers and over TLS (rustls with the bundled web
//! roots) for `https` and `wss` ones.
//!
//! Reads wait at most [`POLL`], so the connection's one thread can also send and notice that it
//! was closed; every slow step checks `closed` often, so closing never waits long for a server
//! that does not answer. A read that waited that long fails as `WouldBlock` ([`Tcp`]).

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use tungstenite::client::IntoClientRequest;
use tungstenite::handshake::HandshakeError;
use tungstenite::handshake::client::Request;
use tungstenite::http::HeaderValue;
use tungstenite::{Error, WebSocket};

use super::endpoint::{Endpoint, endpoint};
use super::protocol::PROTOCOL;
use super::tcp::Tcp;
use crate::target::Target;

/// The longest a read waits before the thread looks at what else it has to do.
pub(super) const POLL: Duration = Duration::from_millis(25);

/// The longest one attempt to reach an address waits, so closing is noticed in between.
const CONNECT_SLICE: Duration = Duration::from_millis(500);

/// A write that cannot go out for this long means the connection is gone.
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);

pub(super) type Socket = WebSocket<Stream>;

/// A TCP connection, with TLS for secure servers.
pub(super) enum Stream {
    Plain(Tcp),
    Tls(Box<StreamOwned<ClientConnection, Tcp>>),
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Stream::Plain(stream) => stream.read(buf),
            Stream::Tls(stream) => stream.read(buf),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Stream::Plain(stream) => stream.write(buf),
            Stream::Tls(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Stream::Plain(stream) => stream.flush(),
            Stream::Tls(stream) => stream.flush(),
        }
    }
}

/// Connects to `target`'s subscribe endpoint by `deadline`, giving up as soon as `closed` is set.
pub(super) fn open(target: &Target, closed: &AtomicBool, deadline: Instant) -> Result<Socket, String> {
    let endpoint = endpoint(target)?;
    let tcp = reach(&endpoint, closed, deadline)?;
    tcp.set_nodelay(true).map_err(|error| error.to_string())?;
    tcp.set_read_timeout(Some(POLL)).map_err(|error| error.to_string())?;
    tcp.set_write_timeout(Some(WRITE_TIMEOUT))
        .map_err(|error| error.to_string())?;
    let stream = if endpoint.secure {
        let name =
            ServerName::try_from(endpoint.host.clone()).map_err(|error| format!("{}: {error}", endpoint.host))?;
        let connection = ClientConnection::new(tls_config()?, name).map_err(|error| error.to_string())?;
        Stream::Tls(Box::new(StreamOwned::new(connection, Tcp(tcp))))
    } else {
        Stream::Plain(Tcp(tcp))
    };
    let mut request = endpoint
        .url
        .as_str()
        .into_client_request()
        .map_err(|error| error.to_string())?;
    request
        .headers_mut()
        .insert("Sec-WebSocket-Protocol", HeaderValue::from_static(PROTOCOL));
    handshake(request, stream, closed, deadline)
}

/// Sends `request` over `stream` and reads the server's answer, reading again after each read
/// that waited a poll, until `deadline`; gives up as soon as `closed` is set.
pub(super) fn handshake<S: Read + Write>(
    request: Request,
    stream: S,
    closed: &AtomicBool,
    deadline: Instant,
) -> Result<WebSocket<S>, String> {
    let mut attempt = tungstenite::client(request, stream);
    loop {
        match attempt {
            Ok((socket, _)) => return Ok(socket),
            Err(HandshakeError::Interrupted(waiting)) => {
                if closed.load(Ordering::SeqCst) {
                    return Err("the connection was closed".to_owned());
                }
                if Instant::now() >= deadline {
                    return Err("the server did not finish the WebSocket handshake".to_owned());
                }
                attempt = waiting.handshake();
            }
            Err(HandshakeError::Failure(error)) => return Err(refusal(error)),
        }
    }
}

/// A TCP connection to the first address of the endpoint that answers.
fn reach(endpoint: &Endpoint, closed: &AtomicBool, deadline: Instant) -> Result<TcpStream, String> {
    let addresses: Vec<SocketAddr> = (endpoint.host.as_str(), endpoint.port)
        .to_socket_addrs()
        .map_err(|error| format!("cannot find {}: {error}", endpoint.host))?
        .collect();
    let mut last = format!("{} has no address", endpoint.host);
    for address in addresses {
        loop {
            if closed.load(Ordering::SeqCst) {
                return Err("the connection was closed".to_owned());
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(format!("{address} did not answer"));
            }
            match TcpStream::connect_timeout(&address, left.min(CONNECT_SLICE)) {
                Ok(stream) => return Ok(stream),
                // Another attempt until the deadline, which reports the address as not answering.
                Err(error) if error.kind() == io::ErrorKind::TimedOut => {}
                Err(error) => {
                    last = error.to_string();
                    break;
                }
            }
        }
    }
    Err(last)
}

/// What a failed handshake says, with the server's answer when it gave one, such as a database
/// that does not exist.
fn refusal(error: Error) -> String {
    match error {
        Error::Http(response) => {
            let body = response
                .body()
                .as_deref()
                .map(String::from_utf8_lossy)
                .unwrap_or_default();
            let status = response.status();
            match body.trim() {
                "" => format!("the server answered {status}"),
                body => format!("the server answered {status}: {body}"),
            }
        }
        other => other.to_string(),
    }
}

/// TLS through rustls with the ring provider and Mozilla's roots, as the rest of Sceneuron uses.
fn tls_config() -> Result<Arc<ClientConfig>, String> {
    static CONFIG: OnceLock<Result<Arc<ClientConfig>, String>> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            let roots = RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
            };
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            let config = ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .map_err(|error| format!("cannot set up TLS: {error}"))?
                .with_root_certificates(roots)
                .with_no_client_auth();
            Ok(Arc::new(config))
        })
        .clone()
}
