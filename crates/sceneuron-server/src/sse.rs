// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Server-sent event streams of snapshots or control views, each served on its connection's
//! thread.

use std::io::{self, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::connection::Connection;
use crate::control_view::{ControlView, view_to_send};
use crate::head::Version;
use crate::http::Reply;
use crate::server::Context;
use crate::{date, lock, to_json};

/// Most streams open at once; each holds a connection, a thread, and a subscriber channel.
pub(crate) const MAX_STREAMS: usize = 32;

/// Idle time after which a comment keeps proxies and clients from closing the stream.
const PING_INTERVAL: Duration = Duration::from_secs(15);

/// How often a stream checks whether the server is stopping or the client left.
const POLL: Duration = Duration::from_millis(100);

/// What a stream sends.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Feed {
    /// Every new snapshot, as `state` events.
    State,
    /// The control view, as `control` events, only when it shows something new. A stream
    /// opened with the control token carries its `generation` and ends when it is revoked.
    Control { generation: Option<u64> },
}

/// Counts open streams against [`MAX_STREAMS`].
#[derive(Debug, Default)]
pub(crate) struct Streams {
    open: Mutex<usize>,
}

/// Holds a place in [`Streams`] until the stream ends.
struct Slot<'a>(&'a Streams);

impl<'a> Slot<'a> {
    fn take(streams: &'a Streams) -> Option<Self> {
        let mut open = lock(&streams.open);
        if *open >= MAX_STREAMS {
            return None;
        }
        *open += 1;
        Some(Slot(streams))
    }
}

impl Drop for Slot<'_> {
    fn drop(&mut self) {
        *lock(&self.0.open) -= 1;
    }
}

/// Streams the feed to the client until it leaves, the hub shuts down, the server stops,
/// or the control token the stream was opened with is revoked.
pub(crate) fn serve(context: &Context, connection: &mut Connection, version: Version, feed: Feed) {
    let Some(_slot) = Slot::take(&context.streams) else {
        connection.send(&Reply::error(503, "too many event streams are open"));
        return;
    };
    let stream = connection.stream();
    let mut writer = EventWriter {
        out: stream,
        chunked: version == Version::Http11,
    };
    // A write error means the client went away; there is nobody left to tell.
    let _ = match feed {
        Feed::State => {
            // Each snapshot is complete, so only the newest queued one matters.
            let frame = |batch: Vec<Arc<str>>| batch.last().map(|snapshot| event("state", snapshot));
            pump(&mut writer, &context.hub.subscribe(), context, stream, frame, || false)
        }
        Feed::Control { generation } => {
            let mut last = None;
            let frame = |batch: Vec<Arc<ControlView>>| {
                view_to_send(&mut last, batch).map(|view| event("control", &to_json(&*view)))
            };
            let revoked = || generation.is_some_and(|generation| context.control.generation() != generation);
            pump(
                &mut writer,
                &context.hub.subscribe_control(),
                context,
                stream,
                frame,
                revoked,
            )
        }
    };
}

/// Sends what `frame` makes of each batch of queued messages, oldest first, until the
/// stream should end.
fn pump<T>(
    writer: &mut EventWriter,
    receiver: &Receiver<T>,
    context: &Context,
    stream: &TcpStream,
    mut frame: impl FnMut(Vec<T>) -> Option<String>,
    revoked: impl Fn() -> bool,
) -> io::Result<()> {
    writer.start()?;
    let mut last_write = Instant::now();
    while !context.stopping.load(Ordering::Acquire) && !revoked() {
        match receiver.recv_timeout(POLL) {
            Ok(first) => {
                let mut batch = vec![first];
                batch.extend(receiver.try_iter());
                if let Some(text) = frame(batch) {
                    writer.frame(&text)?;
                    last_write = Instant::now();
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        // Checked after messages too: a control stream can receive views it does not send.
        if last_write.elapsed() >= PING_INTERVAL {
            writer.ping()?;
            last_write = Instant::now();
        }
        if client_left(stream) {
            return Ok(());
        }
    }
    writer.finish()
}

/// Whether the client closed its side. Event stream clients send nothing after their
/// request, so a pending end of input means they left; any stray bytes are discarded.
/// Without this, a client that left is noticed only when a later write fails.
fn client_left(stream: &TcpStream) -> bool {
    if stream.set_nonblocking(true).is_err() {
        return true;
    }
    let mut reader = stream;
    let mut probe = [0u8; 256];
    let left = match reader.read(&mut probe) {
        Ok(0) => true,
        Ok(_) => false,
        Err(error) => !matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted),
    };
    stream.set_nonblocking(false).is_err() || left
}

fn event(name: &str, json: &str) -> String {
    format!("event: {name}\ndata: {json}\n\n")
}

/// Writes each event as soon as it happens, in one write so that it leaves as one packet.
struct EventWriter<'a> {
    out: &'a TcpStream,
    /// HTTP/1.1 frames the body in chunks so the end of the stream is explicit.
    chunked: bool,
}

impl EventWriter<'_> {
    fn start(&mut self) -> io::Result<()> {
        let (version, framing) = if self.chunked {
            ("1.1", "Transfer-Encoding: chunked\r\n")
        } else {
            ("1.0", "")
        };
        let head = format!(
            "HTTP/{version} 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\n\
             X-Content-Type-Options: nosniff\r\nConnection: close\r\nDate: {}\r\n{framing}\r\n",
            date::now()
        );
        self.out.write_all(head.as_bytes())
    }

    fn ping(&mut self) -> io::Result<()> {
        self.frame(": ping\n\n")
    }

    fn frame(&mut self, text: &str) -> io::Result<()> {
        if self.chunked {
            self.out.write_all(format!("{:x}\r\n{text}\r\n", text.len()).as_bytes())
        } else {
            self.out.write_all(text.as_bytes())
        }
    }

    fn finish(&mut self) -> io::Result<()> {
        if self.chunked {
            self.out.write_all(b"0\r\n\r\n")?;
        }
        Ok(())
    }
}
