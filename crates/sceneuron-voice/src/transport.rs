// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Reaching the speech-to-text server: TCP within a deadline, and TLS through rustls with the
//! web PKI roots bundled in the plugin.

use std::io::{self, Read, Write};
use std::net::{IpAddr, Shutdown, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use rustls::pki_types::CertificateDer;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

use crate::session::Failure;

/// Where TLS trust comes from.
#[derive(Clone)]
pub struct Transport {
    pub(crate) tls: Arc<ClientConfig>,
}

impl Default for Transport {
    /// Trusts the web PKI roots bundled with Sceneuron, so no system store is consulted.
    fn default() -> Self {
        let roots = RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };
        Transport {
            tls: client_config(roots),
        }
    }
}

impl Transport {
    /// Trusts only `certificate`, DER-encoded, such as a test server's own authority.
    pub fn trusting(certificate: &[u8]) -> Result<Transport, String> {
        let mut roots = RootCertStore::empty();
        roots
            .add(CertificateDer::from(certificate.to_vec()))
            .map_err(|error| format!("cannot trust that certificate: {error}"))?;
        Ok(Transport {
            tls: client_config(roots),
        })
    }
}

fn client_config(roots: RootCertStore) -> Arc<ClientConfig> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("the ring provider supports rustls's default protocol versions")
        .with_root_certificates(roots)
        .with_no_client_auth();
    Arc::new(config)
}

pub(crate) enum Stream {
    Plain(TcpStream),
    Tls(Box<StreamOwned<ClientConnection, TcpStream>>),
}

impl Stream {
    pub(crate) fn tcp(&self) -> &TcpStream {
        match self {
            Stream::Plain(tcp) => tcp,
            Stream::Tls(tls) => tls.get_ref(),
        }
    }
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

pub(crate) fn remaining(deadline: Instant) -> Duration {
    deadline.saturating_duration_since(Instant::now())
}

pub(crate) fn is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost") || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

pub(crate) fn connect_tcp(host: &str, port: u16, deadline: Instant) -> Result<TcpStream, Failure> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| Failure::Retry(format!("cannot find {host}: {error}")))?;
    let mut last_error = None;
    for address in addresses {
        let left = remaining(deadline);
        if left.is_zero() {
            break;
        }
        match TcpStream::connect_timeout(&address, left) {
            Ok(tcp) => return Ok(tcp),
            Err(error) => last_error = Some(error),
        }
    }
    let why = last_error.map_or_else(|| "no address answered".to_owned(), |error| error.to_string());
    Err(Failure::Retry(format!("cannot connect to {host}: {why}")))
}

/// Lets another thread abandon a connection while it opens: aborting closes its socket, so the
/// handshakes fail at once and the opening thread can be joined without waiting out timeouts.
/// A name lookup already under way still runs to its end.
#[derive(Clone, Default)]
pub(crate) struct Abort(Arc<Mutex<AbortState>>);

#[derive(Default)]
struct AbortState {
    aborted: bool,
    socket: Option<TcpStream>,
}

impl Abort {
    /// Fails once aborted, and otherwise keeps a handle to `tcp`, when given, for an abort.
    pub(crate) fn watch(&self, tcp: Option<&TcpStream>) -> Result<(), Failure> {
        let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if state.aborted {
            return Err(Failure::Closed);
        }
        if let Some(tcp) = tcp {
            state.socket = tcp.try_clone().ok();
        }
        Ok(())
    }

    pub(crate) fn abort(&self) {
        let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        state.aborted = true;
        if let Some(socket) = state.socket.take() {
            let _ = socket.shutdown(Shutdown::Both);
        }
    }
}
