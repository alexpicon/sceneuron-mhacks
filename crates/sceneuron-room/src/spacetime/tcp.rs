// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The TCP connection under a room's WebSocket. Its reads wait at most [`super::socket::POLL`], and
//! a read that waited that long fails as `WouldBlock`: the TLS and WebSocket handshakes, like the
//! connection's thread, take that as a reason to read again. The standard library allows such a
//! read to fail as `TimedOut` instead, which they take as a failure, so a server slower than one
//! poll could never be reached; this maps `TimedOut` to `WouldBlock`.

use std::io::{self, ErrorKind, Read, Write};
use std::net::TcpStream;

/// A stream whose reads that time out fail as `WouldBlock`.
pub(super) struct Tcp<S = TcpStream>(pub(super) S);

impl<S: Read> Read for Tcp<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.0.read(buf) {
            Err(error) if error.kind() == ErrorKind::TimedOut => Err(ErrorKind::WouldBlock.into()),
            other => other,
        }
    }
}

impl<S: Write> Write for Tcp<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;
    use std::sync::atomic::AtomicBool;
    use std::thread;
    use std::time::{Duration, Instant};

    use tungstenite::client::IntoClientRequest;

    use super::*;
    use crate::spacetime::socket::{POLL, handshake};

    /// A stream that reports a read that timed out as `TimedOut`.
    struct TimingOut(TcpStream);

    impl Read for TimingOut {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.0.read(buf).map_err(|error| match error.kind() {
                ErrorKind::WouldBlock => ErrorKind::TimedOut.into(),
                _ => error,
            })
        }
    }

    impl Write for TimingOut {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }

    #[test]
    fn a_read_that_times_out_fails_as_would_block_on_this_platform() {
        let silent = TcpListener::bind("127.0.0.1:0").unwrap();
        let stream = TcpStream::connect(silent.local_addr().unwrap()).unwrap();
        stream.set_read_timeout(Some(POLL)).unwrap();
        let error = Tcp(stream).read(&mut [0; 16]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::WouldBlock);
    }

    #[test]
    fn the_handshake_waits_for_a_slow_server_when_reads_time_out_as_timed_out() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            thread::sleep(Duration::from_millis(100));
            tungstenite::accept(stream).map_err(|error| error.to_string())
        });
        let stream = TcpStream::connect(address).unwrap();
        stream.set_read_timeout(Some(POLL)).unwrap();
        let mut timing_out = TimingOut(stream);
        let error = timing_out.read(&mut [0; 16]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::TimedOut, "reads time out as TimedOut");
        let request = format!("ws://{address}/v1/database/rooms/subscribe")
            .into_client_request()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        handshake(request, Tcp(timing_out), &AtomicBool::new(false), deadline)
            .expect("the handshake read again after each timeout");
        server.join().unwrap().expect("the server finished the handshake");
    }
}
