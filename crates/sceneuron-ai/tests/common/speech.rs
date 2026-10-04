// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A test ElevenLabs key, and the MP3 audio and voice lists a scripted ElevenLabs server
//! answers with.

use serde_json::json;

use super::server::Reply;

/// A key-shaped test value; never a real credential.
pub const EL_KEY: &str = "el-test-key-0123456789";

/// A few bytes that start like an MP3 file.
pub fn mp3() -> Vec<u8> {
    let mut audio = b"ID3\x04\x00\x00\x00\x00\x00\x00".to_vec();
    audio.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00, 1, 2, 3]);
    audio
}

pub fn audio_reply() -> Reply {
    Reply::bytes(200, "audio/mpeg", mp3())
}

/// A first page of `GET /v2/voices` with `(voice_id, name, category)` voices.
pub fn voices_reply(voices: &[(&str, &str, &str)], has_more: bool) -> Reply {
    let voices: Vec<_> = voices
        .iter()
        .map(|(id, name, category)| json!({"voice_id": id, "name": name, "category": category}))
        .collect();
    Reply::json(
        200,
        json!({"voices": voices, "has_more": has_more, "total_count": voices.len()}),
    )
}

/// An account that has the default voice.
pub fn default_voices() -> Reply {
    voices_reply(&[("JBFqnCBsd6RMkjVDRZzb", "George", "premade")], false)
}
