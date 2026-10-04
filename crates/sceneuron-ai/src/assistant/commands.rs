// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Voice requests: a look and subtitle changes carried out here, and words no local command
//! understood sent to the planner model, whose actions come back for the engine, with the look and
//! subtitle changes among them done here. A request needs a provider the settings can reach and an
//! hourly budget not yet spent, like the assistant's plans; a look needs scene understanding, and
//! subtitles their own consent.

use sceneuron_core::{
    MAX_TARGETS, Project, VoiceAction, VoiceReply, VoiceStep, VoiceTask, canonical_language_tag, language_label,
    same_language,
};
use serde_json::{Value, json};

use super::{Assistant, lock};
use crate::commands::{CommandAnswer, check_edits, command_request, parse_commands};
use crate::provider::AiError;
use crate::settings_update::AiSettingsUpdate;

const BUDGET_SPENT: &str = "the hourly budget is spent, so the AI takes no voice commands until it allows more";

impl Assistant {
    /// Carries out a voice request for `project`, waiting for the model when it asks the AI; the
    /// error says why nothing was done, in words for the streamer.
    pub fn carry_out(&self, project: &Project, task: &VoiceTask) -> Result<VoiceReply, String> {
        match task {
            VoiceTask::Act { action } => Ok(VoiceReply {
                understood: String::new(),
                steps: vec![VoiceStep::Done(self.host_action(project, action))],
            }),
            VoiceTask::Ask { text } => {
                let answer = self.ask_for_commands(project, text)?;
                let steps = answer
                    .actions
                    .into_iter()
                    .map(|action| {
                        if action.for_host() {
                            VoiceStep::Done(self.host_action(project, &action))
                        } else {
                            VoiceStep::Act(action)
                        }
                    })
                    .collect();
                Ok(VoiceReply {
                    understood: answer.understood,
                    steps,
                })
            }
        }
    }

    /// Queues `job` on the voice commands thread, which [`Assistant::shutdown`] stops.
    #[cfg(feature = "hub")]
    pub(crate) fn queue_voice(&self, job: Box<dyn FnOnce() + Send>) {
        self.commands.post(job);
    }

    /// One call for the words, and one more with the reason when its project edits do not apply.
    fn ask_for_commands(&self, project: &Project, said: &str) -> Result<CommandAnswer, String> {
        if self.director.over_budget() {
            return Err(BUDGET_SPENT.to_owned());
        }
        let settings = lock(&self.settings).with_credentials(&self.credentials);
        let provider = self.planner(&settings).map_err(|error| refusal(&error))?;
        let mut spent = 0.0;
        let first = command_request(project, said, None, &settings);
        let (answer, _) = self
            .ask(provider.as_ref(), &first, &settings, &mut spent)
            .map_err(|error| refusal(&error))?;
        let error = match checked(&answer, project) {
            Ok(commands) => return Ok(commands),
            Err(error) => error,
        };
        let retry = command_request(project, said, Some((&answer, &error)), &settings);
        let (answer, _) = self
            .ask(provider.as_ref(), &retry, &settings, &mut spent)
            .map_err(|error| refusal(&error))?;
        checked(&answer, project).map_err(|error| format!("the AI's answer could not be used: {error}"))
    }

    /// Does an action only the host can do; says what it did, or why not.
    fn host_action(&self, project: &Project, action: &VoiceAction) -> String {
        match action {
            VoiceAction::Look => self.look_for_voice(project),
            VoiceAction::Room { step } => match lock(&self.room).as_ref() {
                Some(handler) => handler(*step),
                None => "this Sceneuron has no shared room".to_owned(),
            },
            VoiceAction::Subtitles { enabled } => self.subtitles_for_voice(*enabled),
            VoiceAction::SubtitleLanguage { language } => self.translate_for_voice(language),
            _ => "only the engine can do that".to_owned(),
        }
    }

    fn look_for_voice(&self, project: &Project) -> String {
        let settings = lock(&self.settings).with_credentials(&self.credentials);
        if !(settings.enabled && settings.consent) {
            return "the vision model cannot look: turn on scene understanding in the AI tab".to_owned();
        }
        if !project.ai.enabled {
            return "the vision model cannot look: scene understanding is off for this project".to_owned();
        }
        // Building a provider makes no call, and refuses settings that cannot work, such as no key.
        if let Err(AiError::Config(message)) = (self.connector)(&settings) {
            return format!("the vision model cannot look: {message}");
        }
        self.director.analyze_now();
        "asked the vision model to look at the camera".to_owned()
    }

    fn subtitles_for_voice(&self, enabled: bool) -> String {
        let state = if enabled { "on" } else { "off" };
        if lock(&self.settings).subtitles.enabled == enabled {
            return format!("subtitles were already {state}");
        }
        match self.change_subtitles(json!({ "enabled": enabled })) {
            Ok(()) => format!("turned subtitles {state}{}", self.missing_key(enabled)),
            Err(why) => format!("subtitles stay {}: {why}", if enabled { "off" } else { "on" }),
        }
    }

    /// Puts `language` first among the languages subtitles translate into, keeping the others
    /// that fit, and turns subtitles on when they were agreed to.
    fn translate_for_voice(&self, language: &str) -> String {
        let Some(code) = canonical_language_tag(language) else {
            return format!("{language:?} is not a language code");
        };
        let label = language_label(&code);
        let subtitles = lock(&self.settings).subtitles.clone();
        if same_language(&subtitles.language, &code) {
            return format!("subtitles are already in {label}, the spoken language");
        }
        let mut targets = vec![code.clone()];
        targets.extend(
            subtitles
                .targets
                .iter()
                .filter(|known| !same_language(known, &code))
                .cloned(),
        );
        targets.truncate(MAX_TARGETS);
        let turn_on = subtitles.consent && !subtitles.enabled;
        let change = if turn_on {
            json!({ "targets": targets, "enabled": true })
        } else {
            json!({ "targets": targets })
        };
        if let Err(why) = self.change_subtitles(change) {
            return format!("subtitles were not changed: {why}");
        }
        match (subtitles.enabled || turn_on, subtitles.consent) {
            (true, _) => format!("subtitles translate into {label} first{}", self.missing_key(true)),
            (false, false) => format!(
                "subtitles will translate into {label} first; to show them, confirm in the AI tab that microphone \
                 audio may be sent to ElevenLabs for subtitles"
            ),
            (false, true) => format!("subtitles will translate into {label} first"),
        }
    }

    fn change_subtitles(&self, change: Value) -> Result<(), String> {
        let update: AiSettingsUpdate =
            serde_json::from_value(json!({ "subtitles": change })).map_err(|error| error.to_string())?;
        self.update_settings(update)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// What subtitles still need to show, when they are on without an ElevenLabs key.
    fn missing_key(&self, enabled: bool) -> &'static str {
        let keyless = lock(&self.settings)
            .with_credentials(&self.credentials)
            .elevenlabs_api_key
            .is_empty();
        if enabled && keyless {
            "; they show once an ElevenLabs key is added in the AI tab"
        } else {
            ""
        }
    }
}

/// Reads an answer and checks that its edits apply; the error is the reason to send back.
fn checked(answer: &Value, project: &Project) -> Result<CommandAnswer, String> {
    let commands = parse_commands(answer).map_err(|error| error.message().to_owned())?;
    check_edits(&commands, project)?;
    Ok(commands)
}

/// Why a call could not be made or answered, for the streamer; settings that cannot work yet say
/// how to set the AI up.
fn refusal(error: &AiError) -> String {
    match error {
        AiError::Config(message) => {
            format!("no command matched, and the AI that takes any other is not set up: {message}")
        }
        other => other.to_string(),
    }
}
