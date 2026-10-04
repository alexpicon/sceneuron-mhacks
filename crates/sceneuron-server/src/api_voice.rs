// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Voice endpoints: what voice control heard and did, push-to-talk, what can be said, typed
//! commands, and the audio of spoken cards.

use serde::Deserialize;

use crate::Hub;
use crate::api_control::Answer;
use crate::assistant::Assistant;
use crate::body::read_json;
use crate::connection::Request;
use crate::endpoints::speech_id;
use crate::http::Reply;

type ApiResult = Result<Reply, Reply>;

/// Longest typed command, in characters, as long as the heard line keeps.
const MAX_COMMAND_CHARS: usize = 200;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TalkBody {
    /// Whether push-to-talk is held.
    down: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandBody {
    /// What the streamer would say to Sceneuron.
    text: String,
}

/// Whether voice control is on, its mode, whether it listens, and what it last heard and did.
pub(crate) fn voice(hub: &Hub) -> Reply {
    Reply::json(200, &hub.read(|engine| engine.voice_view()))
}

/// Holds or lets go of push-to-talk, so a key can be held to talk.
pub(crate) fn talk(hub: &Hub, request: &mut Request, answer: Answer) -> ApiResult {
    let body: TalkBody = read_json(request)?;
    hub.update(|engine, now| engine.set_talking(body.down, now));
    Ok(answer.reply(hub))
}

/// What can be said to Sceneuron now, by kind, as the matcher reads it.
pub(crate) fn guide(hub: &Hub) -> Reply {
    Reply::json(200, &hub.read(|engine| engine.voice_guide()))
}

/// Carries out typed words as if said with push to talk held, so commands can be tried without
/// speaking, even with voice control off; answers with voice control as `GET /api/voice` shows it.
/// What only the host can do, the AI included, finishes later, as the heard line then says.
pub(crate) fn command(hub: &Hub, request: &mut Request) -> ApiResult {
    let body: CommandBody = read_json(request)?;
    let count = body.text.trim().chars().count();
    if !(1..=MAX_COMMAND_CHARS).contains(&count) {
        let message = format!("text must be 1 to {MAX_COMMAND_CHARS} characters");
        return Err(Reply::error(400, &message));
    }
    hub.update(|engine, now| engine.type_command(&body.text, now));
    Ok(voice(hub))
}

/// The audio of a spoken card; not found until it was generated.
pub(crate) fn speech(assistant: &dyn Assistant, path: &str) -> Reply {
    let found = speech_id(path).and_then(|id| assistant.speech(id));
    match found {
        Some(speech) => Reply::bytes(speech.content_type, speech.audio),
        None => Reply::error(404, "this spoken card is not ready"),
    }
}
