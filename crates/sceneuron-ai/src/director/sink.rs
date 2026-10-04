// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Where the director, the speaker, the voice input, and the subtitle translator report.

use std::collections::BTreeMap;

use sceneuron_core::{AiStatus, Heard, SceneObservation, SubtitleStatus, Verdict, VoiceStatus};

/// Where the director reports. Called from the director thread.
pub trait Sink: Send + Sync {
    /// The model's view of a frame captured at `captured_ms` on the host's clock.
    fn observed(&self, observation: SceneObservation, captured_ms: u64);

    /// What the director is doing; sent whenever it changes.
    fn status(&self, status: AiStatus);

    /// Judgments without a caption, about facts observed at `observed_ms` on the host's clock:
    /// quick judgments, from the quick judge's thread, and cued conditions turning off when their
    /// cue goes away, from the director thread.
    fn verdicts(&self, verdicts: Vec<Verdict>, observed_ms: u64);

    /// Scene understanding stopped after running, so conditions should turn off.
    fn reset(&self, why: &str);

    /// How voice control is set up and what it is doing; sent at the start and whenever
    /// either changes. Called from any thread.
    fn voice(&self, _status: VoiceStatus) {}

    /// Words the voice input heard. Called from the voice thread.
    fn heard(&self, _heard: &Heard) {}

    /// The audio for the speech cue with `nonce` is cached as `audio`. Called from the
    /// speaker's thread.
    fn spoken(&self, _nonce: u64, _audio: &str) {}

    /// A spoken card could not be generated, and why; for the event log.
    fn speech_failed(&self, _why: &str) {}

    /// Something about voice worth a line in the event log, such as which voice speaks cards.
    fn voice_note(&self, _note: &str) {}

    /// How subtitles are set up and what they are doing; sent with the voice status. Called
    /// from any thread.
    fn subtitles(&self, _status: SubtitleStatus) {}

    /// The translations of subtitle segment `id`, by target. Called from the translator's thread.
    fn translated(&self, _id: u64, _translations: &BTreeMap<String, String>) {}

    /// Subtitle segment `id` will not be translated, and why, without its words. Called from the
    /// translator's thread.
    fn translation_failed(&self, _id: u64, _why: &str) {}
}
