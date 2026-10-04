// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Voice control and subtitles: the voice input, configured from the voice and subtitle settings
//! and the ElevenLabs key, with keyterms from the project, reporting what it hears and how both
//! are doing through the sink, and adding the audio it sends to the spend. Under an assistant, it
//! stops listening while the hourly budget is spent and listens again once the budget allows.
//!
//! Keyterms are the wake word and the card names and trigger phrases of decks whose voice is
//! on; they help recognition and cost extra. In push-to-talk mode, with no such decks, none are
//! sent, since push to talk needs no wake word, and none while voice control is off, as when
//! only subtitles listen.

mod state;

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use sceneuron_core::{Heard, Project};
use sceneuron_voice::{Voice, VoiceEvent, VoiceInput};

use crate::director::{Ledger, Sink};
use crate::elevenlabs::ElevenLabs;
use crate::settings::AiSettings;
use crate::voice_cost::{VoiceUsage, voice_cost_usd};
use state::LinkState;

/// The voice input and what it was configured from.
pub struct VoiceLink {
    voice: Arc<Voice>,
    state: Arc<Mutex<LinkState>>,
    sink: Arc<dyn Sink>,
}

impl VoiceLink {
    /// Starts the voice input with `settings`, keys included, reporting through `sink`; voice
    /// spend goes to `ledger`, whose hourly budget it keeps to.
    pub(crate) fn start(
        settings: &AiSettings,
        elevenlabs: &ElevenLabs,
        sink: Arc<dyn Sink>,
        ledger: Ledger,
    ) -> VoiceLink {
        let recorder = ledger.clone();
        let spent = move |usage, cost| recorder.record_voice(usage, cost);
        let link = VoiceLink::open(settings, elevenlabs, sink, ledger.over_budget(), spent);
        link.follow_budget(&ledger);
        link
    }

    /// Starts a voice input on its own, for a host without an assistant and so without an AI
    /// status or a budget: `spent` hears each report of the audio sent and its estimated cost.
    pub fn standalone(
        settings: &AiSettings,
        elevenlabs: &ElevenLabs,
        sink: Arc<dyn Sink>,
        spent: impl FnMut(VoiceUsage, f64) + Send + 'static,
    ) -> VoiceLink {
        let link = VoiceLink::open(settings, elevenlabs, sink, false, spent);
        report(&link.state, link.sink.as_ref());
        link
    }

    fn open(
        settings: &AiSettings,
        elevenlabs: &ElevenLabs,
        sink: Arc<dyn Sink>,
        budget_spent: bool,
        spent: impl FnMut(VoiceUsage, f64) + Send + 'static,
    ) -> VoiceLink {
        let state = LinkState::new(settings, elevenlabs, budget_spent);
        let config = state.config();
        let state = Arc::new(Mutex::new(state));
        let events = handler(Arc::clone(&state), Arc::clone(&sink), spent);
        let voice = Arc::new(Voice::start(config, elevenlabs.transport.clone(), events));
        VoiceLink { voice, state, sink }
    }

    /// New settings, keys included. A language detected is forgotten when subtitles turn off or
    /// the spoken language changes.
    pub fn set_settings(&self, settings: &AiSettings) {
        self.change(|state| {
            let language = state.language().to_owned();
            state.settings = settings.voice.clone();
            state.subtitles = settings.subtitles.clone();
            settings.elevenlabs_api_key.clone_into(&mut state.key);
            if !state.subtitles.enabled || state.language() != language {
                state.detected.clear();
            }
        });
    }

    /// Takes the keyterms from `project`.
    pub fn set_project(&self, project: &Project) {
        let terms = voice_terms(project);
        if lock(&self.state).terms != terms {
            self.change(|state| state.terms = terms);
        }
    }

    /// Push to talk pressed or released.
    pub fn talk(&self, down: bool) {
        self.voice.talk(down);
    }

    /// A handle for whatever feeds microphone audio.
    pub fn input(&self) -> VoiceInput {
        self.voice.input()
    }

    /// The keyterms sessions are asked to bias toward now.
    pub fn keyterms(&self) -> Vec<String> {
        lock(&self.state).config().keyterms
    }

    /// Closes any session and stops the voice thread.
    pub fn shutdown(&self) {
        self.voice.shutdown();
    }

    /// Stops listening whenever the director says the hourly budget is spent, and listens again
    /// when it says the budget allows. The budget is read once more after watching starts, in
    /// case it changed since the link started.
    fn follow_budget(&self, ledger: &Ledger) {
        let (voice, state, sink) = (
            Arc::downgrade(&self.voice),
            Arc::downgrade(&self.state),
            Arc::clone(&self.sink),
        );
        ledger.watch(move |spent| {
            if let (Some(voice), Some(state)) = (voice.upgrade(), state.upgrade()) {
                change(&voice, &state, sink.as_ref(), |state| state.budget_spent = spent);
            }
        });
        self.change(|state| state.budget_spent = ledger.over_budget());
    }

    fn change(&self, edit: impl FnOnce(&mut LinkState)) {
        change(&self.voice, &self.state, self.sink.as_ref(), edit);
    }
}

/// Applies `edit`, reconfigures the voice input when what it runs on changed, which keeps
/// capture going through changes that do not concern it, and reports the status. The voice
/// input is reconfigured under the lock, so changes from different threads reach it in the
/// order they were made.
fn change(voice: &Voice, state: &Mutex<LinkState>, sink: &dyn Sink, edit: impl FnOnce(&mut LinkState)) {
    {
        let mut state = lock(state);
        let before = state.config();
        edit(&mut state);
        state.version += 1;
        let after = state.config();
        if after != before {
            voice.configure(after);
        }
    }
    report(state, sink);
}

/// The card names, then the trigger phrases, of every deck whose voice is on.
pub fn voice_terms(project: &Project) -> Vec<String> {
    let decks = || project.decks.iter().filter(|deck| deck.voice);
    let names = decks().flat_map(|deck| deck.cards.iter().map(|card| card.name.clone()));
    let phrases = decks().flat_map(|deck| deck.cards.iter().flat_map(|card| card.phrases.clone()));
    names.chain(phrases).collect()
}

/// What the voice thread reports: transcripts to the engine, statuses and a newly detected
/// language combined with the settings, and seconds sent priced into the spend.
fn handler(
    state: Arc<Mutex<LinkState>>,
    sink: Arc<dyn Sink>,
    mut spent: impl FnMut(VoiceUsage, f64) + Send + 'static,
) -> impl FnMut(VoiceEvent) + Send + 'static {
    move |event| match event {
        VoiceEvent::Heard(heard) => sink.heard(&Heard {
            text: heard.text,
            is_final: heard.is_final,
            addressed: heard.addressed,
        }),
        VoiceEvent::Status(input) => {
            {
                let mut state = lock(&state);
                state.input = input;
                state.version += 1;
            }
            report(&state, sink.as_ref());
        }
        VoiceEvent::Language(code) => {
            let changed = {
                let mut state = lock(&state);
                let changed = state.detected != code;
                if changed {
                    state.detected = code;
                    state.version += 1;
                }
                changed
            };
            if changed {
                report(&state, sink.as_ref());
            }
        }
        VoiceEvent::Usage {
            audio_seconds,
            keyterms,
        } => {
            let usage = VoiceUsage {
                characters: 0,
                audio_seconds,
                keyterm_seconds: if keyterms { audio_seconds } else { 0.0 },
            };
            let model = lock(&state).settings.stt_model.clone();
            let cost = voice_cost_usd(&usage, "", &model);
            spent(usage, cost);
        }
    }
}

/// Hands the sink the current voice and subtitle statuses, and again until no change raced them.
/// The state is never held meanwhile, since the sink may call back into the link.
fn report(state: &Mutex<LinkState>, sink: &dyn Sink) {
    loop {
        let (status, subtitles, version) = {
            let state = lock(state);
            (state.status(), state.subtitle_status(), state.version)
        };
        sink.voice(status);
        sink.subtitles(subtitles);
        if lock(state).version == version {
            return;
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
