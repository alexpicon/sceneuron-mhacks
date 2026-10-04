// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Voice request fixtures: an engine whose host answers voice requests, words said with push to
//! talk held, the heard line's result, the newest request, and a reply from the AI.

use sceneuron_core::{Engine, Heard, VoiceMode, VoiceReply, VoiceStep, VoiceTask};

use super::voice::meeting_engine;

/// The meeting engine with push to talk, and a host that answers voice requests.
pub fn hosted() -> Engine {
    let mut engine = meeting_engine(VoiceMode::PushToTalk);
    engine.set_voice_host(true);
    engine
}

/// Hears final words said with push to talk held.
pub fn held(engine: &mut Engine, text: &str, at_ms: u64) {
    let heard = Heard {
        text: text.to_owned(),
        is_final: true,
        addressed: true,
    };
    engine.hear(&heard, at_ms);
}

pub fn heard_result(engine: &Engine) -> String {
    engine.voice_view().heard.unwrap().result
}

/// The nonce and task of the newest waiting request.
pub fn newest(engine: &Engine) -> (u64, VoiceTask) {
    let request = engine.voice_requests().last().expect("a request").clone();
    (request.nonce, request.task)
}

/// The AI's reply: it took the words as "make it festive", and these steps.
pub fn reply(steps: Vec<VoiceStep>) -> Result<VoiceReply, String> {
    Ok(VoiceReply {
        understood: "make it festive".to_owned(),
        steps,
    })
}
