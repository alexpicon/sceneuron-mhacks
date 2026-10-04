// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! One connection and its single request: reading the head and the body before a
//! deadline, writing the reply, and closing without resetting what the client has yet
//! to read.

use std::io::{self, ErrorKind, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::time::{Duration, Instant};

use crate::head::{self, Head, HeadEnd, MAX_HEAD};
use crate::http::Reply;
use crate::registry::Entry;
use crate::routes;
use crate::server::Context;

/// The longest a client may take to send its request head; real clients send it at once,
/// so a short limit keeps idle sockets from holding connection places.
const HEAD_TIMEOUT: Duration = Duration::from_secs(2);

/// The longest a single write may wait for the client to take the data.
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);

/// How long a closing connection keeps reading from a client that is still sending.
const LINGER: Duration = Duration::from_millis(500);

/// How much a closing connection reads before it gives up on the client.
const LINGER_BYTES: usize = 1024 * 1024;

/// A request whose head is parsed; its body stays on the connection until a handler
/// reads it.
pub(crate) struct Request<'a> {
    pub(crate) head: Head,
    pub(crate) connection: &'a mut Connection,
}

pub(crate) struct Connection {
    stream: TcpStream,
    /// Bytes that arrived after the head: the start of the body.
    pending: Vec<u8>,
    /// When the whole request must have arrived.
    deadline: Instant,
}

/// Answers the connection's one request, then closes it.
pub(crate) fn serve(stream: TcpStream, context: &Context, entry: &Entry) {
    let _ = stream.set_nodelay(true);
    let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));
    let accepted = Instant::now();
    let mut connection = Connection {
        stream,
        pending: Vec::new(),
        deadline: accepted + context.request_timeout.min(HEAD_TIMEOUT),
    };
    match connection.read_head() {
        Ok(head) => {
            entry.mark_started();
            connection.deadline = accepted + context.request_timeout;
            routes::handle(
                context,
                Request {
                    head,
                    connection: &mut connection,
                },
            );
        }
        Err(Some(reply)) => connection.send(&reply),
        // Nothing arrived, or the client is gone: there is nobody to answer.
        Err(None) => return,
    }
    connection.finish();
}

impl Connection {
    /// The parsed head, an error reply for the client, or `None` when no reply is due.
    fn read_head(&mut self) -> Result<Head, Option<Reply>> {
        let mut received = vec![0u8; MAX_HEAD];
        let mut filled = 0;
        let mut end = HeadEnd::default();
        loop {
            if let Some(length) = end.find(&received[..filled]) {
                self.pending = received[length..filled].to_vec();
                return head::parse(&received[..length]).map_err(Some);
            }
            if filled == MAX_HEAD {
                return Err(Some(head::too_large()));
            }
            match self.read_some(&mut received[filled..]) {
                Ok(0) if filled == 0 => return Err(None),
                Ok(0) => return Err(Some(Reply::error(400, "the request ended inside its head"))),
                Ok(read) => filled += read,
                Err(error) if error.kind() == ErrorKind::TimedOut && filled > 0 => {
                    return Err(Some(Reply::error(408, "the request did not arrive in time")));
                }
                Err(_) => return Err(None),
            }
        }
    }

    /// Reads a body of exactly `length` bytes, which the caller has checked against the
    /// body limit.
    pub(crate) fn read_body(&mut self, length: usize, expects_continue: bool) -> Result<Vec<u8>, Reply> {
        let mut body = std::mem::take(&mut self.pending);
        body.truncate(length);
        if body.len() < length && expects_continue {
            // If this fails, reading the body fails too.
            let _ = (&self.stream).write_all(b"HTTP/1.1 100 Continue\r\n\r\n");
        }
        let mut filled = body.len();
        body.resize(length, 0);
        while filled < length {
            match self.read_some(&mut body[filled..]) {
                Ok(0) => return Err(Reply::error(400, "the request body ended early")),
                Ok(read) => filled += read,
                Err(error) if error.kind() == ErrorKind::TimedOut => {
                    return Err(Reply::error(408, "the request body did not arrive in time"));
                }
                Err(_) => return Err(Reply::error(400, "the request body could not be read")),
            }
        }
        Ok(body)
    }

    pub(crate) fn send(&mut self, reply: &Reply) {
        // A failed write means the client left; there is nobody left to tell.
        let _ = (&self.stream).write_all(&reply.to_bytes());
    }

    /// The socket, for responses written as they happen.
    pub(crate) fn stream(&self) -> &TcpStream {
        &self.stream
    }

    /// Reads into `buffer` until the request deadline; `TimedOut` once it passed.
    fn read_some(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            let left = self.deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(ErrorKind::TimedOut.into());
            }
            self.stream.set_read_timeout(Some(left))?;
            match self.stream.read(buffer) {
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) if error.kind() == ErrorKind::WouldBlock => return Err(ErrorKind::TimedOut.into()),
                result => return result,
            }
        }
    }

    /// Ends the response, then reads what the client still sends for a moment before
    /// closing: closing a socket with unread input resets the connection, and the reset
    /// can destroy the response before the client reads it.
    fn finish(self) {
        if self.stream.shutdown(Shutdown::Write).is_err() {
            return;
        }
        let deadline = Instant::now() + LINGER;
        let mut sink = [0u8; 8192];
        let mut drained = 0;
        while drained < LINGER_BYTES {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() || self.stream.set_read_timeout(Some(left)).is_err() {
                return;
            }
            match (&self.stream).read(&mut sink) {
                Ok(0) => return,
                Ok(read) => drained += read,
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(_) => return,
            }
        }
    }
}
