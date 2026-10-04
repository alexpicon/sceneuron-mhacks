// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The host-facing entry point: AI settings kept in `ai.json`, the director, the planner that
//! turns the streamer's words into a validated edit plan, the speaker that voices cards, the
//! voice input that hears commands and subtitles, the voice requests that the AI and the settings
//! answer, and the translator of subtitles.

mod commands;
mod plans;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use sceneuron_core::{EditPlan, Project, RoomStep, TranslationJob};

use crate::commands::Runner;
use crate::credentials::Credentials;
use crate::director::{Director, Sink};
use crate::elevenlabs::ElevenLabs;
use crate::provider::{AiError, Connector, Provider};
use crate::settings::AiSettings;
use crate::settings_file::{load_settings, save_settings};
use crate::settings_update::AiSettingsUpdate;
use crate::settings_view::AiSettingsView;
use crate::speaker::Speaker;
use crate::speech_cache::SpeechCache;
use crate::translator::Translator;
use crate::voice_link::VoiceLink;

/// A plan that applies cleanly, with the project it produces.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanOutcome {
    pub plan: EditPlan,
    /// The current project with the plan applied and validated; not saved.
    pub project: Project,
    /// The model that wrote the plan.
    pub model: String,
    /// Estimated cost of every call the plan took, in US dollars.
    pub cost_usd: f64,
}

/// Owns the AI settings, the director thread, the planner, the speaker thread, the voice input,
/// the voice commands thread, and the translator thread.
pub struct Assistant {
    path: PathBuf,
    connector: Connector,
    /// Keys from the environment, used wherever the editor set none.
    credentials: Credentials,
    settings: Mutex<AiSettings>,
    /// The planner provider and the settings it was built from, reused while they match so
    /// what it learned about the endpoint is kept.
    planner: Mutex<Option<(AiSettings, Arc<dyn Provider>)>>,
    director: Director,
    speaker: Speaker,
    speech: SpeechCache,
    voice: Arc<VoiceLink>,
    /// Carries out voice requests one at a time, off the threads that hear words.
    commands: Runner,
    translator: Translator,
    /// Carries out a room step for voice and says what happened; set by the host that holds the
    /// shared room.
    room: Mutex<Option<RoomHandler>>,
}

/// What reveals the shared room's votes or starts its next round, answering in a few words.
pub type RoomHandler = Box<dyn Fn(RoomStep) -> String + Send + Sync>;

impl Assistant {
    /// Loads the settings from `path` and starts the director and the speaker. `credentials`
    /// supply keys the editor did not set; they are used but never saved. Spoken cards are kept
    /// in a `speech` folder next to `path`. The warning says why saved settings could not be
    /// used, if they could not.
    pub fn new(
        path: PathBuf,
        connector: Connector,
        sink: Arc<dyn Sink>,
        credentials: Credentials,
    ) -> (Assistant, Option<String>) {
        Self::with_elevenlabs(path, connector, sink, credentials, &ElevenLabs::default())
    }

    /// [`Assistant::new`] with ElevenLabs reached at `elevenlabs_url`, such as a test server;
    /// see [`ElevenLabs::at`].
    pub fn with_elevenlabs_url(
        path: PathBuf,
        connector: Connector,
        sink: Arc<dyn Sink>,
        credentials: Credentials,
        elevenlabs_url: &str,
    ) -> (Assistant, Option<String>) {
        Self::with_elevenlabs(path, connector, sink, credentials, &ElevenLabs::at(elevenlabs_url))
    }

    /// [`Assistant::new`] with ElevenLabs reached as `elevenlabs` says.
    pub fn with_elevenlabs(
        path: PathBuf,
        connector: Connector,
        sink: Arc<dyn Sink>,
        credentials: Credentials,
        elevenlabs: &ElevenLabs,
    ) -> (Assistant, Option<String>) {
        let (settings, warning) = load_settings(&path);
        let effective = settings.with_credentials(&credentials);
        let director = Director::spawn(effective.clone(), Arc::clone(&connector), Arc::clone(&sink));
        let speech = SpeechCache::new(path.parent().unwrap_or(&path).join("speech"));
        let speaker = Speaker::spawn(
            effective.clone(),
            speech.clone(),
            &elevenlabs.api_url,
            director.ledger(),
            Arc::clone(&sink),
        );
        let translator = Translator::spawn(
            effective.clone(),
            Arc::clone(&connector),
            director.ledger(),
            Arc::clone(&sink),
        );
        let voice = VoiceLink::start(&effective, elevenlabs, sink, director.ledger());
        let assistant = Assistant {
            path,
            connector,
            credentials,
            settings: Mutex::new(settings),
            planner: Mutex::new(None),
            director,
            speaker,
            speech,
            voice: Arc::new(voice),
            commands: Runner::spawn(),
            translator,
            room: Mutex::new(None),
        };
        (assistant, warning)
    }

    /// The settings without the API keys.
    pub fn settings_view(&self) -> AiSettingsView {
        lock(&self.settings).view_with(&self.credentials)
    }

    /// Validates and saves a change, then hands the new settings to the director.
    pub fn update_settings(&self, update: AiSettingsUpdate) -> Result<AiSettingsView, AiError> {
        let mut settings = lock(&self.settings);
        let next = settings.updated(&update)?;
        save_settings(&self.path, &next)
            .map_err(|error| AiError::Config(format!("cannot save the AI settings: {error}")))?;
        *settings = next.clone();
        // Still under the lock, so the director, the speaker, the translator, and the voice input
        // receive concurrent updates in saved order.
        let effective = next.with_credentials(&self.credentials);
        self.director.set_settings(effective.clone());
        self.speaker.set_settings(effective.clone());
        self.translator.set_settings(effective.clone());
        self.voice.set_settings(&effective);
        Ok(next.view_with(&self.credentials))
    }

    /// Speaks the speech cue `nonce`; the sink hears when its audio is ready or why it is not.
    pub fn speak(&self, nonce: u64, text: &str) {
        self.speaker.speak(nonce, text);
    }

    /// The generated audio of a spoken card, by the id the sink reported.
    pub fn speech(&self, id: &str) -> Option<Vec<u8>> {
        self.speech.get(id)
    }

    /// Translates the subtitle segment of `job`; the sink hears the answer or why there is none.
    pub fn translate(&self, job: TranslationJob) {
        self.translator.translate(job);
    }

    /// Voice control: push to talk, the microphone's input, and keyterms from the project.
    pub fn voice(&self) -> &Arc<VoiceLink> {
        &self.voice
    }

    /// Lets voice reveal the shared room's votes and start its next round through `handler`.
    pub fn set_room(&self, handler: RoomHandler) {
        *lock(&self.room) = Some(handler);
    }

    /// The director, for frames, project changes, and local hints.
    pub fn director(&self) -> &Director {
        &self.director
    }

    /// Stops the voice input, the voice commands, the speaker, the translator, and the director;
    /// see [`Director::shutdown`]. A voice command in flight finishes first, within its model
    /// call's timeout.
    pub fn shutdown(&self) {
        self.voice.shutdown();
        self.commands.shutdown();
        self.speaker.shutdown();
        self.translator.shutdown();
        self.director.shutdown();
    }
}

pub(super) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
