// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The editor's Camera & mic endpoints: the camera preview and what on-device analysis finds,
//! the sources that carry audio, which the Sceneuron Voice filter turns into the microphone for
//! voice control and subtitles, and setting up OBS in one click, the Stream Deck included. Only
//! the editor token reaches them.

use crate::api::AttachBody;
use crate::body::read_json;
use crate::connection::Request;
use crate::http::Reply;
use crate::server::Context;
use crate::setup::SetupAnswer;
use crate::{Host, PreviewImage, stream_deck};

type ApiResult = Result<Reply, Reply>;

const JPEG: &str = "image/jpeg";

/// Marks a test picture from a host without a camera.
const SIMULATED_HEADER: &str = "X-Sceneuron-Simulated";

/// A JPEG of what camera analysis sees, or `{"reason": ...}` saying why there is none. Both
/// answer 200, so an editor that polls while no camera is analyzed fills no console with
/// errors. Like every reply, it carries `Cache-Control: no-store`.
pub(crate) fn preview(host: &dyn Host) -> Reply {
    match host.camera_preview() {
        Ok(PreviewImage { jpeg, simulated }) => {
            let reply = Reply::bytes(JPEG, jpeg);
            if simulated {
                reply.with_header(SIMULATED_HEADER, "true")
            } else {
                reply
            }
        }
        Err(reason) => Reply::json(200, &serde_json::json!({ "reason": reason })),
    }
}

/// What on-device analysis found in the latest analyzed frame.
pub(crate) fn analysis(host: &dyn Host) -> Reply {
    Reply::json(200, &host.camera_analysis())
}

/// The sources that carry audio; `available` is false outside OBS, where there are none.
pub(crate) fn audio_sources(host: &dyn Host) -> Reply {
    let sources = host.list_audio_sources();
    let body = serde_json::json!({ "available": sources.is_some(), "sources": sources.unwrap_or_default() });
    Reply::json(200, &body)
}

/// Adds the Sceneuron Voice filter to the source with the UUID in the body.
pub(crate) fn attach_voice(host: &dyn Host, request: &mut Request) -> ApiResult {
    let body: AttachBody = read_json(request)?;
    host.attach_voice_filter(&body.uuid)
        .map_err(|message| Reply::error(400, &message))?;
    Ok(Reply::json(200, &serde_json::json!({ "ok": true })))
}

/// "Set up OBS for me": puts the overlay, camera analysis, and the voice filter in place where
/// they are missing, then pairs the Stream Deck when none is, and answers what was added and
/// what was skipped. A host that cannot set up OBS at all sends no pairing link either.
pub(crate) fn set_up(context: &Context) -> ApiResult {
    let obs = context.host.set_up().map_err(|message| Reply::error(400, &message))?;
    let stream_deck = stream_deck::pair(context);
    Ok(Reply::json(200, &SetupAnswer { obs, stream_deck }))
}
