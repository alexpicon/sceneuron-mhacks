// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The simulated vision model's loop: while watching is on, it looks at the scripted camera every
//! few seconds or when asked, feeds the result to the engine, and reports a status whose message
//! always says that it is simulated. Spend is estimated from the settings' prices, so the hourly
//! budget pauses it like the real one.

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use sceneuron_core::AiStatus;
use sceneuron_server::Hub;

use super::scenes;
use super::settings::DemoSettings;

const LOOK_INTERVAL: Duration = Duration::from_secs(3);
/// How long each simulated look "thinks", so the editor can show it.
const THINKING: Duration = Duration::from_millis(350);
const INPUT_TOKENS: u64 = 1_450;
const OUTPUT_TOKENS: u64 = 95;
const HOUR_MS: u64 = 3_600_000;

/// What wakes the loop before its next look is due.
#[derive(Debug, Default)]
struct Signal {
    stop: bool,
    look: bool,
    changed: bool,
}

/// State shared by the assistant, which the server calls, and the loop.
#[derive(Debug, Default)]
pub(crate) struct Shared {
    settings: Mutex<DemoSettings>,
    signal: Mutex<Signal>,
    wake: Condvar,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Shared {
    pub(crate) fn settings(&self) -> MutexGuard<'_, DemoSettings> {
        lock(&self.settings)
    }

    pub(crate) fn look_now(&self) {
        self.raise(|signal| signal.look = true);
    }

    pub(crate) fn settings_changed(&self) {
        self.raise(|signal| signal.changed = true);
    }

    pub(crate) fn stop(&self) {
        self.raise(|signal| signal.stop = true);
    }

    fn raise(&self, set: impl FnOnce(&mut Signal)) {
        set(&mut lock(&self.signal));
        self.wake.notify_all();
    }

    /// Waits until `deadline` or a signal, and returns the signals raised meanwhile.
    fn wait(&self, deadline: Instant) -> Signal {
        let mut signal = lock(&self.signal);
        loop {
            if signal.stop || signal.look || signal.changed {
                return std::mem::take(&mut *signal);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Signal::default();
            }
            signal = self
                .wake
                .wait_timeout(signal, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }
}

/// Usage so far, as the status reports it.
#[derive(Debug, Default)]
struct Usage {
    calls: u64,
    input_tokens: u64,
    output_tokens: u64,
    cost_usd: f64,
    last_latency_ms: Option<f64>,
    /// Spend per look in the last hour, oldest first.
    recent: VecDeque<(u64, f64)>,
}

impl Usage {
    fn record(&mut self, settings: &DemoSettings, latency_ms: f64, now_ms: u64) {
        let cost = (INPUT_TOKENS as f64 * settings.input_price_per_mtok
            + OUTPUT_TOKENS as f64 * settings.output_price_per_mtok)
            / 1_000_000.0;
        self.calls += 1;
        self.input_tokens += INPUT_TOKENS;
        self.output_tokens += OUTPUT_TOKENS;
        self.cost_usd += cost;
        self.last_latency_ms = Some(latency_ms);
        self.recent.push_back((now_ms, cost));
    }

    fn hour_cost(&mut self, now_ms: u64) -> f64 {
        while self
            .recent
            .front()
            .is_some_and(|(at, _)| now_ms.saturating_sub(*at) >= HOUR_MS)
        {
            self.recent.pop_front();
        }
        self.recent.iter().map(|(_, cost)| cost).sum()
    }

    fn status(&mut self, settings: &DemoSettings, state: &str, message: String, now_ms: u64) -> AiStatus {
        AiStatus {
            state: state.to_owned(),
            message,
            provider: "simulated".to_owned(),
            model: format!("{} (simulated)", settings.vision_model),
            calls: self.calls,
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            cost_usd: self.cost_usd,
            hour_cost_usd: self.hour_cost(now_ms),
            last_latency_ms: self.last_latency_ms,
            quick_message: "quick judgments are not simulated".to_owned(),
            ..AiStatus::default()
        }
    }
}

/// Runs until [`Shared::stop`].
pub(crate) fn run(shared: &Shared, hub: &Hub) {
    let mut usage = Usage::default();
    let mut step = 0;
    let mut watching = false;
    let mut next_look = Instant::now();
    loop {
        let signal = shared.wait(next_look);
        if signal.stop {
            return;
        }
        let settings = shared.settings().clone();
        let publish = |usage: &mut Usage, state: &str, message: String| {
            let status = usage.status(&settings, state, message, hub.now_ms());
            hub.update(|engine, now| engine.set_ai_status(status, now));
        };
        if !(settings.consent && settings.enabled) {
            if watching {
                watching = false;
                hub.update(|engine, now| engine.reset_conditions("the simulated AI stopped watching", now));
            }
            let (state, message) = if settings.consent {
                ("off", "Simulated AI is off. Nothing is sent anywhere.")
            } else {
                (
                    "needs_setup",
                    "Simulated AI: confirm consent in the AI tab to start watching.",
                )
            };
            publish(&mut usage, state, message.to_owned());
            next_look = Instant::now() + LOOK_INTERVAL;
            continue;
        }
        let due = signal.look || !watching || Instant::now() >= next_look;
        if !watching {
            watching = true;
            step = 0;
        }
        if !due {
            continue;
        }
        let policy = hub.read(|engine| engine.project().ai.clone());
        next_look = Instant::now() + LOOK_INTERVAL.max(Duration::from_millis(policy.min_interval_ms));
        if usage.hour_cost(hub.now_ms()) >= policy.max_cost_per_hour {
            let message = format!(
                "Simulated AI paused: this hour's estimated spend reached the ${:.2} budget.",
                policy.max_cost_per_hour
            );
            publish(&mut usage, "paused", message);
            continue;
        }
        let watching_message = "Simulated AI: looking at a scripted scene. No camera image or API is used.";
        publish(&mut usage, "thinking", watching_message.to_owned());
        let started = Instant::now();
        thread::sleep(THINKING);
        let conditions = hub.read(|engine| engine.project().conditions.clone());
        let observation = scenes::observation(step, &conditions);
        step += 1;
        hub.update(|engine, now| engine.observe_scene(observation, now));
        usage.record(&settings, started.elapsed().as_secs_f64() * 1000.0, hub.now_ms());
        publish(&mut usage, "watching", watching_message.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_counts_looks_and_forgets_spend_after_an_hour() {
        let settings = DemoSettings::default();
        let mut usage = Usage::default();
        usage.record(&settings, 350.0, 1_000);
        usage.record(&settings, 360.0, 2_000);
        let per_look = (1_450.0 * 1.0 + 95.0 * 5.0) / 1_000_000.0;
        assert!((usage.hour_cost(2_000) - 2.0 * per_look).abs() < 1e-12);
        assert!((usage.hour_cost(HOUR_MS + 1_500) - per_look).abs() < 1e-12);
        let status = usage.status(&settings, "watching", "Simulated".to_owned(), HOUR_MS + 1_500);
        assert_eq!(
            (status.calls, status.input_tokens, status.output_tokens),
            (2, 2_900, 190)
        );
        assert_eq!(status.model, "simulated-vision (simulated)");
        assert_eq!(status.last_latency_ms, Some(360.0));
        assert!((status.cost_usd - 2.0 * per_look).abs() < 1e-12);
    }

    #[test]
    fn signals_wake_the_loop_and_are_cleared_once_read() {
        let shared = Shared::default();
        shared.look_now();
        let far = Instant::now() + Duration::from_secs(60);
        assert!(shared.wait(far).look);
        let soon = Instant::now() + Duration::from_millis(20);
        let quiet = shared.wait(soon);
        assert!(!quiet.look && !quiet.stop && !quiet.changed);
        shared.stop();
        assert!(shared.wait(far).stop);
    }
}
