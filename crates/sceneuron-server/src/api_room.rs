// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The shared room endpoint: the room's state for the editor, and its actions. A control token may
//! only reveal the votes and start the next round, and its answers carry no names or votes.

use serde_json::json;

use crate::access::Access;
use crate::body::read_json;
use crate::connection::Request;
use crate::http::Reply;
use crate::room::{Room, RoomAction, RoomError, RoomRequest};

type ApiResult = Result<Reply, Reply>;

const CONTROL_ONLY: &str = "the control token can only reveal the votes and start the next round";

pub(crate) fn state(room: &dyn Room, access: Access) -> ApiResult {
    if matches!(access, Access::Control { .. }) {
        return Err(Reply::error(403, CONTROL_ONLY));
    }
    Ok(Reply::json(200, &room.state()))
}

pub(crate) fn act(room: &dyn Room, request: &mut Request, access: Access) -> ApiResult {
    let body: RoomRequest = read_json(request)?;
    let control = matches!(access, Access::Control { .. });
    if control && !matches!(body.action, RoomAction::Reveal | RoomAction::Next) {
        return Err(Reply::error(403, CONTROL_ONLY));
    }
    let state = room.act(body).map_err(refusal)?;
    if control {
        return Ok(Reply::json(200, &json!({ "ok": true })));
    }
    Ok(Reply::json(200, &state))
}

/// Keeps the room's status when it is an error status; anything else becomes 500.
fn refusal(error: RoomError) -> Reply {
    let status = if (400..600).contains(&error.status) {
        error.status
    } else {
        500
    };
    Reply::error(status, &error.message)
}
