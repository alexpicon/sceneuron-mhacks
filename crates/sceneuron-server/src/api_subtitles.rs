// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Subtitle endpoints: the lines, live words, translations, and status the editor and the overlay
//! show, and a simulated line for checking a layout without speaking.

use serde::Deserialize;

use crate::Hub;
use crate::body::read_json;
use crate::connection::Request;
use crate::http::Reply;

type ApiResult = Result<Reply, Reply>;

/// Longest simulated line, in characters.
const MAX_SIMULATED_CHARS: usize = 300;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SimulateBody {
    /// What to show as a final line in the spoken language.
    text: String,
}

/// The subtitles now.
pub(crate) fn subtitles(hub: &Hub) -> Reply {
    Reply::json(200, &hub.read(|engine| engine.subtitles_view()))
}

/// Shows a line as if it were heard, so a layout can be checked without a microphone; it is never
/// translated, and only the fact lands in the event log.
pub(crate) fn simulate(hub: &Hub, request: &mut Request) -> ApiResult {
    let body: SimulateBody = read_json(request)?;
    let count = body.text.trim().chars().count();
    if !(1..=MAX_SIMULATED_CHARS).contains(&count) {
        let message = format!("text must be 1 to {MAX_SIMULATED_CHARS} characters");
        return Err(Reply::error(400, &message));
    }
    hub.update(|engine, now| engine.simulate_subtitle(&body.text, now));
    Ok(subtitles(hub))
}
