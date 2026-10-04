// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The local HTTP server: it listens on the IPv4 and, where present, IPv6 loopback
//! addresses, with one accept thread each, and serves each connection, event streams
//! included, on a short-lived thread of its own.

use std::io::ErrorKind;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::config::{MAX_REQUEST_TIMEOUT, ServerConfig, ServerError};
use crate::control_access::ControlAccess;
use crate::links::EditorLinks;
use crate::registry::Connections;
use crate::setup::SetupStep;
use crate::sse::Streams;
use crate::{Assistant, Host, Hub, Room, accept, stream_deck, token};

/// How long `stop` waits for the accept thread and the connection threads, so that it
/// returns within a second even when a thread cannot be woken.
const STOP_GRACE: Duration = Duration::from_millis(900);

/// What request handling shares across threads.
pub(crate) struct Context {
    pub(crate) hub: Arc<Hub>,
    pub(crate) host: Arc<dyn Host>,
    pub(crate) assistant: Arc<dyn Assistant>,
    pub(crate) room: Arc<dyn Room>,
    pub(crate) port: u16,
    pub(crate) token: String,
    pub(crate) overlay_token: String,
    /// The control token, which revoking replaces while the server runs, and its pairing codes.
    pub(crate) control: ControlAccess,
    pub(crate) links: EditorLinks,
    /// Exact `Host` header values that requests may carry.
    pub(crate) allowed_hosts: Vec<String>,
    pub(crate) streams: Streams,
    pub(crate) connections: Arc<Connections>,
    /// How long a client may take to send its request.
    pub(crate) request_timeout: Duration,
    pub(crate) stopping: AtomicBool,
}

impl Context {
    pub(crate) fn overlay_url(&self) -> String {
        format!("http://127.0.0.1:{}/overlay#token={}", self.port, self.overlay_token)
    }
}

/// The running HTTP server; dropping it stops it.
pub struct Server {
    port: u16,
    context: Arc<Context>,
    /// Each accept thread with the address it listens on; each returns the connection
    /// threads that may still run when it ends.
    acceptors: Vec<(SocketAddr, JoinHandle<Vec<JoinHandle<()>>>)>,
}

impl Server {
    /// Listens on 127.0.0.1, and on [::1] with the same port when the system has IPv6
    /// loopback, and starts answering requests.
    ///
    /// Holding [::1] keeps another program from listening there and receiving the token
    /// from a browser that resolves `localhost` to IPv6 first. When another program
    /// already holds it, requests for `localhost` are refused and only 127.0.0.1 works.
    pub fn start(
        hub: Arc<Hub>,
        host: Arc<dyn Host>,
        assistant: Arc<dyn Assistant>,
        room: Arc<dyn Room>,
        config: ServerConfig,
    ) -> Result<Server, ServerError> {
        token::check_token(&config.token).map_err(ServerError::InvalidToken)?;
        token::check_token(&config.overlay_token).map_err(ServerError::InvalidToken)?;
        token::check_token(&config.control_token).map_err(ServerError::InvalidToken)?;
        if config.overlay_token == config.token {
            let reason = "the overlay token must differ from the editor token".to_owned();
            return Err(ServerError::InvalidToken(reason));
        }
        if config.control_token == config.token || config.control_token == config.overlay_token {
            let reason = "the control token must differ from the editor and overlay tokens".to_owned();
            return Err(ServerError::InvalidToken(reason));
        }
        let timeout = config.request_timeout;
        if timeout.is_zero() || timeout > MAX_REQUEST_TIMEOUT {
            return Err(ServerError::InvalidTimeout(timeout));
        }
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, config.port)).map_err(ServerError::Bind)?;
        let port = listener.local_addr().map_err(ServerError::Bind)?.port();
        let mut listeners = vec![listener];
        let mut allowed_hosts = vec![format!("127.0.0.1:{port}")];
        match TcpListener::bind((Ipv6Addr::LOCALHOST, port)) {
            Ok(v6) => {
                listeners.push(v6);
                allowed_hosts.extend([format!("localhost:{port}"), format!("[::1]:{port}")]);
            }
            // Without IPv6 loopback, `localhost` can only reach 127.0.0.1.
            Err(error) if matches!(error.kind(), ErrorKind::AddrNotAvailable | ErrorKind::Unsupported) => {
                allowed_hosts.push(format!("localhost:{port}"));
            }
            Err(_) => {}
        }
        let context = Arc::new(Context {
            hub,
            host,
            assistant,
            room,
            port,
            token: config.token,
            overlay_token: config.overlay_token,
            control: ControlAccess::new(config.control_token),
            links: EditorLinks::default(),
            allowed_hosts,
            streams: Streams::default(),
            connections: Arc::default(),
            request_timeout: timeout,
            stopping: AtomicBool::new(false),
        });
        let mut server = Server {
            port,
            context,
            acceptors: Vec::new(),
        };
        for listener in listeners {
            let address = listener.local_addr().map_err(ServerError::Bind)?;
            let context = Arc::clone(&server.context);
            let acceptor = thread::Builder::new()
                .name("sceneuron-http".to_owned())
                .spawn(move || accept::run(&listener, &context))
                .map_err(ServerError::Thread)?;
            server.acceptors.push((address, acceptor));
        }
        Ok(server)
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// The editor's address with its token, for a Custom Browser Dock or a bookmark.
    ///
    /// Addresses carry their token in the fragment, which browsers never send to a server, so
    /// opening one cannot leak the token to another program on the port; the page proves the
    /// server first (see `/api/hello`).
    pub fn editor_url(&self) -> String {
        format!("http://127.0.0.1:{}/editor#token={}", self.port, self.context.token)
    }

    /// A single-use editor address that works for one minute, for opening the editor in a
    /// browser through the command line without exposing the token there.
    pub fn editor_link(&self) -> String {
        let code = self.context.links.issue();
        format!("http://127.0.0.1:{}/editor?code={code}", self.port)
    }

    /// The overlay page's address with the read-only overlay token.
    pub fn overlay_url(&self) -> String {
        self.context.overlay_url()
    }

    /// Sends Sceneuron's plugin in the Stream Deck app a pairing link, as "Set up OBS for me"
    /// does: only when it is installed and no control surface traded a code for the control
    /// token or made a request with it since the server started or the token was revoked.
    pub fn pair_stream_deck(&self) -> SetupStep {
        stream_deck::pair(&self.context)
    }

    /// Connections open right now, event streams included.
    pub fn open_connections(&self) -> usize {
        self.context.connections.count()
    }

    /// Open connections that sent their request head, so making room never closes them.
    pub fn started_connections(&self) -> usize {
        self.context.connections.started()
    }

    /// Stops accepting, closes every connection and event stream, and waits for their
    /// threads; dropping does the same.
    pub fn stop(mut self) {
        self.shutdown();
    }

    /// Waits at most [`STOP_GRACE`]; a thread that is still busy then, such as one inside
    /// a slow [`Host`] call, is left to finish on its own instead of hanging the caller.
    fn shutdown(&mut self) {
        if self.acceptors.is_empty() {
            return;
        }
        let deadline = Instant::now() + STOP_GRACE;
        self.context.stopping.store(true, Ordering::Release);
        // Connections accepted from here on are refused, so none escapes this close.
        self.context.connections.close_all();
        // Each listening socket closes when its accept thread ends.
        for (address, acceptor) in self.acceptors.drain(..) {
            for worker in accept::stop(acceptor, address, deadline) {
                if accept::join_before(&worker, deadline) {
                    let _ = worker.join();
                }
            }
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown();
    }
}
