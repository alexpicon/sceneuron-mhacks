// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Proof that the server holds the tokens, asked for before a page sends one.
//!
//! When OBS quits or moves the editor to another port, another program can listen on the old
//! port. Pages and control surfaces therefore ask `GET /api/hello?nonce=N` first, with no
//! token, and send their token only when the answer is HMAC-SHA256 of `N:port` under that
//! token for the port they are on.
//! The port is part of the message, so a program on the old port cannot pass on the real
//! server's answer from the new one.

use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;

use crate::http::Reply;
use crate::server::Context;

const MIN_NONCE_CHARS: usize = 16;
const MAX_NONCE_CHARS: usize = 128;

/// Answers a hello for each token the server holds.
pub(crate) fn hello(context: &Context, nonce: Option<String>) -> Reply {
    let Some(nonce) = nonce.filter(|nonce| {
        (MIN_NONCE_CHARS..=MAX_NONCE_CHARS).contains(&nonce.len()) && nonce.bytes().all(|b| b.is_ascii_hexdigit())
    }) else {
        let message = format!("nonce must be {MIN_NONCE_CHARS}-{MAX_NONCE_CHARS} hexadecimal characters");
        return Reply::error(400, &message);
    };
    let message = format!("{nonce}:{}", context.port);
    Reply::json(
        200,
        &json!({
            "editor": proof(&context.token, &message),
            "overlay": proof(&context.overlay_token, &message),
            "control": context.control.proof(&message),
        }),
    )
}

/// HMAC-SHA256 of `message` under `key`, as lowercase hexadecimal.
pub fn proof(key: &str, message: &str) -> String {
    // HMAC accepts keys of any length, so this cannot fail.
    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes()).unwrap_or_else(|_| unreachable!());
    mac.update(message.as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
