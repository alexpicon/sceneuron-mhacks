// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A scripted HTTP/1.1 server on 127.0.0.1 that answers each connection with the next canned
//! reply and records what it received.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::Value;

const IO_TIMEOUT: Duration = Duration::from_secs(10);

/// One canned response.
#[derive(Clone, Debug)]
pub struct Reply {
    status: u16,
    content_type: &'static str,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    delay: Duration,
}

impl Reply {
    pub fn json(status: u16, body: Value) -> Reply {
        Reply::bytes(status, "application/json", body.to_string().into_bytes())
    }

    /// Any body, such as audio.
    pub fn bytes(status: u16, content_type: &'static str, body: Vec<u8>) -> Reply {
        Reply {
            status,
            content_type,
            headers: Vec::new(),
            body,
            delay: Duration::ZERO,
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Reply {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }

    /// Waits this long before answering.
    pub fn delayed(mut self, delay: Duration) -> Reply {
        self.delay = delay;
        self
    }
}

/// A request as the server received it.
#[derive(Clone, Debug)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Recorded {
    /// A header value, matched without regard to case.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("request body is JSON")
    }
}

/// Serves the replies in order, one per connection; later connections get a 500.
pub struct MockServer {
    pub port: u16,
    requests: Arc<Mutex<Vec<Recorded>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl MockServer {
    pub fn start(replies: Vec<Reply>) -> MockServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (recorded, stopped) = (Arc::clone(&requests), Arc::clone(&stop));
        let thread = thread::spawn(move || serve(listener, replies.into(), &recorded, &stopped));
        MockServer {
            port,
            requests,
            stop,
            thread: Some(thread),
        }
    }

    /// `http://127.0.0.1:<port>`.
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(listener: TcpListener, mut replies: VecDeque<Reply>, requests: &Mutex<Vec<Recorded>>, stop: &AtomicBool) {
    for stream in listener.incoming() {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        let Ok(mut stream) = stream else { continue };
        let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
        let Some(request) = read_request(&mut stream) else {
            continue;
        };
        requests.lock().unwrap().push(request);
        let reply = replies.pop_front().unwrap_or_else(|| {
            Reply::json(
                500,
                serde_json::json!({"error": {"message": "no more scripted replies"}}),
            )
        });
        thread::sleep(reply.delay);
        let mut head = format!(
            "HTTP/1.1 {} Scripted\r\ncontent-type: {}\r\ncontent-length: {}\r\nconnection: close\r\n",
            reply.status,
            reply.content_type,
            reply.body.len()
        );
        for (name, value) in &reply.headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        head.push_str("\r\n");
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(&reply.body);
        let _ = stream.flush();
    }
}

fn read_request(stream: &mut TcpStream) -> Option<Recorded> {
    let mut raw = Vec::new();
    let mut buffer = [0u8; 8192];
    let head_end = loop {
        if let Some(end) = raw.windows(4).position(|window| window == b"\r\n\r\n") {
            break end;
        }
        let read = stream.read(&mut buffer).ok()?;
        if read == 0 {
            return None;
        }
        raw.extend_from_slice(&buffer[..read]);
    };
    let head = String::from_utf8_lossy(&raw[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let mut request_line = lines.next()?.split(' ');
    let (method, path) = (request_line.next()?.to_owned(), request_line.next()?.to_owned());
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
        .collect();
    let length: usize = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse().ok())
        .unwrap_or(0);
    let mut body = raw[head_end + 4..].to_vec();
    while body.len() < length {
        let read = stream.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&buffer[..read]);
    }
    Some(Recorded {
        method,
        path,
        headers,
        body,
    })
}
