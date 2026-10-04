// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Hosting the AI in a Sceneuron hub: model results, heard words, and subtitle translations flow
//! into the engine, project and presence changes flow to the director, speech cues flow to the
//! speaker, subtitle segments flow to the translator, push to talk and keyterms flow to the voice
//! input, and the server's AI endpoints are answered by the assistant.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use sceneuron_core::{AiStatus, Heard, Project, SceneObservation, SubtitleStatus, Verdict, VoiceStatus};
use sceneuron_server::{AssistantError, Hub, HubEvent, PlanRequest, ProposedPlan, Speech};
use serde_json::{Value, json};

use crate::assistant::Assistant;
use crate::director::{Director, Sink};
use crate::hub_commands::follow_requests;
use crate::hub_subtitles::follow_subtitles;
use crate::planner::PlanMode;
use crate::provider::AiError;
use crate::settings_update::AiSettingsUpdate;
use crate::tts::SPEECH_CONTENT_TYPE;
use crate::voice_link::VoiceLink;

/// Applies what the director reports to the hub's engine.
pub struct HubSink {
    hub: Arc<Hub>,
}

impl HubSink {
    pub fn new(hub: Arc<Hub>) -> HubSink {
        HubSink { hub }
    }
}

impl Sink for HubSink {
    fn observed(&self, observation: SceneObservation, captured_ms: u64) {
        self.hub
            .update(|engine, _| engine.observe_scene(observation, captured_ms));
    }

    fn status(&self, status: AiStatus) {
        self.hub.update(|engine, now| engine.set_ai_status(status, now));
    }

    fn verdicts(&self, verdicts: Vec<Verdict>, observed_ms: u64) {
        self.hub
            .update(|engine, _| engine.observe_verdicts(verdicts, observed_ms));
    }

    fn reset(&self, why: &str) {
        self.hub.update(|engine, now| engine.reset_conditions(why, now));
    }

    fn voice(&self, status: VoiceStatus) {
        self.hub.update(|engine, now| engine.set_voice_status(status, now));
    }

    fn heard(&self, heard: &Heard) {
        self.hub.update(|engine, now| engine.hear(heard, now));
    }

    fn spoken(&self, nonce: u64, audio: &str) {
        self.hub.update(|engine, now| engine.speech_ready(nonce, audio, now));
    }

    fn speech_failed(&self, why: &str) {
        self.hub.update(|engine, now| engine.note("voice", why, now));
    }

    fn voice_note(&self, note: &str) {
        self.hub.update(|engine, now| engine.note("voice", note, now));
    }

    fn subtitles(&self, status: SubtitleStatus) {
        self.hub.update(|engine, now| engine.set_subtitle_status(status, now));
    }

    fn translated(&self, id: u64, translations: &BTreeMap<String, String>) {
        self.hub
            .update(|engine, now| engine.subtitles_translated(id, translations, now));
    }

    fn translation_failed(&self, id: u64, why: &str) {
        self.hub
            .update(|engine, now| engine.subtitles_translation_failed(id, why, now));
    }
}

/// Keeps the director in step with the hub: the project's conditions and AI policy now and after
/// every project change, and presence changes as they happen. Each new speech cue goes to the
/// speaker once, each new subtitle segment to the translator once, each new voice request to the
/// assistant once, and the voice input follows push to talk and the project (see
/// [`follow_voice`]).
pub fn follow_hub(hub: &Arc<Hub>, assistant: &Arc<Assistant>) {
    follow_voice(hub, assistant.voice());
    follow_subtitles(hub, assistant);
    follow_requests(hub, assistant);
    sync_project(hub, assistant.director());
    let (hub_ref, assistant_ref) = (Arc::downgrade(hub), Arc::downgrade(assistant));
    let spoken = AtomicU64::new(0);
    hub.add_listener(Box::new(move |event| {
        let Some(assistant) = assistant_ref.upgrade() else {
            return;
        };
        match event {
            HubEvent::ProjectChanged => {
                if let Some(hub) = hub_ref.upgrade() {
                    sync_project(&hub, assistant.director());
                }
            }
            HubEvent::PresenceChanged(_) => assistant.director().presence_changed(),
            HubEvent::StateChanged { .. } => {
                let Some(hub) = hub_ref.upgrade() else {
                    return;
                };
                let cue = hub.read(|engine| {
                    let cue = engine.speech().filter(|cue| cue.audio.is_none())?;
                    Some((cue.nonce, cue.text.clone()))
                });
                if let Some((nonce, text)) = cue
                    && spoken.fetch_max(nonce, Ordering::SeqCst) < nonce
                {
                    assistant.speak(nonce, &text);
                }
            }
            HubEvent::SaveFailed(_) => {}
        }
    }));
}

/// Keeps a voice input in step with the hub: it holds push to talk while the engine does, so
/// the editor, a control surface, or a hotkey can hold it, and lets go when the engine lets go
/// by itself; and it takes its keyterms from the project now and after every change.
pub fn follow_voice(hub: &Arc<Hub>, voice: &Arc<VoiceLink>) {
    // The project is copied out first: the voice input reports to the hub, which must not be
    // called while it is read.
    voice.set_project(&hub.read(|engine| engine.project().clone()));
    let talking = AtomicBool::new(false);
    let (hub_ref, voice_ref) = (Arc::downgrade(hub), Arc::downgrade(voice));
    hub.add_listener(Box::new(move |event| {
        let (Some(hub), Some(voice)) = (hub_ref.upgrade(), voice_ref.upgrade()) else {
            return;
        };
        match event {
            HubEvent::StateChanged { .. } => {
                let now = hub.read(|engine| engine.talking());
                if talking.swap(now, Ordering::SeqCst) != now {
                    voice.talk(now);
                }
            }
            HubEvent::ProjectChanged => voice.set_project(&hub.read(|engine| engine.project().clone())),
            HubEvent::PresenceChanged(_) | HubEvent::SaveFailed(_) => {}
        }
    }));
}

/// Gives the director the project's conditions and the conditions and cues of the built-in
/// reactions that watch the camera now.
fn sync_project(hub: &Hub, director: &Director) {
    let (conditions, cues, policy, wants_caption) = hub.read(|engine| {
        let project = engine.project();
        let caption = project
            .elements
            .iter()
            .any(|element| sceneuron_core::shows_caption(&element.props.text));
        let reactions = engine.active_reactions();
        let mut conditions = project.conditions.clone();
        conditions.extend(reactions.iter().map(|reaction| reaction.condition.clone()));
        let cues = reactions.iter().map(|reaction| reaction.cue()).collect();
        (conditions, cues, project.ai.clone(), caption)
    });
    director.set_project_with_cues(conditions, cues, policy, wants_caption);
}

/// The server's AI endpoints, answered by an [`Assistant`].
pub struct AiService {
    assistant: Arc<Assistant>,
}

impl AiService {
    pub fn new(assistant: Arc<Assistant>) -> AiService {
        AiService { assistant }
    }
}

impl sceneuron_server::Assistant for AiService {
    fn settings(&self) -> Value {
        serde_json::to_value(self.assistant.settings_view()).unwrap_or_else(|_| json!({ "available": false }))
    }

    fn update_settings(&self, update: Value) -> Result<Value, AssistantError> {
        let update: AiSettingsUpdate = serde_json::from_value(update)
            .map_err(|error| AssistantError::new(400, format!("invalid AI settings: {error}")))?;
        let view = self.assistant.update_settings(update).map_err(refusal)?;
        serde_json::to_value(view).map_err(|error| AssistantError::new(500, error.to_string()))
    }

    fn plan(&self, project: &Project, request: &PlanRequest) -> Result<ProposedPlan, AssistantError> {
        let mode = match request.mode {
            sceneuron_server::PlanMode::Edit => PlanMode::Edit,
            sceneuron_server::PlanMode::Setup => PlanMode::Setup,
        };
        let outcome = self
            .assistant
            .plan(project, &request.request, mode, request.use_camera)
            .map_err(refusal)?;
        Ok(ProposedPlan {
            plan: outcome.plan,
            project: outcome.project,
            model: outcome.model,
            cost_usd: outcome.cost_usd,
        })
    }

    fn analyze_now(&self) -> Result<(), AssistantError> {
        let view = self.assistant.settings_view();
        if !(view.enabled && view.consent) {
            return Err(AssistantError::new(400, "turn on scene understanding first"));
        }
        self.assistant.director().analyze_now();
        Ok(())
    }

    fn speech(&self, id: &str) -> Option<Speech> {
        let audio = self.assistant.speech(id)?;
        Some(Speech {
            audio,
            content_type: SPEECH_CONTENT_TYPE,
        })
    }
}

/// Problems with the streamer's input are 400; problems at the provider are gateway errors, so
/// the editor never mistakes them for its own access token being refused.
fn refusal(error: AiError) -> AssistantError {
    let status = match error {
        AiError::Config(_) => 400,
        AiError::RateLimited(_) => 429,
        AiError::Unavailable(_) => 503,
        AiError::Auth(_) | AiError::BadRequest(_) | AiError::Refused(_) | AiError::Invalid(_) => 502,
    };
    AssistantError::new(status, error.to_string())
}
