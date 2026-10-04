// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Simulated voice requests in the development server: the simulated AI carries out a look and
//! subtitle changes at once, and answers words no local command knows with a simulated plan's
//! project edit, so the whole voice path can be shown and tested without a model. Nothing leaves
//! the computer.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use sceneuron_core::{
    MAX_TARGETS, Project, VoiceAction, VoiceReply, VoiceRequest, VoiceStep, VoiceTask, canonical_language_tag,
    language_label, same_language,
};
use sceneuron_server::{Assistant, Hub, HubEvent, PlanMode, PlanRequest};
use serde_json::json;

use super::{DemoAssistant, plans};

/// Answers each new voice request of the hub's engine once, as the simulated AI.
pub(crate) fn follow(hub: &Arc<Hub>, assistant: &Arc<DemoAssistant>) {
    hub.update(|engine, _| engine.set_voice_host(true));
    let (hub_ref, assistant_ref) = (Arc::downgrade(hub), Arc::downgrade(assistant));
    let answered = AtomicU64::new(0);
    hub.add_listener(Box::new(move |event| {
        if !matches!(event, HubEvent::StateChanged { .. }) {
            return;
        }
        let (Some(hub), Some(assistant)) = (hub_ref.upgrade(), assistant_ref.upgrade()) else {
            return;
        };
        let after = answered.load(Ordering::SeqCst);
        let (requests, project) = hub.read(|engine| {
            let requests = engine.voice_requests().iter();
            let new: Vec<VoiceRequest> = requests.filter(|request| request.nonce > after).cloned().collect();
            (new, engine.project().clone())
        });
        for request in requests {
            answered.fetch_max(request.nonce, Ordering::SeqCst);
            let reply = assistant.carry_out(&project, &request.task);
            hub.update(|engine, now| engine.voice_replied(request.nonce, reply, now));
        }
    }));
}

impl DemoAssistant {
    fn carry_out(&self, project: &Project, task: &VoiceTask) -> Result<VoiceReply, String> {
        let (understood, steps) = match task {
            VoiceTask::Act { action } => (String::new(), vec![VoiceStep::Done(self.host_action(action))]),
            VoiceTask::Ask { text } => {
                let request = PlanRequest {
                    request: text.clone(),
                    mode: PlanMode::Edit,
                    use_camera: false,
                };
                let plan = plans::propose(project, &request);
                let steps = if plan.operations.is_empty() {
                    Vec::new()
                } else {
                    let edit = VoiceAction::EditProject {
                        summary: plan.summary.clone(),
                        operations: plan.operations,
                    };
                    vec![VoiceStep::Act(edit)]
                };
                ("a simulated plan".to_owned(), steps)
            }
        };
        Ok(VoiceReply { understood, steps })
    }

    fn host_action(&self, action: &VoiceAction) -> String {
        match action {
            VoiceAction::Look if self.shared.settings().enabled => {
                self.shared.look_now();
                "the simulated vision model looks at the camera now".to_owned()
            }
            VoiceAction::Look => "the vision model cannot look: turn on watching the camera with AI first".to_owned(),
            VoiceAction::Subtitles { enabled } => {
                let state = if *enabled { "on" } else { "off" };
                match self.update_settings(json!({ "subtitles": { "enabled": enabled } })) {
                    Ok(_) => format!("turned subtitles {state}"),
                    Err(error) => format!("subtitles were not turned {state}: {}", error.message),
                }
            }
            VoiceAction::SubtitleLanguage { language } => self.translate(language),
            VoiceAction::Room { step } => {
                let room = self
                    .room
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone();
                room.and_then(|room| room.upgrade()).map_or_else(
                    || "this Sceneuron has no shared room".to_owned(),
                    |room| room.voice_step(*step),
                )
            }
            _ => "only the engine can do that".to_owned(),
        }
    }

    /// Puts `language` first among the simulated subtitles' targets.
    fn translate(&self, language: &str) -> String {
        let Some(code) = canonical_language_tag(language) else {
            return format!("{language:?} is not a language code");
        };
        let current = self.subtitles.settings();
        let mut targets = vec![code.clone()];
        targets.extend(
            current
                .targets
                .iter()
                .filter(|known| !same_language(known, &code))
                .cloned(),
        );
        targets.truncate(MAX_TARGETS);
        match self.update_settings(json!({ "subtitles": { "targets": targets } })) {
            Ok(_) => format!("subtitles translate into {} first", language_label(&code)),
            Err(error) => format!("subtitles were not changed: {}", error.message),
        }
    }
}

#[cfg(test)]
mod tests {
    use sceneuron_core::default_project;
    use sceneuron_server::HubConfig;

    use super::*;

    fn said(hub: &Hub, words: &str) -> String {
        hub.update(|engine, now| engine.type_command(words, now));
        hub.read(|engine| engine.voice_view().heard.unwrap().result)
    }

    #[test]
    fn the_simulated_ai_answers_every_request_at_once() {
        let hub = Hub::start(default_project(), HubConfig::default()).unwrap();
        let (demo, assistant) = super::super::start(Arc::clone(&hub)).unwrap();
        let answer = said(&hub, "make the name label blue");
        assert!(
            answer.starts_with("the AI took it as \"a simulated plan\": changed the project: Simulated plan:"),
            "{answer}"
        );
        let label = hub.read(|engine| engine.project().element("name-label").unwrap().props.background.clone());
        assert_eq!(label, "#3b82f6ee");
        assert_eq!(
            said(&hub, "look now"),
            "the vision model cannot look: turn on watching the camera with AI first"
        );
        assert_eq!(
            said(&hub, "subtitles in Spanish"),
            "subtitles translate into Spanish (es) first"
        );
        assert_eq!(assistant.subtitles.settings().targets, ["es"]);
        assert!(hub.read(|engine| engine.voice_requests().is_empty()));
        demo.stop();
        hub.shutdown();
    }
}
