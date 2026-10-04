// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A simulated AI for the development server, so the editor's AI features can be developed,
//! tested, and shown without OBS, a camera, or a paid API. Settings live in memory, plans come
//! from keywords, scene understanding replays a scripted camera, voice hears a scripted line and
//! speaks cards with a chime, voice requests get simulated answers, and subtitles show a scripted
//! speaker with canned translations. Nothing leaves the computer, and every status message and
//! plan says that it is simulated.

mod commands;
mod features;
mod hearing;
mod packs;
mod plans;
mod recipes;
mod restyle;
mod scenes;
mod settings;
mod subtitles;
mod voice;
mod watcher;

use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use sceneuron_core::Project;
use sceneuron_server::{Assistant, AssistantError, Hub, PlanRequest, ProposedPlan, Speech};
use serde_json::Value;

use self::hearing::DemoVoiceState;
use self::subtitles::DemoSubtitles;
use self::watcher::Shared;

/// How long a simulated plan takes, so the editor's progress and cancel controls can be seen.
const PLAN_DELAY: Duration = Duration::from_millis(1500);

/// The [`Assistant`] the development server uses with `--demo-ai`.
pub(crate) struct DemoAssistant {
    shared: Arc<Shared>,
    voice: Arc<DemoVoiceState>,
    subtitles: Arc<DemoSubtitles>,
    hub: Arc<Hub>,
    /// The shared room, so voice can reveal its votes and start its next round.
    room: std::sync::Mutex<Option<std::sync::Weak<sceneuron_room::RoomClient>>>,
}

impl Assistant for DemoAssistant {
    fn settings(&self) -> Value {
        let mut json = self.shared.settings().to_json();
        self.voice.settings().add_to(&mut json);
        self.subtitles.add_to(&mut json);
        json
    }

    /// Applies every field of `update`, or none of them when any field is refused.
    fn update_settings(&self, update: Value) -> Result<Value, AssistantError> {
        let mut rest = update;
        let fields = rest
            .as_object_mut()
            .ok_or_else(|| AssistantError::new(400, "the settings update must be a JSON object"))?;
        let voice = self.voice.settings().updated(fields)?;
        let subtitles = self.subtitles.updated(&Value::Object(fields.clone()))?;
        for field in ["voice", "elevenlabs_api_key", "subtitles"] {
            fields.remove(field);
        }
        let mut json = {
            let mut settings = self.shared.settings();
            settings.update(&rest)?;
            settings.to_json()
        };
        let (status, has_key) = (voice.status(), voice.key_hint.is_some());
        voice.add_to(&mut json);
        *self.voice.settings() = voice;
        self.subtitles.set(subtitles);
        self.subtitles.add_to(&mut json);
        let subtitle_status = self.subtitles.status(has_key);
        self.hub.update(|engine, now| engine.set_voice_status(status, now));
        self.hub
            .update(|engine, now| engine.set_subtitle_status(subtitle_status, now));
        self.shared.settings_changed();
        Ok(json)
    }

    fn plan(&self, project: &Project, request: &PlanRequest) -> Result<ProposedPlan, AssistantError> {
        let model = {
            let settings = self.shared.settings();
            if request.use_camera && !settings.consent {
                let message = "confirm consent in the AI tab before the assistant may see the camera";
                return Err(AssistantError::new(400, message));
            }
            format!("{} (simulated)", settings.planner_model)
        };
        thread::sleep(PLAN_DELAY);
        let plan = plans::propose(project, request);
        let project = plan
            .apply(project)
            .map_err(|error| AssistantError::new(502, format!("the simulated plan is invalid: {error}")))?;
        Ok(ProposedPlan {
            plan,
            project,
            model,
            cost_usd: 0.0,
        })
    }

    fn analyze_now(&self) -> Result<(), AssistantError> {
        if !self.shared.settings().enabled {
            return Err(AssistantError::new(400, "turn on watching the camera with AI first"));
        }
        self.shared.look_now();
        Ok(())
    }

    fn speech(&self, id: &str) -> Option<Speech> {
        let audio = self.voice.audio(id)?;
        Some(Speech {
            audio,
            content_type: "audio/wav",
        })
    }
}

/// The running simulated scene understanding and subtitle speaker.
pub(crate) struct DemoAi {
    assistant: Arc<DemoAssistant>,
    shared: Arc<Shared>,
    thread: JoinHandle<()>,
    subtitles: Arc<DemoSubtitles>,
    speaker: JoinHandle<()>,
}

/// Starts the simulated vision loop and returns it with the assistant to give the server.
pub(crate) fn start(hub: Arc<Hub>) -> std::io::Result<(DemoAi, Arc<DemoAssistant>)> {
    let shared = Arc::new(Shared::default());
    let voice = Arc::new(DemoVoiceState::default());
    hearing::follow(&hub, &voice);
    let subtitles = Arc::new(DemoSubtitles::default());
    let speaker = subtitles::start(&hub, &subtitles)?;
    let (looping, watched) = (Arc::clone(&shared), Arc::clone(&hub));
    let thread = thread::Builder::new()
        .name("sceneuron-demo-ai".to_owned())
        .spawn(move || watcher::run(&looping, &watched))?;
    let assistant = Arc::new(DemoAssistant {
        shared: Arc::clone(&shared),
        voice,
        subtitles: Arc::clone(&subtitles),
        hub: Arc::clone(&hub),
        room: std::sync::Mutex::new(None),
    });
    commands::follow(&hub, &assistant);
    let demo = DemoAi {
        assistant: Arc::clone(&assistant),
        shared,
        thread,
        subtitles,
        speaker,
    };
    Ok((demo, assistant))
}

impl DemoAi {
    /// Lets voice reveal the votes and start the next round in `room`.
    pub(crate) fn follow_room(&self, room: &Arc<sceneuron_room::RoomClient>) {
        *self
            .assistant
            .room
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::downgrade(room));
    }

    pub(crate) fn stop(self) {
        self.shared.stop();
        subtitles::stop(&self.subtitles);
        let _ = self.thread.join();
        let _ = self.speaker.join();
    }
}
