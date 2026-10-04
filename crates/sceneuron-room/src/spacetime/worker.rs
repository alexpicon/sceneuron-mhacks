// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A connection's thread: it opens the WebSocket, subscribes to the public tables, sends the calls
//! callers queue, and reads what the server sends, reporting this connection's room whenever it
//! changed. A call's answer comes in the same message as the rows it changed, so the room is
//! reported before the call completes. A connection that hears nothing from the server for its
//! idle limit, not even the answer to a ping, counts as lost.

use std::io::ErrorKind;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use tungstenite::protocol::CloseFrame;
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::{Bytes, Error, Message as Frame};

use super::cache::{Cache, QUERIES};
use super::link::Link;
use super::protocol::{self, Message};
use super::socket::{self, Socket};
use crate::backend::{BackendEvent, RoomView};
use crate::target::Target;

/// How long connecting and the first look at the rooms may take.
pub(super) const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// How long closing waits for the server to answer the close.
const CLOSE_WAIT: Duration = Duration::from_millis(500);

/// How long a connection may hear nothing from the server before it counts as lost: three of the
/// pings SpacetimeDB sends every 15 seconds.
pub(super) const IDLE_LIMIT: Duration = Duration::from_secs(45);

/// Why following the rooms stopped.
enum Stop {
    /// A caller closed the connection.
    Closed,
    /// The connection ended by itself, with the reason when there is one.
    Lost(Option<String>),
}

/// What the thread knows about the server's side.
#[derive(Default)]
struct Follow {
    /// This connection's identity, which the server sends first.
    me: Option<String>,
    cache: Cache,
    /// The last room reported, so only changes are; `None` until the first report.
    last: Option<Option<RoomView>>,
}

/// Connects, follows the rooms, and reports until the connection is closed, ends, or hears nothing
/// from the server for `idle_limit`.
pub(super) fn run(link: &Link, target: &Target, idle_limit: Duration) {
    let deadline = Instant::now() + CONNECT_TIMEOUT;
    let mut socket = match socket::open(target, &link.closed, deadline) {
        Ok(socket) => socket,
        Err(reason) => return link.set_ready(Err(reason)),
    };
    link.queue(protocol::subscribe(&QUERIES, link.request_id()));
    match follow(link, &mut socket, deadline, idle_limit) {
        Stop::Closed => say_goodbye(&mut socket),
        Stop::Lost(reason) => link.lost(reason),
    }
}

fn follow(link: &Link, socket: &mut Socket, deadline: Instant, idle_limit: Duration) -> Stop {
    let mut state = Follow::default();
    let mut farewell = None;
    // When the server last sent anything, and whether it was pinged since.
    let mut heard = Instant::now();
    let mut pinged = false;
    loop {
        if link.closed.load(Ordering::SeqCst) {
            return Stop::Closed;
        }
        if !link.is_ready() && Instant::now() >= deadline {
            return Stop::Lost(Some("the server did not send the rooms in time".to_owned()));
        }
        let quiet = heard.elapsed();
        if quiet >= idle_limit {
            return Stop::Lost(Some("the server stopped answering".to_owned()));
        }
        // Any server answers a ping, so a quiet room stays joined whether or not its server pings.
        if !pinged && quiet >= idle_limit / 3 {
            if let Err(error) = socket.send(Frame::Ping(Bytes::new())) {
                return Stop::Lost(Some(error.to_string()));
            }
            pinged = true;
        }
        for message in link.take_outbox() {
            if let Err(error) = socket.send(Frame::text(message)) {
                return Stop::Lost(Some(error.to_string()));
            }
        }
        let read = socket.read();
        if read.is_ok() {
            heard = Instant::now();
            pinged = false;
        }
        match read {
            Ok(Frame::Text(text)) => {
                if let Err(reason) = state.handle(link, text.as_str()) {
                    return Stop::Lost(Some(reason));
                }
            }
            Ok(Frame::Close(frame)) => farewell = frame.map(|frame| frame.reason.to_string()),
            Ok(_) => {}
            Err(Error::Io(error)) if error.kind() == ErrorKind::WouldBlock => {}
            Err(Error::ConnectionClosed | Error::AlreadyClosed) => {
                return Stop::Lost(farewell.filter(|reason| !reason.is_empty()));
            }
            Err(error) => return Stop::Lost(Some(error.to_string())),
        }
    }
}

impl Follow {
    fn handle(&mut self, link: &Link, text: &str) -> Result<(), String> {
        match protocol::parse(text, self.me.as_deref())? {
            Message::Identity(identity) => self.me = Some(identity),
            Message::Subscribed(tables) => {
                self.cache.apply(&tables)?;
                link.set_ready(Ok(()));
                self.publish(link);
            }
            Message::SubscriptionFailed(reason) => return Err(format!("cannot follow the rooms: {reason}")),
            Message::Transaction { tables, call } => {
                self.cache.apply(&tables)?;
                self.publish(link);
                if let Some((id, outcome)) = call {
                    link.finish(id, outcome);
                }
            }
            Message::Other => {}
        }
        Ok(())
    }

    /// Reports this connection's room when it changed.
    fn publish(&mut self, link: &Link) {
        let Some(me) = self.me.as_deref() else {
            return;
        };
        let view = self.cache.view_of(me);
        if self.last.as_ref() == Some(&view) {
            return;
        }
        self.last = Some(view.clone());
        if !link.closed.load(Ordering::SeqCst) {
            (link.events)(BackendEvent::Room(view));
        }
    }
}

/// Closes the WebSocket and waits a moment for the server to answer, so it sees a clean close.
fn say_goodbye(socket: &mut Socket) {
    let frame = CloseFrame {
        code: CloseCode::Normal,
        reason: "".into(),
    };
    if socket.close(Some(frame)).is_err() {
        return;
    }
    let deadline = Instant::now() + CLOSE_WAIT;
    while Instant::now() < deadline {
        match socket.read() {
            Ok(_) => {}
            Err(Error::Io(error)) if error.kind() == ErrorKind::WouldBlock => {}
            Err(_) => return,
        }
    }
}
