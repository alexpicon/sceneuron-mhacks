// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What voice control asks of the host: a look, a subtitles change, or words for the AI. Each
//! waits as a [`VoiceRequest`] until the host replies, and the heard line says it is waiting. The
//! reply's actions are carried out like local commands, and the heard line and the event log say
//! what the AI understood and what was done, or why not.

use super::actions::{RoomStep, VoiceAction, VoiceReply, VoiceRequest, VoiceStep, VoiceTask};
use crate::engine::Engine;
use crate::subtitles::language_label;
use crate::types::Outcome;

/// Requests that may wait at once; an older one is dropped for a new one.
const MAX_WAITING: usize = 3;
/// How long a request may wait for its reply: longer than a model call and the calls queued
/// before it.
pub const VOICE_REQUEST_TIMEOUT_MS: u64 = 150_000;

impl Engine {
    /// Says whether a host answers voice requests, as Sceneuron's AI does once it follows the
    /// engine. Until one does, a request is refused at once rather than left to wait.
    pub fn set_voice_host(&mut self, answers: bool) -> Outcome {
        self.voice.answered = answers;
        Outcome::NONE
    }

    /// The requests waiting for the host, oldest first; each is answered once, with
    /// [`Engine::voice_replied`].
    pub fn voice_requests(&self) -> &[VoiceRequest] {
        &self.voice.requests
    }

    /// Carries out the host's reply to the request `nonce`: `Err` says why nothing could be done.
    /// A reply to a request that is no longer waiting, because it ran out of time or a newer
    /// request replaced it, does nothing.
    pub fn voice_replied(&mut self, nonce: u64, reply: Result<VoiceReply, String>, now_ms: u64) -> Outcome {
        let Some(index) = self.voice.requests.iter().position(|request| request.nonce == nonce) else {
            return Outcome::NONE;
        };
        let request = self.voice.requests.remove(index);
        let mut saved = false;
        let result = match reply {
            Err(why) => why,
            Ok(VoiceReply { understood, steps }) => {
                let mut done = Vec::new();
                for step in steps {
                    match step {
                        VoiceStep::Done(what) => done.push(what),
                        VoiceStep::Act(action) => {
                            let outcome = self.act(&action, now_ms);
                            saved |= outcome.saved;
                            done.push(outcome.result);
                        }
                    }
                }
                match (&request.task, done.is_empty()) {
                    (VoiceTask::Ask { .. }, true) if understood.is_empty() => "the AI found nothing to do".to_owned(),
                    (VoiceTask::Ask { .. }, true) => format!("the AI found nothing to do: {understood}"),
                    (VoiceTask::Ask { .. }, false) => format!("the AI took it as {understood:?}: {}", done.join("; ")),
                    (VoiceTask::Act { .. }, _) => done.join("; "),
                }
            }
        };
        self.answer_heard(nonce, &result, now_ms);
        self.bump(if saved { Outcome::SAVED } else { Outcome::CHANGED })
    }

    /// Queues a task for the host; returns what the heard line says while it waits, or why it is
    /// refused when no host answers.
    pub(super) fn ask_host(&mut self, task: VoiceTask, now_ms: u64) -> String {
        if !self.voice.answered {
            return unanswered(&task);
        }
        let waiting = waiting_for(&task);
        if self.voice.requests.len() == MAX_WAITING {
            let dropped = self.voice.requests.remove(0);
            self.answer_heard(dropped.nonce, "dropped, because newer commands were waiting", now_ms);
        }
        let nonce = self.take_nonce();
        self.voice.requests.push(VoiceRequest {
            nonce,
            task,
            at_ms: now_ms,
        });
        self.voice.awaited = Some(nonce);
        waiting
    }

    /// Drops the requests waiting longer than [`VOICE_REQUEST_TIMEOUT_MS`], saying so.
    pub(crate) fn expire_requests(&mut self, now_ms: u64) -> Outcome {
        let late = |at_ms: u64| now_ms.saturating_sub(at_ms) > VOICE_REQUEST_TIMEOUT_MS;
        let expired: Vec<VoiceRequest> = self
            .voice
            .requests
            .iter()
            .filter(|request| late(request.at_ms))
            .cloned()
            .collect();
        if expired.is_empty() {
            return Outcome::NONE;
        }
        self.voice.requests.retain(|request| !late(request.at_ms));
        for request in expired {
            let why = match request.task {
                VoiceTask::Ask { .. } => "the AI did not answer in time",
                VoiceTask::Act { .. } => "Sceneuron's AI did not carry it out in time",
            };
            self.answer_heard(request.nonce, why, now_ms);
        }
        self.bump(Outcome::CHANGED)
    }

    /// Logs what came of the request `nonce` and shows it on the heard line, if the line still
    /// waits for it.
    fn answer_heard(&mut self, nonce: u64, result: &str, now_ms: u64) {
        self.record(now_ms, "voice", result);
        if self.voice.awaited == Some(nonce) {
            self.voice.awaited = None;
            if let Some(heard) = self.voice.heard.as_mut() {
                result.clone_into(&mut heard.result);
            }
        }
    }
}

/// Why `task` is refused while no host answers.
fn unanswered(task: &VoiceTask) -> String {
    const NOT_RUNNING: &str = "Sceneuron's AI is not running";
    match task {
        VoiceTask::Ask { .. } => format!("no command matched, and {NOT_RUNNING} to take others"),
        VoiceTask::Act {
            action: VoiceAction::Look,
        } => format!("the vision model cannot look: {NOT_RUNNING}"),
        VoiceTask::Act {
            action: VoiceAction::Room { .. },
        } => format!("the room cannot change: {NOT_RUNNING}"),
        VoiceTask::Act { .. } => format!("subtitles cannot change: {NOT_RUNNING}"),
    }
}

/// What the heard line says while `task` waits.
fn waiting_for(task: &VoiceTask) -> String {
    match task {
        VoiceTask::Ask { .. } => "asking the AI".to_owned(),
        VoiceTask::Act { action } => match action {
            VoiceAction::Look => "asking the vision model to look".to_owned(),
            VoiceAction::Room { step: RoomStep::Reveal } => "revealing the votes".to_owned(),
            VoiceAction::Room { step: RoomStep::Next } => "starting the next round".to_owned(),
            VoiceAction::Subtitles { enabled: true } => "turning subtitles on".to_owned(),
            VoiceAction::Subtitles { enabled: false } => "turning subtitles off".to_owned(),
            VoiceAction::SubtitleLanguage { language } => {
                format!("translating subtitles into {}", language_label(language))
            }
            _ => "asking Sceneuron's AI".to_owned(),
        },
    }
}
