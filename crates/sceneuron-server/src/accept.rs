// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The accept loop: admits connections up to the limit and serves each on a thread of
//! its own, and refuses the rest at once.

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::connection;
use crate::http::Reply;
use crate::server::Context;

/// The pause after an accept error such as running out of descriptors, which would
/// otherwise repeat at once.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(50);

/// How long stopping waits for the loop to end before it tries to wake it again.
const WAKE_RETRY: Duration = Duration::from_millis(100);

/// Accepts connections until the server stops; returns the connection threads that may
/// still run.
pub(crate) fn run(listener: &TcpListener, context: &Arc<Context>) -> Vec<JoinHandle<()>> {
    let mut workers = Vec::new();
    loop {
        let accepted = listener.accept();
        if context.stopping.load(Ordering::Acquire) {
            return workers;
        }
        reap(&mut workers);
        match accepted {
            Ok((stream, _)) => admit(stream, context, &mut workers),
            Err(error) if matches!(error.kind(), ErrorKind::ConnectionAborted | ErrorKind::Interrupted) => {}
            Err(_) => thread::sleep(ACCEPT_BACKOFF),
        }
    }
}

/// Ends a [`run`] whose context is stopping, and returns its connection threads; empty
/// if it did not end before `deadline`.
///
/// The loop blocks in `accept` and checks for a stop after every connection, so each
/// attempt to wake it is a connection to its own address. An attempt may take all the time
/// left: a loop that is not busy accepting has room in its backlog, so a connection that
/// is slow to open (as when a network filter inspects it) is still on its way, and
/// abandoning it for a new one would only start the wait again.
pub(crate) fn stop(
    acceptor: JoinHandle<Vec<JoinHandle<()>>>,
    address: SocketAddr,
    deadline: Instant,
) -> Vec<JoinHandle<()>> {
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if acceptor.is_finished() || left.is_zero() {
            break;
        }
        let _ = TcpStream::connect_timeout(&address, left);
        let retry = (Instant::now() + WAKE_RETRY).min(deadline);
        join_before(&acceptor, retry);
    }
    if acceptor.is_finished() {
        acceptor.join().unwrap_or_default()
    } else {
        Vec::new()
    }
}

/// Waits until the thread ended or `deadline` passed; true if it ended.
pub(crate) fn join_before<T>(handle: &JoinHandle<T>, deadline: Instant) -> bool {
    while !handle.is_finished() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    handle.is_finished()
}

fn admit(stream: TcpStream, context: &Arc<Context>, workers: &mut Vec<JoinHandle<()>>) {
    let Some(entry) = context.connections.register(&stream) else {
        refuse(stream);
        return;
    };
    let context = Arc::clone(context);
    let spawned = thread::Builder::new()
        .name("sceneuron-connection".to_owned())
        .spawn(move || {
            // Held until `serve` has closed the socket, so the count of open connections
            // never drops before their descriptors are released.
            connection::serve(stream, &context, &entry);
        });
    // If the thread cannot start, dropping the closure closes the socket and frees the place.
    if let Ok(worker) = spawned {
        workers.push(worker);
    }
}

/// Answers 503 without a thread of its own. The socket is nonblocking, so a client that
/// never reads cannot hold up the loop, and what already arrived is read so that closing
/// does not reset the connection before the client sees the answer.
fn refuse(stream: TcpStream) {
    if stream.set_nonblocking(true).is_err() {
        return;
    }
    let reply = Reply::error(503, "too many connections are open");
    let _ = (&stream).write_all(&reply.to_bytes());
    let _ = stream.shutdown(Shutdown::Write);
    let mut sink = [0u8; 4096];
    for _ in 0..4 {
        if !matches!((&stream).read(&mut sink), Ok(read) if read > 0) {
            return;
        }
    }
}

/// Joins the connection threads that already ended.
fn reap(workers: &mut Vec<JoinHandle<()>>) {
    let mut index = 0;
    while index < workers.len() {
        if workers[index].is_finished() {
            let _ = workers.swap_remove(index).join();
        } else {
            index += 1;
        }
    }
}
