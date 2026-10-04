// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A stand-in for a SpacetimeDB server on 127.0.0.1: it accepts one WebSocket on a database's
//! subscribe path with the JSON text protocol, and a test scripts what it sends and checks what it
//! receives, in the shapes SpacetimeDB 2.10.2 uses.

#![allow(dead_code)]

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::{Value, json};
use tungstenite::handshake::server::{Request, Response};
use tungstenite::http::HeaderValue;
use tungstenite::protocol::CloseFrame;
use tungstenite::{Error, Message, WebSocket};

pub const ANA: &str = "c20002be6f41d29b2ff483a43fb19ab53d86eae569c224497d2707eaab4ffcbf";
pub const BEN: &str = "c2006005700b59274b6b83b451d9e1efecd2b01afb10256af1cbf35b85ae0782";
const PROTOCOL: &str = "v1.json.spacetimedb";

/// One accepted WebSocket, as the server sees it.
pub struct Peer(WebSocket<TcpStream>);

/// A server running a script on the first connection; `finish` returns what the script returned.
pub struct Wire<T> {
    pub server: String,
    script: JoinHandle<T>,
}

impl<T> Wire<T> {
    pub fn finish(self) -> T {
        self.script.join().expect("the server's script panicked")
    }
}

/// Serves the WebSocket of database `rooms` and runs `script` on the first connection.
// tungstenite's accept callback returns its large error response type.
#[allow(clippy::result_large_err)]
pub fn serve<T: Send + 'static>(script: impl FnOnce(&mut Peer) -> T + Send + 'static) -> Wire<T> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let server = format!("http://{}", listener.local_addr().unwrap());
    let script = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let socket = tungstenite::accept_hdr(stream, |request: &Request, mut response: Response| {
            assert_eq!(request.uri().path(), "/v1/database/rooms/subscribe");
            assert_eq!(request.headers()["Sec-WebSocket-Protocol"], PROTOCOL);
            let chosen = HeaderValue::from_static(PROTOCOL);
            response.headers_mut().insert("Sec-WebSocket-Protocol", chosen);
            Ok(response)
        })
        .unwrap();
        script(&mut Peer(socket))
    });
    Wire { server, script }
}

/// Answers the first HTTP request with `status` and `body`, as a server without the database does.
pub fn refuse(status: &str, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let server = format!("http://{}", listener.local_addr().unwrap());
    let status = status.to_owned();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut head = [0u8; 4096];
        let _ = stream.read(&mut head);
        let reply = format!("HTTP/1.1 {status}\r\ncontent-length: {}\r\n\r\n{body}", body.len());
        let _ = stream.write_all(reply.as_bytes());
    });
    server
}

impl Peer {
    pub fn send(&mut self, message: &Value) {
        self.0.send(Message::text(message.to_string())).unwrap();
    }

    /// The next text message, as JSON.
    pub fn next(&mut self) -> Value {
        loop {
            match self.0.read().unwrap() {
                Message::Text(text) => return serde_json::from_str(text.as_str()).unwrap(),
                Message::Close(frame) => panic!("the client closed: {frame:?}"),
                _ => {}
            }
        }
    }

    /// Waits for the client's close frame.
    pub fn closing(&mut self) -> Option<CloseFrame> {
        loop {
            match self.0.read() {
                Ok(Message::Close(frame)) => return frame,
                Ok(_) => {}
                Err(Error::Io(error)) if error.kind() == ErrorKind::WouldBlock => panic!("the client did not close"),
                Err(error) => panic!("the connection ended without a close: {error}"),
            }
        }
    }

    /// Waits for the client's close frame, answering its pings as a server does but never pinging,
    /// and counts the pings.
    pub fn answer_pings(&mut self) -> (usize, Option<CloseFrame>) {
        let mut pings = 0;
        loop {
            match self.0.read() {
                Ok(Message::Ping(_)) => pings += 1,
                Ok(Message::Close(frame)) => return (pings, frame),
                Ok(_) => {}
                Err(error) => panic!("the connection ended without a close: {error}"),
            }
        }
    }

    pub fn close(&mut self, frame: CloseFrame) {
        self.0.close(Some(frame)).unwrap();
        while self.0.read().is_ok() {}
    }

    /// Sends the identity, then answers the subscription with `rows`, as the server starts.
    pub fn greet(&mut self, identity: &str, rows: Vec<Value>) {
        self.send(&json!({"IdentityToken": {
            "identity": {"__identity__": format!("0x{identity}")},
            "token": "eyJ0",
            "connection_id": {"__connection_id__": 1468235083487063790u64},
        }}));
        let subscribe = self.next();
        let request_id = subscribe["Subscribe"]["request_id"].clone();
        self.send(&json!({"InitialSubscription": {
            "database_update": {"tables": rows},
            "request_id": request_id,
            "total_host_execution_duration": {"__time_duration_micros__": 900},
        }}));
    }
}

/// A table's inserted rows, each the JSON text of a product value.
pub fn inserts(table: &str, rows: &[Value]) -> Value {
    let rows: Vec<String> = rows.iter().map(Value::to_string).collect();
    json!({"table_id": 4096, "table_name": table, "num_rows": rows.len(), "updates": [{"deletes": [], "inserts": rows}]})
}

/// The full update SpacetimeDB sends the caller of a reducer.
pub fn answer(caller: &str, call: &Value, status: Value) -> Value {
    let call = &call["CallReducer"];
    json!({"TransactionUpdate": {
        "status": status,
        "timestamp": {"__timestamp_micros_since_unix_epoch__": 1790974996000000i64},
        "caller_identity": {"__identity__": format!("0x{caller}")},
        "caller_connection_id": {"__connection_id__": 1},
        "reducer_call": {"reducer_name": call["reducer"], "reducer_id": 2, "args": call["args"], "request_id": call["request_id"]},
        "energy_quanta_used": {"quanta": 20091},
        "total_host_execution_duration": {"__time_duration_micros__": 1281},
    }})
}

/// The update every other subscriber gets.
pub fn light(tables: Vec<Value>) -> Value {
    json!({"TransactionUpdateLight": {"request_id": 0, "update": {"tables": tables}}})
}
