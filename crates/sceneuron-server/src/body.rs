// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Request bodies: the size limit and JSON parsing.

use serde::de::DeserializeOwned;

use crate::connection::Request;
use crate::http::Reply;

/// Largest accepted request body.
pub(crate) const MAX_BODY: usize = 256 * 1024;

/// Reads and parses a JSON body of at most [`MAX_BODY`] bytes.
pub(crate) fn read_json<T: DeserializeOwned>(request: &mut Request) -> Result<T, Reply> {
    let length = usize::try_from(request.head.content_length)
        .ok()
        .filter(|length| *length <= MAX_BODY)
        .ok_or_else(too_large)?;
    let expects_continue = request.head.expects_continue();
    let body = request.connection.read_body(length, expects_continue)?;
    serde_json::from_slice(&body).map_err(|error| Reply::error(400, &format!("invalid request body: {error}")))
}

pub(crate) fn too_large() -> Reply {
    Reply::error(413, &format!("request bodies are limited to {} KiB", MAX_BODY / 1024))
}
