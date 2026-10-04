// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Where ElevenLabs is reached: its HTTPS API for text to speech and the voice list, and its
//! realtime speech-to-text WebSocket.

use sceneuron_voice::{DEFAULT_ENDPOINT, Transport};

use crate::tts::DEFAULT_ELEVENLABS_URL;

/// Where ElevenLabs is reached and how its TLS is trusted.
#[derive(Clone)]
pub struct ElevenLabs {
    /// The HTTPS API, for text to speech and the voice list.
    pub api_url: String,
    /// The realtime speech-to-text WebSocket.
    pub realtime_url: String,
    pub transport: Transport,
}

impl Default for ElevenLabs {
    fn default() -> Self {
        Self {
            api_url: DEFAULT_ELEVENLABS_URL.to_owned(),
            realtime_url: DEFAULT_ENDPOINT.to_owned(),
            transport: Transport::default(),
        }
    }
}

impl ElevenLabs {
    /// The API at `api_url`, with its realtime WebSocket at the same host: `https://` becomes
    /// `wss://`, and `http://` `ws://`, which the voice input accepts only on this computer.
    pub fn at(api_url: &str) -> ElevenLabs {
        let base = api_url.trim_end_matches('/');
        let socket = match base.split_once("://") {
            Some(("http", rest)) => format!("ws://{rest}"),
            Some((_, rest)) => format!("wss://{rest}"),
            None => format!("wss://{base}"),
        };
        ElevenLabs {
            api_url: base.to_owned(),
            realtime_url: format!("{socket}{REALTIME_PATH}"),
            transport: Transport::default(),
        }
    }
}

/// The realtime speech-to-text WebSocket's path, from [`DEFAULT_ENDPOINT`].
const REALTIME_PATH: &str = "/v1/speech-to-text/realtime";
