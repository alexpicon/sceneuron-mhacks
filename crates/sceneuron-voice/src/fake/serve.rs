// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! One connection to the fake server: the handshake, the session, and the scripted answers.

use std::io;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use serde_json::{Value, json};
use tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tungstenite::http::StatusCode;
use tungstenite::{Error as WsError, Message, WebSocket};

use super::{FakeSession, Shared, Stream, lock};

/// Every `vad` segment is this much audio unless the script says otherwise: one second.
const DEFAULT_VAD_BYTES: usize = 32_000;

pub(super) fn connection(
    tcp: TcpStream,
    tls: Option<Arc<ServerConfig>>,
    state: &Shared,
    stop: &AtomicBool,
) -> Result<(), String> {
    let setup = |tcp: &TcpStream| {
        tcp.set_nonblocking(false)?;
        tcp.set_read_timeout(Some(Duration::from_secs(5)))
    };
    setup(&tcp).map_err(|error| error.to_string())?;
    let stream = match tls {
        Some(config) => {
            let connection = ServerConnection::new(config).map_err(|error| error.to_string())?;
            Stream::Tls(Box::new(StreamOwned::new(connection, tcp)))
        }
        None => Stream::Plain(tcp),
    };
    let mut session = FakeSession::default();
    let http_refusal = {
        let mut state = lock(state);
        state.connections += 1;
        state.script.http_refusals.get(state.connections - 1).cloned().flatten()
    };
    // tungstenite's handshake callback fixes the error type, an HTTP response.
    #[allow(clippy::result_large_err)]
    let callback = |request: &Request, response: Response| {
        session.path = request.uri().path().to_owned();
        session.query = decode_query(request.uri().query().unwrap_or_default());
        let key = request
            .headers()
            .get("xi-api-key")
            .and_then(|value| value.to_str().ok());
        session.key = key.map(str::to_owned);
        match &http_refusal {
            Some((status, body)) => Err(refused(ErrorResponse::new(Some(body.clone())), *status)),
            None => Ok(response),
        }
    };
    let accepted = tungstenite::accept_hdr(stream, callback).map_err(|error| error.to_string());
    if let Some((status, _)) = http_refusal {
        drop(accepted);
        session.http_refused = Some(status);
        session.ended = true;
        lock(state).sessions.push(session);
        return Ok(());
    }
    let mut socket = accepted?;
    let (index, refusal, close_after, vad_bytes) = {
        let mut state = lock(state);
        let index = state.sessions.len();
        let refusal = if session.key.as_deref() == Some(state.script.key.as_str()) {
            state.script.refusals.get(index).cloned().flatten()
        } else {
            Some(("auth_error".to_owned(), "Invalid API key".to_owned()))
        };
        let close_after = if refusal.is_none() {
            state.started += 1;
            state.script.close_after.get(state.started - 1).copied().flatten()
        } else {
            None
        };
        let vad_bytes = match state.script.vad_bytes {
            0 => DEFAULT_VAD_BYTES,
            bytes => bytes,
        };
        state.sessions.push(session.clone());
        (index, refusal, close_after, vad_bytes)
    };
    if let Some((code, error)) = refusal {
        send(&mut socket, json!({"message_type": code, "error": error}));
    } else {
        let asked = |wanted: &str, setting: &str| {
            let mut query = session.query.iter();
            query.any(|(name, value)| name == wanted && value == setting)
        };
        let vad = asked("commit_strategy", "vad");
        let detects = asked("include_language_detection", "true");
        send(
            &mut socket,
            json!({
                "message_type": "session_started",
                "session_id": format!("fake-{index}"),
                "config": {"sample_rate": 16000, "audio_format": "pcm_16000", "model_id": "scribe_v2_realtime"},
            }),
        );
        tcp_of(&socket)
            .set_read_timeout(Some(Duration::from_millis(20)))
            .map_err(|error| error.to_string())?;
        let mut segment = 0;
        while !stop.load(Ordering::SeqCst) {
            let text = match socket.read() {
                Ok(Message::Text(text)) => text,
                Ok(Message::Close(_)) => break,
                Ok(_) => continue,
                Err(WsError::Io(error)) if waiting(&error) => continue,
                Err(_) => break,
            };
            let value: Value = serde_json::from_str(text.as_str()).unwrap_or(Value::Null);
            if value["message_type"] != "input_audio_chunk" {
                lock(state).sessions[index]
                    .other_messages
                    .push(text.as_str().to_owned());
                continue;
            }
            let audio = STANDARD
                .decode(value["audio_base_64"].as_str().unwrap_or_default())
                .unwrap_or_default();
            let commit = value["commit"].as_bool() == Some(true);
            segment += audio.len();
            let messages = {
                let mut state = lock(state);
                let recorded = &mut state.sessions[index];
                recorded.audio.extend_from_slice(&audio);
                recorded.audio_messages += 1;
                recorded.commits += usize::from(commit);
                recorded.sample_rates.extend(value["sample_rate"].as_u64());
                recorded.audio_messages
            };
            if commit || (vad && segment >= vad_bytes) {
                segment = 0;
                answer(&mut socket, state, commit, detects);
            }
            if close_after == Some(messages) {
                break;
            }
        }
    }
    finish(&mut socket);
    lock(state).sessions[index].ended = true;
    Ok(())
}

/// Sends the next scripted transcript: a partial with its first word, then the final text, and,
/// when the session `detects` the language and the script names one, the same text with its
/// language as a transcript with timestamps. A commit with nothing left to say gets an empty final
/// transcript, as for silence.
fn answer(socket: &mut WebSocket<Stream>, state: &Shared, commit: bool, detects: bool) {
    let (text, language) = {
        let mut state = lock(state);
        let text = state.script.transcripts.get(state.next_transcript).cloned();
        state.next_transcript += usize::from(text.is_some());
        (text, state.script.language.clone())
    };
    match text {
        Some(text) => {
            if let Some(first) = text.split_whitespace().next() {
                send(socket, json!({"message_type": "partial_transcript", "text": first}));
            }
            send(socket, json!({"message_type": "committed_transcript", "text": text}));
            if detects && !language.is_empty() {
                let message = "committed_transcript_with_timestamps";
                send(
                    socket,
                    json!({"message_type": message, "text": text, "language_code": language, "words": []}),
                );
            }
        }
        None if commit => send(socket, json!({"message_type": "committed_transcript", "text": ""})),
        None => {}
    }
}

/// `response` with `status`, or a 500 when `status` is not one.
fn refused(mut response: ErrorResponse, status: u16) -> ErrorResponse {
    *response.status_mut() = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    response
}

fn send(socket: &mut WebSocket<Stream>, message: Value) {
    let _ = socket.send(Message::text(message.to_string()));
}

/// Closes the session and waits briefly for the client's reply.
fn finish(socket: &mut WebSocket<Stream>) {
    let _ = socket.close(None);
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        match socket.read() {
            Ok(_) => {}
            Err(WsError::Io(error)) if waiting(&error) => {}
            Err(_) => break,
        }
    }
}

fn tcp_of(socket: &WebSocket<Stream>) -> &TcpStream {
    match socket.get_ref() {
        Stream::Plain(tcp) => tcp,
        Stream::Tls(tls) => tls.get_ref(),
    }
}

fn waiting(error: &io::Error) -> bool {
    matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut)
}

fn decode_query(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            (decode(name), decode(value))
        })
        .collect()
}

fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let hex = bytes
            .get(at + 1..at + 3)
            .and_then(|pair| std::str::from_utf8(pair).ok());
        match (bytes[at], hex.and_then(|pair| u8::from_str_radix(pair, 16).ok())) {
            (b'%', Some(byte)) => {
                decoded.push(byte);
                at += 3;
            }
            (b'+', _) => {
                decoded.push(b' ');
                at += 1;
            }
            (byte, _) => {
                decoded.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}
