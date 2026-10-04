// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! One vision call: encode the frame, ask the model, and account for the answer.

use super::worker::Worker;
use crate::cost::{cost_usd, prices_for};
use crate::image::encode_jpeg;
use crate::judge::{judge_request, parse_observation};
use crate::provider::{AiError, Usage};

impl Worker {
    /// Judges the newest frame. The director's locks are not held during the call, so hosts
    /// keep offering frames and changes while the model thinks.
    pub(super) fn call(&mut self, reason: &'static str) {
        let (Some(candidate), Some(provider)) = (self.candidate.take(), self.provider.clone()) else {
            return;
        };
        let jpeg = match encode_jpeg(&candidate.frame, self.settings.image_width) {
            Ok(jpeg) => jpeg,
            Err(error) => {
                self.note = error.to_string();
                return;
            }
        };
        let judged: Vec<_> = self.judged().into_iter().cloned().collect();
        let request = judge_request(&judged, &self.hints, jpeg, &self.settings);
        self.gate.started(self.now_ms(), candidate.thumbnail);
        self.thinking = Some(reason);
        self.publish();
        let result = provider.complete(&request);
        self.thinking = None;
        let now_ms = self.now_ms();
        match result {
            Ok(reply) => {
                let prices = prices_for(&self.settings, &reply.model);
                let cost = prices.map_or(0.0, |prices| cost_usd(&reply.usage, &prices));
                self.account(&reply.model, &reply.usage, cost, now_ms);
                self.totals.last_latency_ms = Some(reply.latency_ms);
                match parse_observation(&reply.json) {
                    Ok(observation) => {
                        self.gate.succeeded();
                        self.note.clear();
                        self.cues_answered(Some(&observation), candidate.frame.captured_ms);
                        if !self.outdated(&request.model) {
                            self.sink.observed(observation, candidate.frame.captured_ms);
                        }
                    }
                    Err(error) => self.transient(error, now_ms),
                }
            }
            // A refusal is an answer: it counts as a call and does not back off.
            Err(AiError::Refused(_)) => {
                self.totals.calls += 1;
                self.gate.succeeded();
                self.cues_answered(None, candidate.frame.captured_ms);
                self.note = "the model declined to judge this image".to_owned();
            }
            Err(error) if error.is_permanent() => self.failure = Some(error.to_string()),
            Err(error) => {
                if matches!(error, AiError::Invalid(_)) {
                    self.totals.calls += 1;
                }
                self.transient(error, now_ms);
            }
        }
    }

    /// Adds a call's usage and cost to the totals and the hourly budget.
    pub(super) fn account(&mut self, model: &str, usage: &Usage, cost: f64, now_ms: u64) {
        self.totals.add(usage, cost);
        self.gate.spent(now_ms, cost);
        self.note_price(model);
    }

    /// Remembers `model` for the status when its spend counts as zero for want of a price.
    pub(super) fn note_price(&mut self, model: &str) {
        if prices_for(&self.settings, model).is_none() {
            self.unpriced.insert(model.to_owned());
        }
    }

    /// Whether settings or conditions changed while the call was in flight, so its answer
    /// describes a question nobody is asking anymore.
    fn outdated(&self, model: &str) -> bool {
        let inbox = self.shared.lock();
        let settings_changed = inbox
            .settings
            .as_ref()
            .is_some_and(|settings| !settings.enabled || !settings.consent || settings.vision_model != model);
        let watch_changed = inbox
            .watch
            .as_ref()
            .is_some_and(|watch| !watch.policy.enabled || watch.uncued() != self.watch.uncued());
        inbox.shutdown || settings_changed || watch_changed
    }

    fn transient(&mut self, error: AiError, now_ms: u64) {
        let backoff_ms = self.gate.failed(now_ms);
        self.note = format!("retrying in {} s: {error}", backoff_ms.div_ceil(1000));
        self.wake_at_ms = Some(now_ms.saturating_add(backoff_ms));
    }
}
