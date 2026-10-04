// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The mailbox between the host's threads and the director thread.

use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

use super::quick::QuickStatus;

use sceneuron_core::{AiPolicy, Condition, Cue};

use crate::image::Frame;
use crate::provider::Usage;
use crate::settings::AiSettings;
use crate::voice_cost::VoiceUsage;

/// The project inputs the director watches.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Watch {
    pub conditions: Vec<Condition>,
    pub policy: AiPolicy,
    pub wants_caption: bool,
    /// Conditions judged only while the facts show their cue; see `super::cues`.
    pub cues: Vec<Cue>,
}

impl Watch {
    /// The enabled conditions, cued ones included.
    pub fn enabled(&self) -> Vec<&Condition> {
        self.conditions.iter().filter(|condition| condition.enabled).collect()
    }

    /// The enabled conditions without a cue: quick judgments may judge these, and only a change
    /// to them is worth a new look.
    pub fn uncued(&self) -> Vec<&Condition> {
        let cued = |condition: &Condition| self.cues.iter().any(|cue| cue.condition == condition.id);
        self.enabled()
            .into_iter()
            .filter(|condition| !cued(condition))
            .collect()
    }
}

/// Inputs waiting for the director thread. Newer values replace older ones that were not
/// picked up yet; flags and usage records accumulate.
#[derive(Default)]
pub(super) struct Inbox {
    /// The newest frame, kept after it is picked up so the assistant can use it too.
    pub latest: Option<Frame>,
    pub fresh_frame: bool,
    pub settings: Option<AiSettings>,
    pub watch: Option<Watch>,
    pub presence_changed: bool,
    /// The newest facts, and when the host observed them.
    pub hints: Option<(Vec<String>, u64)>,
    pub novel: bool,
    pub analyze_now: bool,
    /// Calls made elsewhere: the model that answered, its usage, and its cost.
    pub usage: Vec<(String, Usage, f64)>,
    /// Quick judgments made since: their usage and cost, which count toward the budget.
    pub quick_usage: Vec<(Usage, f64)>,
    /// Voice usage since, such as spoken cards, and its cost.
    pub voice_usage: Vec<(VoiceUsage, f64)>,
    /// Subtitle translations since: the model that answered, its usage, and its cost.
    pub translation_usage: Vec<(String, Usage, f64)>,
    /// The quick judge's status changed and should be published.
    pub quick_changed: bool,
    pub shutdown: bool,
}

/// Everything picked up from the inbox in one go.
#[derive(Default)]
pub(super) struct Events {
    pub frame: Option<Frame>,
    pub settings: Option<AiSettings>,
    pub watch: Option<Watch>,
    pub presence_changed: bool,
    pub hints: Option<(Vec<String>, u64)>,
    pub novel: bool,
    pub analyze_now: bool,
    pub usage: Vec<(String, Usage, f64)>,
    pub quick_usage: Vec<(Usage, f64)>,
    pub voice_usage: Vec<(VoiceUsage, f64)>,
    pub translation_usage: Vec<(String, Usage, f64)>,
}

impl Inbox {
    pub fn has_news(&self) -> bool {
        self.fresh_frame
            || self.settings.is_some()
            || self.watch.is_some()
            || self.presence_changed
            || self.hints.is_some()
            || self.novel
            || self.analyze_now
            || !self.usage.is_empty()
            || !self.quick_usage.is_empty()
            || !self.voice_usage.is_empty()
            || !self.translation_usage.is_empty()
            || self.quick_changed
    }

    /// Takes every new input, leaving the inbox empty apart from the latest frame.
    pub fn take(&mut self) -> Events {
        let fresh = std::mem::take(&mut self.fresh_frame);
        Events {
            frame: if fresh { self.latest.clone() } else { None },
            settings: self.settings.take(),
            watch: self.watch.take(),
            presence_changed: std::mem::take(&mut self.presence_changed),
            hints: self.hints.take(),
            novel: std::mem::take(&mut self.novel),
            analyze_now: std::mem::take(&mut self.analyze_now),
            usage: std::mem::take(&mut self.usage),
            quick_usage: {
                self.quick_changed = false;
                std::mem::take(&mut self.quick_usage)
            },
            voice_usage: std::mem::take(&mut self.voice_usage),
            translation_usage: std::mem::take(&mut self.translation_usage),
        }
    }
}

/// Called with whether the hourly budget is spent.
pub(super) type BudgetWatcher = Arc<dyn Fn(bool) + Send + Sync>;

/// State shared between the handle and the thread.
pub(super) struct Shared {
    pub inbox: Mutex<Inbox>,
    pub wake: Condvar,
    /// Whether scene understanding is running, for `Director::wants_frame`.
    pub active: AtomicBool,
    /// Capture time of the last offered frame, or `u64::MAX` before the first.
    pub last_frame_ms: AtomicU64,
    /// Whether the hourly budget is spent, which also stops quick judgments.
    pub budget_spent: AtomicBool,
    /// Whether the last hour's spend reached the budget, even while scene understanding is off.
    pub over_budget: AtomicBool,
    /// Told on the director thread whether the budget is spent, each time that changes.
    pub budget_watchers: Mutex<Vec<BudgetWatcher>>,
    /// What the quick judge last reported.
    pub quick: Mutex<QuickStatus>,
}

impl Shared {
    pub fn lock(&self) -> MutexGuard<'_, Inbox> {
        self.inbox.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Changes the inbox and wakes the thread.
    pub fn post(&self, change: impl FnOnce(&mut Inbox)) {
        change(&mut self.lock());
        self.wake.notify_one();
    }

    /// Tells every budget watcher whether the budget is spent, holding no lock meanwhile.
    pub fn budget_changed(&self, spent: bool) {
        let watchers = self
            .budget_watchers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        for watcher in watchers {
            watcher(spent);
        }
    }
}
