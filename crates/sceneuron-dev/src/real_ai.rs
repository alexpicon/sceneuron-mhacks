// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The real AI in the development server: the assistant calls the configured models with keys
//! from the AI settings or the `.env` file. There is no camera here, so scene understanding
//! stays idle, but the assistant and its plans work as they do in OBS.

use std::path::Path;
use std::sync::Arc;

use sceneuron_ai::{
    AiService, Assistant, Credentials, HubSink, VoiceLink, default_connector, env_file_path, follow_hub,
};
use sceneuron_server::Hub;

use crate::voice_wav::elevenlabs;

/// The running assistant; stop it before the hub.
pub(crate) struct RealAi {
    assistant: Arc<Assistant>,
}

impl RealAi {
    /// Lets voice reveal the votes and start the next round in `room`.
    pub(crate) fn follow_room(&self, room: &Arc<sceneuron_room::RoomClient>) {
        let reached = Arc::downgrade(room);
        self.assistant.set_room(Box::new(move |step| {
            reached
                .upgrade()
                .map_or_else(|| "the room is closed".to_owned(), |room| room.voice_step(step))
        }));
    }

    pub(crate) fn stop(self) {
        self.assistant.shutdown();
    }

    /// The assistant's voice input.
    pub(crate) fn voice(&self) -> Arc<VoiceLink> {
        Arc::clone(self.assistant.voice())
    }
}

/// Starts the assistant with settings in `settings_path` and optional keys from `env_file`, with
/// speech to text at `voice_url` when given; returns it, the server's view of it, and warnings
/// to show.
pub(crate) fn start(
    hub: &Arc<Hub>,
    settings_path: &Path,
    env_file: &Path,
    voice_url: Option<&str>,
) -> (RealAi, Arc<AiService>, Vec<String>) {
    let (credentials, mut warnings) = Credentials::load(Some(&env_file_path(env_file)));
    let sink = Arc::new(HubSink::new(Arc::clone(hub)));
    let (assistant, warning) = Assistant::with_elevenlabs(
        settings_path.to_path_buf(),
        default_connector(),
        sink,
        credentials,
        &elevenlabs(voice_url),
    );
    warnings.extend(warning);
    let assistant = Arc::new(assistant);
    follow_hub(hub, &assistant);
    let service = Arc::new(AiService::new(Arc::clone(&assistant)));
    (RealAi { assistant }, service, warnings)
}
