// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A director sink that records everything it receives, and polling helpers.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use sceneuron_ai::Sink;
use sceneuron_core::{AiStatus, SceneObservation, SubtitleStatus, Verdict, VoiceStatus};

/// Everything a sink can receive.
#[derive(Clone, Debug, PartialEq)]
pub enum Received {
    Observed(SceneObservation, u64),
    Verdicts(Vec<Verdict>, u64),
    Status(AiStatus),
    Reset(String),
    Voice(VoiceStatus),
    Spoken(u64, String),
    SpeechFailed(String),
    VoiceNote(String),
    Subtitles(SubtitleStatus),
    Translated(u64, BTreeMap<String, String>),
    TranslationFailed(u64, String),
}

/// What became of one subtitle segment's translation.
pub type Translation = (u64, Result<BTreeMap<String, String>, String>);

#[derive(Default)]
pub struct RecordingSink {
    received: Mutex<Vec<Received>>,
}

impl RecordingSink {
    pub fn received(&self) -> Vec<Received> {
        self.received.lock().unwrap().clone()
    }

    pub fn statuses(&self) -> Vec<AiStatus> {
        let received = self.received();
        received
            .into_iter()
            .filter_map(|item| match item {
                Received::Status(status) => Some(status),
                _ => None,
            })
            .collect()
    }

    /// State names in order, with consecutive repeats collapsed.
    pub fn states(&self) -> Vec<String> {
        let mut states: Vec<String> = self.statuses().into_iter().map(|status| status.state).collect();
        states.dedup();
        states
    }

    pub fn last_status(&self) -> Option<AiStatus> {
        self.statuses().pop()
    }

    pub fn observations(&self) -> Vec<(SceneObservation, u64)> {
        let received = self.received();
        received
            .into_iter()
            .filter_map(|item| match item {
                Received::Observed(observation, at) => Some((observation, at)),
                _ => None,
            })
            .collect()
    }

    pub fn verdicts(&self) -> Vec<(Vec<Verdict>, u64)> {
        let received = self.received();
        received
            .into_iter()
            .filter_map(|item| match item {
                Received::Verdicts(verdicts, at) => Some((verdicts, at)),
                _ => None,
            })
            .collect()
    }

    pub fn resets(&self) -> Vec<String> {
        let received = self.received();
        received
            .into_iter()
            .filter_map(|item| match item {
                Received::Reset(why) => Some(why),
                _ => None,
            })
            .collect()
    }

    /// Voice statuses, spoken cues, and speech failures, in order.
    pub fn voice_events(&self) -> Vec<Received> {
        let received = self.received();
        received
            .into_iter()
            .filter(|item| {
                matches!(
                    item,
                    Received::Voice(_) | Received::Spoken(..) | Received::SpeechFailed(_)
                )
            })
            .collect()
    }

    pub fn spoken(&self) -> Vec<(u64, String)> {
        let received = self.received();
        received
            .into_iter()
            .filter_map(|item| match item {
                Received::Spoken(nonce, audio) => Some((nonce, audio)),
                _ => None,
            })
            .collect()
    }

    pub fn voice_notes(&self) -> Vec<String> {
        let received = self.received();
        received
            .into_iter()
            .filter_map(|item| match item {
                Received::VoiceNote(note) => Some(note),
                _ => None,
            })
            .collect()
    }

    pub fn speech_failures(&self) -> Vec<String> {
        let received = self.received();
        received
            .into_iter()
            .filter_map(|item| match item {
                Received::SpeechFailed(why) => Some(why),
                _ => None,
            })
            .collect()
    }

    /// Translations and translation failures, in order, by segment id.
    pub fn translations(&self) -> Vec<Translation> {
        let received = self.received();
        received
            .into_iter()
            .filter_map(|item| match item {
                Received::Translated(id, translations) => Some((id, Ok(translations))),
                Received::TranslationFailed(id, why) => Some((id, Err(why))),
                _ => None,
            })
            .collect()
    }

    /// Waits until the last status has `state`.
    pub fn wait_for_state(&self, state: &str) -> AiStatus {
        let reached = wait_until(|| self.last_status().is_some_and(|status| status.state == state));
        let last = self.last_status();
        assert!(reached, "never reached state {state:?}; last status {last:?}");
        last.unwrap()
    }
}

impl Sink for RecordingSink {
    fn observed(&self, observation: SceneObservation, captured_ms: u64) {
        self.received
            .lock()
            .unwrap()
            .push(Received::Observed(observation, captured_ms));
    }

    fn status(&self, status: AiStatus) {
        self.received.lock().unwrap().push(Received::Status(status));
    }

    fn verdicts(&self, verdicts: Vec<Verdict>, observed_ms: u64) {
        self.received
            .lock()
            .unwrap()
            .push(Received::Verdicts(verdicts, observed_ms));
    }

    fn reset(&self, why: &str) {
        self.received.lock().unwrap().push(Received::Reset(why.to_owned()));
    }

    fn voice(&self, status: VoiceStatus) {
        self.received.lock().unwrap().push(Received::Voice(status));
    }

    fn spoken(&self, nonce: u64, audio: &str) {
        self.received
            .lock()
            .unwrap()
            .push(Received::Spoken(nonce, audio.to_owned()));
    }

    fn speech_failed(&self, why: &str) {
        self.received
            .lock()
            .unwrap()
            .push(Received::SpeechFailed(why.to_owned()));
    }

    fn voice_note(&self, note: &str) {
        self.received.lock().unwrap().push(Received::VoiceNote(note.to_owned()));
    }

    fn subtitles(&self, status: SubtitleStatus) {
        self.received.lock().unwrap().push(Received::Subtitles(status));
    }

    fn translated(&self, id: u64, translations: &BTreeMap<String, String>) {
        let received = Received::Translated(id, translations.clone());
        self.received.lock().unwrap().push(received);
    }

    fn translation_failed(&self, id: u64, why: &str) {
        let received = Received::TranslationFailed(id, why.to_owned());
        self.received.lock().unwrap().push(received);
    }
}

/// Polls `condition` every 5 ms for up to 5 seconds.
pub fn wait_until(mut condition: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if condition() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(5));
    }
}
