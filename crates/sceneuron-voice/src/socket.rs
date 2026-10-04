// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! One WebSocket session with the speech-to-text server: the WebSocket handshake over the
//! transport with the key in the `xi-api-key` header, the wait for `session_started`, and the
//! messages after it.

use std::io::{self, ErrorKind as IoErrorKind};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rustls::pki_types::ServerName;
use rustls::{ClientConnection, StreamOwned};
use tungstenite::client::IntoClientRequest;
use tungstenite::handshake::HandshakeError;
use tungstenite::http::HeaderValue;
use tungstenite::protocol::WebSocketConfig;
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::{Error as WsError, Message, WebSocket};

use crate::config::ApiKey;
use crate::protocol::{ServerMessage, parse_server_message};
use crate::session::{Failure, failure_for};
use crate::transport::{Abort, Stream, Transport, connect_tcp, is_loopback, remaining};

/// Connecting, both handshakes, and the wait for `session_started` share this much time.
const OPEN_TIMEOUT: Duration = Duration::from_secs(10);
/// How long a read waits for the server before the worker turns to its other work.
const POLL: Duration = Duration::from_millis(20);
/// A write that cannot finish in this time means the connection is gone.
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
/// Transcripts are small; a larger message means something is wrong.
const MAX_MESSAGE_BYTES: usize = 1 << 20;

/// An open session. Reads wait at most [`POLL`], so one thread can also serve its commands.
pub(crate) struct Link {
    socket: WebSocket<Stream>,
}

impl Link {
    /// Opens a session at `url`, unless `abort` ends the attempt first. A `ws://` URL is refused
    /// unless the server is on this computer, so the key never crosses a network in the clear.
    /// Messages never contain it.
    pub(crate) fn open(url: &str, key: &ApiKey, transport: &Transport, abort: &Abort) -> Result<Link, Failure> {
        Link::open_inner(url, key, transport, abort).map_err(|failure| failure.redacted(key))
    }

    fn open_inner(url: &str, key: &ApiKey, transport: &Transport, abort: &Abort) -> Result<Link, Failure> {
        let deadline = Instant::now() + OPEN_TIMEOUT;
        let mut request = url
            .into_client_request()
            .map_err(|error| Failure::Fatal(format!("the speech-to-text address is not valid: {error}")))?;
        let uri = request.uri().clone();
        let host = uri.host().unwrap_or_default().trim_matches(['[', ']']).to_owned();
        let secure = match uri.scheme_str() {
            Some("wss") => true,
            Some("ws") if is_loopback(&host) => false,
            _ => {
                return Err(Failure::Fatal(
                    "the speech-to-text address must start with wss://".to_owned(),
                ));
            }
        };
        let mut value = HeaderValue::from_str(key.expose())
            .map_err(|_| Failure::Fatal("the ElevenLabs API key has characters a header cannot carry".to_owned()))?;
        value.set_sensitive(true);
        let headers = request.headers_mut();
        headers.insert("xi-api-key", value);
        headers.insert(
            "user-agent",
            HeaderValue::from_static(concat!("sceneuron/", env!("CARGO_PKG_VERSION"))),
        );
        let port = uri.port_u16().unwrap_or(if secure { 443 } else { 80 });
        abort.watch(None)?;
        let tcp = connect_tcp(&host, port, deadline)?;
        abort.watch(Some(&tcp))?;
        let setup = |tcp: &TcpStream| {
            tcp.set_nodelay(true)?;
            tcp.set_write_timeout(Some(WRITE_TIMEOUT))?;
            tcp.set_read_timeout(Some(remaining(deadline)))
        };
        setup(&tcp).map_err(|error| Failure::Retry(format!("cannot set up the connection: {error}")))?;
        let stream = if secure {
            let name = ServerName::try_from(host.clone())
                .map_err(|_| Failure::Fatal(format!("{host} is not a valid server name")))?;
            let connection = ClientConnection::new(Arc::clone(&transport.tls), name)
                .map_err(|error| Failure::Retry(format!("cannot start TLS: {error}")))?;
            Stream::Tls(Box::new(StreamOwned::new(connection, tcp)))
        } else {
            Stream::Plain(tcp)
        };
        let config = WebSocketConfig::default()
            .max_message_size(Some(MAX_MESSAGE_BYTES))
            .max_frame_size(Some(MAX_MESSAGE_BYTES));
        let (socket, _) = tungstenite::client::client_with_config(request, stream, Some(config))
            .map_err(|error| handshake_failure(error, &host))?;
        let mut link = Link { socket };
        link.wait_for_start(deadline)?;
        link.read_timeout(POLL)?;
        Ok(link)
    }

    fn read_timeout(&self, timeout: Duration) -> Result<(), Failure> {
        let timeout = timeout.max(Duration::from_millis(1));
        self.socket
            .get_ref()
            .tcp()
            .set_read_timeout(Some(timeout))
            .map_err(|error| Failure::Retry(format!("cannot set up the connection: {error}")))
    }

    fn wait_for_start(&mut self, deadline: Instant) -> Result<(), Failure> {
        loop {
            let left = remaining(deadline);
            if left.is_zero() {
                return Err(Failure::Retry(
                    "the speech-to-text server did not start a session".to_owned(),
                ));
            }
            self.read_timeout(left.min(Duration::from_millis(200)))?;
            match self.receive()? {
                Some(ServerMessage::SessionStarted) => return Ok(()),
                Some(ServerMessage::Error { kind, code, message }) => return Err(failure_for(kind, &code, &message)),
                _ => {}
            }
        }
    }

    /// The next message, or `None` when nothing usable arrived within the read timeout.
    pub(crate) fn receive(&mut self) -> Result<Option<ServerMessage>, Failure> {
        match self.socket.read() {
            Ok(Message::Text(text)) => Ok(parse_server_message(text.as_str())),
            Ok(Message::Close(Some(frame))) if frame.code != CloseCode::Normal => Err(Failure::Retry(format!(
                "the server ended the session ({}): {}",
                u16::from(frame.code),
                frame.reason.as_str()
            ))),
            Ok(Message::Close(_)) => Err(Failure::Closed),
            Ok(_) => Ok(None),
            Err(WsError::Io(error)) if waiting(&error) => Ok(None),
            Err(WsError::ConnectionClosed | WsError::AlreadyClosed) => Err(Failure::Closed),
            Err(error) => Err(Failure::Retry(format!("the speech-to-text connection failed: {error}"))),
        }
    }

    pub(crate) fn send(&mut self, text: String) -> Result<(), Failure> {
        self.socket.send(Message::text(text)).map_err(|error| match error {
            WsError::ConnectionClosed | WsError::AlreadyClosed => Failure::Closed,
            WsError::Io(error) if waiting(&error) => Failure::Retry("the connection stalled".to_owned()),
            error => Failure::Retry(format!("cannot send audio: {error}")),
        })
    }

    /// Says goodbye as the protocol asks; the connection closes when the link is dropped.
    pub(crate) fn close(mut self) {
        let _ = self.socket.close(None);
        let _ = self.socket.flush();
    }
}

fn waiting(error: &io::Error) -> bool {
    matches!(error.kind(), IoErrorKind::WouldBlock | IoErrorKind::TimedOut)
}

fn handshake_failure<Role: tungstenite::handshake::HandshakeRole>(error: HandshakeError<Role>, host: &str) -> Failure {
    let error = match error {
        HandshakeError::Interrupted(_) => {
            return Failure::Retry(format!("{host} did not answer the WebSocket handshake in time"));
        }
        HandshakeError::Failure(error) => error,
    };
    match error {
        WsError::Http(response) => {
            let status = response.status().as_u16();
            let body = response
                .body()
                .as_deref()
                .map(String::from_utf8_lossy)
                .unwrap_or_default();
            let detail = body.trim();
            let message = format!("ElevenLabs answered HTTP {status} {detail}")
                .trim_end()
                .to_owned();
            match status {
                401 | 403 => Failure::Fatal(format!("ElevenLabs refused the API key (HTTP {status})")),
                429 | 500..=599 => Failure::Retry(message),
                _ => Failure::Fatal(message),
            }
        }
        WsError::Io(error) if waiting(&error) => Failure::Retry(format!("{host} did not answer in time")),
        error => Failure::Retry(format!("cannot open a session with {host}: {error}")),
    }
}
