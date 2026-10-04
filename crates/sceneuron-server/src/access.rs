// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What the token a request presents allows.

use crate::head::Head;
use crate::http::query_param;
use crate::server::Context;
use crate::tokens_match;

pub(crate) const TOKEN_HEADER: &str = "X-Sceneuron-Token";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Access {
    None,
    /// The overlay token: reading state and events only.
    Overlay,
    /// The control token, for the control requests only. `generation` tells which control
    /// token it was, so its event streams end when that token is revoked.
    Control {
        generation: u64,
    },
    Full,
}

/// The access a request's token gives.
///
/// The header wins over the query parameter when both are present. The control token counts
/// only in the header: it is never put in an address, where logs and history could keep it. A
/// request with it shows that a control surface is paired.
pub(crate) fn access(context: &Context, head: &Head, query: &str) -> Access {
    let header = head.header_values(TOKEN_HEADER).next();
    let Some(token) = header.map(str::to_owned).or_else(|| query_param(query, "token")) else {
        return Access::None;
    };
    if tokens_match(&context.token, &token) {
        return Access::Full;
    }
    if tokens_match(&context.overlay_token, &token) {
        return Access::Overlay;
    }
    if header.is_none() {
        return Access::None;
    }
    context
        .control
        .accept(&token)
        .map_or(Access::None, |generation| Access::Control { generation })
}
