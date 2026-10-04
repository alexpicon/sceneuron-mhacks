// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What the director reports: running totals and the status the editor shows.

use std::sync::PoisonError;
use std::sync::atomic::Ordering;

use sceneuron_core::AiStatus;

use super::worker::Worker;
use crate::cost::prices_for;
use crate::gate::Decision;
use crate::provider::Usage;
use crate::voice_cost::VoiceUsage;

/// Consecutive transient failures from which the status reads `failed` while retrying.
const FAILED_AFTER: u32 = 3;

/// Calls, tokens, and spend since the director started.
#[derive(Clone, Debug, Default)]
pub(super) struct Totals {
    pub calls: u64,
    /// Every input token, cached or not.
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub last_latency_ms: Option<f64>,
    pub voice_characters: u64,
    pub voice_seconds: f64,
    pub voice_cost_usd: f64,
    pub translation_calls: u64,
    pub translation_input_tokens: u64,
    pub translation_output_tokens: u64,
    pub translation_cost_usd: f64,
}

impl Totals {
    /// Spend on quick judgments, which count toward the budget but not as looks.
    pub fn add_quick(&mut self, cost_usd: f64) {
        if cost_usd.is_finite() && cost_usd > 0.0 {
            self.cost_usd += cost_usd;
        }
    }

    /// Voice usage, which counts toward the spend and the budget but not as looks.
    pub fn add_voice(&mut self, usage: &VoiceUsage, cost_usd: f64) {
        self.voice_characters += usage.characters;
        if usage.audio_seconds.is_finite() && usage.audio_seconds > 0.0 {
            self.voice_seconds += usage.audio_seconds;
        }
        if cost_usd.is_finite() && cost_usd > 0.0 {
            self.voice_cost_usd += cost_usd;
            self.cost_usd += cost_usd;
        }
    }

    /// A subtitle translation, which counts toward the spend and the budget but not as a look.
    pub fn add_translation(&mut self, usage: &Usage, cost_usd: f64) {
        self.translation_calls += 1;
        self.translation_input_tokens += usage.input_tokens + usage.cache_read_tokens;
        self.translation_output_tokens += usage.output_tokens;
        if cost_usd.is_finite() && cost_usd > 0.0 {
            self.translation_cost_usd += cost_usd;
            self.cost_usd += cost_usd;
        }
    }

    pub fn add(&mut self, usage: &Usage, cost_usd: f64) {
        self.calls += 1;
        self.input_tokens += usage.input_tokens + usage.cache_read_tokens;
        self.output_tokens += usage.output_tokens;
        if cost_usd.is_finite() && cost_usd > 0.0 {
            self.cost_usd += cost_usd;
        }
    }
}

impl Worker {
    /// The state name and the note that explains it.
    pub(super) fn state(&self) -> (&'static str, String) {
        if !self.settings.enabled {
            return ("off", "scene understanding is off in the AI settings".to_owned());
        }
        if !self.watch.policy.enabled {
            return ("off", "scene understanding is off for this project".to_owned());
        }
        if !self.settings.consent {
            let message = "confirm that camera images may be sent to the provider".to_owned();
            return ("needs_setup", message);
        }
        if let Some(message) = &self.setup_error {
            return ("needs_setup", message.clone());
        }
        if let Some(message) = &self.failure {
            return ("failed", message.clone());
        }
        if !self.has_work() {
            let message = "add a condition, or {{scene}} to a text, for the vision model to watch".to_owned();
            return ("off", message);
        }
        if let Some(message) = &self.paused {
            return ("paused", message.clone());
        }
        if let Some(reason) = self.thinking {
            return ("thinking", format!("looking at the camera ({reason})"));
        }
        if self.gate.failures() >= FAILED_AFTER {
            return ("failed", self.note.clone());
        }
        ("watching", self.watching_note())
    }

    /// The status sent to the sink at `now_ms`.
    pub(super) fn status(&self, now_ms: u64) -> AiStatus {
        let (state, message) = self.state();
        let quick = self.shared.quick.lock().unwrap_or_else(PoisonError::into_inner).clone();
        let provider = match &self.provider {
            Some(provider) => provider.kind(),
            None => self.settings.provider.name(),
        };
        AiStatus {
            state: state.to_owned(),
            message,
            provider: provider.to_owned(),
            model: self.settings.vision_model.clone(),
            calls: self.totals.calls,
            input_tokens: self.totals.input_tokens,
            output_tokens: self.totals.output_tokens,
            cost_usd: self.totals.cost_usd,
            hour_cost_usd: self.gate.hour_cost(now_ms),
            last_latency_ms: self.totals.last_latency_ms,
            quick_state: quick.state.to_owned(),
            quick_message: quick.message,
            quick_calls: quick.calls,
            quick_latency_ms: quick.latency_ms,
            voice_characters: self.totals.voice_characters,
            voice_seconds: self.totals.voice_seconds,
            voice_cost_usd: self.totals.voice_cost_usd,
            translation_calls: self.totals.translation_calls,
            translation_input_tokens: self.totals.translation_input_tokens,
            translation_output_tokens: self.totals.translation_output_tokens,
            translation_cost_usd: self.totals.translation_cost_usd,
        }
    }

    /// Tells spoken cards and voice control whether the hourly budget is spent, and wakes when
    /// it no longer is.
    pub(super) fn watch_budget(&mut self) {
        let over = self.gate.pause(&self.watch.policy, self.now_ms());
        if let Some(Decision::Paused { until_ms, .. }) = over {
            self.wake_at_ms = Some(self.wake_at_ms.map_or(until_ms, |at| at.min(until_ms)));
        }
        let spent = over.is_some();
        if self.shared.over_budget.swap(spent, Ordering::Relaxed) != spent {
            self.shared.budget_changed(spent);
        }
    }

    /// The current note, or else what cued conditions wait for, or else which models' spend is
    /// not counted.
    fn watching_note(&self) -> String {
        let mut unpriced = self.unpriced.clone();
        let vision_model = &self.settings.vision_model;
        if prices_for(&self.settings, vision_model).is_none() {
            unpriced.insert(vision_model.clone());
        }
        if self.note.is_empty()
            && let Some(waiting) = self.waiting_note()
        {
            return waiting;
        }
        if !self.note.is_empty() || unpriced.is_empty() {
            return self.note.clone();
        }
        let models: Vec<&str> = unpriced.iter().map(String::as_str).collect();
        format!(
            "no known price for {}, so its spend counts as $0; set prices in the AI settings",
            models.join(", ")
        )
    }
}
