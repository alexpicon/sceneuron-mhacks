// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Shared helpers for the sceneuron-dev binary tests: running the binary, reading the URL it
//! prints, raw HTTP requests to its API, project files under the cargo target dir, and voice
//! fixtures.

#![allow(dead_code)]

pub mod voice;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, ExitStatus, Output, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

pub const BINARY: &str = env!("CARGO_BIN_EXE_sceneuron-dev");
pub const WAIT: Duration = Duration::from_secs(10);

pub fn output(args: &[&str]) -> Output {
    Command::new(BINARY).args(args).stdin(Stdio::null()).output().unwrap()
}

/// A running binary whose first lines announced the editor URL.
pub struct Running {
    pub child: Child,
    pub stdin: Option<ChildStdin>,
    pub lines: Receiver<String>,
    pub port: u16,
    pub token: String,
}

impl Running {
    pub fn start(args: &[&str]) -> Self {
        Self::start_with_env(args, &[])
    }

    /// Like [`Running::start`], with extra environment variables.
    pub fn start_with_env(args: &[&str], env: &[(&str, &str)]) -> Self {
        let mut child = Command::new(BINARY)
            .args(args)
            .envs(env.iter().copied())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let (sender, lines) = channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if sender.send(line).is_err() {
                    return;
                }
            }
        });
        let mut running = Self {
            child,
            stdin,
            lines,
            port: 0,
            token: String::new(),
        };
        let editor = running.line_starting("Editor:");
        let url = editor.trim_start_matches("Editor:").trim();
        let rest = url.strip_prefix("http://127.0.0.1:").unwrap();
        let (port, query) = rest.split_once("/editor#token=").unwrap();
        running.port = port.parse().unwrap();
        running.token = query.to_owned();
        running
    }

    pub fn line_starting(&self, prefix: &str) -> String {
        let deadline = Instant::now() + WAIT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let line = self
                .lines
                .recv_timeout(left)
                .unwrap_or_else(|_| panic!("no line {prefix:?}"));
            if line.starts_with(prefix) {
                return line;
            }
        }
    }

    pub fn request(&self, method: &str, path: &str, body: &str) -> (u16, Value) {
        self.request_with(method, path, "", body)
    }

    /// Sends a request with extra header lines, each ending in CRLF.
    pub fn request_with(&self, method: &str, path: &str, headers: &str, body: &str) -> (u16, Value) {
        self.send(&self.token, method, path, headers, body)
    }

    /// Sends a request with `token` instead of the editor token.
    pub fn request_as(&self, token: &str, method: &str, path: &str, body: &str) -> (u16, Value) {
        self.send(token, method, path, "", body)
    }

    fn send(&self, token: &str, method: &str, path: &str, headers: &str, body: &str) -> (u16, Value) {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream.set_read_timeout(Some(WAIT)).unwrap();
        let head = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\nX-Sceneuron-Token: {}\r\n\
             {headers}Content-Length: {}\r\n\r\n{body}",
            self.port,
            token,
            body.len()
        );
        stream.write_all(head.as_bytes()).unwrap();
        let mut reply = String::new();
        stream.read_to_string(&mut reply).unwrap();
        let (head, body) = reply.split_once("\r\n\r\n").unwrap();
        let status = head.split(' ').nth(1).unwrap().parse().unwrap();
        (status, serde_json::from_str(body).unwrap())
    }

    /// Types `line` on the binary's standard input.
    pub fn say(&mut self, line: &str) {
        let stdin = self.stdin.as_mut().unwrap();
        stdin.write_all(format!("{line}\n").as_bytes()).unwrap();
        stdin.flush().unwrap();
    }

    pub fn quit_with_q(&mut self) -> ExitStatus {
        let mut stdin = self.stdin.take().unwrap();
        stdin.write_all(b"q\n").unwrap();
        self.wait_for_exit()
    }

    pub fn wait_for_exit(&mut self) -> ExitStatus {
        let deadline = Instant::now() + WAIT;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline, "the binary did not exit");
            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn project_path(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("dev-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("project.json")
}

pub fn saved_automation(path: &PathBuf) -> bool {
    let project: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    project["automation_enabled"].as_bool().unwrap()
}
