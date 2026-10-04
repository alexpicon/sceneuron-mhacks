// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Cued conditions in the director: conditions the vision model judges only while on-device
//! analysis reports a fact, such as the tongue-out reaction, which waits for an open mouth.
//!
//! While its fact is missing, a cued condition costs nothing: the vision model is not asked about
//! it, Jev never is, and no frame is converted or look made for it alone. Once the fact has shown
//! without a break for [`CUE_DWELL_MS`], the director asks for a frame taken from then on and
//! looks at it, which a short opening of the mouth in speech rarely causes. When the fact goes
//! away after the vision model saw the condition hold, the condition turns off without a look, so
//! the next time it holds is news again.
//!
//! A look that does not see the condition hold rests it: its cue does not arm again for
//! [`FIRST_REST_MS`], twice that after the next such look, and so on up to [`MAX_REST_MS`], until a
//! look sees it hold. Talking keeps opening the mouth for longer than the dwell time, so without a
//! rest it would bring a look every second or two.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::atomic::Ordering;

use sceneuron_core::{AiPolicy, Condition, Cue, SceneObservation, Verdict};

use super::inbox::Watch;
use super::worker::Worker;
use super::{Director, NO_FRAME};
use crate::gate::Reason;

/// How long a cue's fact must show without a break before the director looks for its condition.
const CUE_DWELL_MS: u64 = 400;
/// How long a cued condition rests after a look that did not see it hold.
const FIRST_REST_MS: u64 = 20_000;
/// The longest rest, reached after four such looks in a row.
const MAX_REST_MS: u64 = 160_000;

/// What the director knows about the cues of the conditions it watches.
#[derive(Debug, Default)]
pub(super) struct CueWatch {
    /// When each cued condition's fact began to show without a break, by condition id.
    seen_since: BTreeMap<String, u64>,
    /// Cued conditions whose fact has shown for the dwell time.
    armed: BTreeSet<String>,
    /// Cued conditions the vision model last saw hold.
    held: BTreeSet<String>,
    /// Cued conditions that looks did not see hold since one last did, by condition id.
    rests: BTreeMap<String, Rest>,
    /// When the newest facts were observed, on the host's clock.
    latest_ms: u64,
    /// A look for a newly armed condition waits for a frame taken at or after this time.
    fresh_after_ms: Option<u64>,
}

/// A cued condition's rest after looks that did not see it hold.
#[derive(Clone, Copy, Debug)]
struct Rest {
    /// Such looks in a row.
    misses: u32,
    /// How long this rest lasts.
    ms: u64,
    /// The cue does not arm before this time, on the host's clock.
    until_ms: u64,
}

impl CueWatch {
    /// Rests `id` after a look at a frame captured at `captured_ms` did not see it hold.
    fn rest(&mut self, id: String, captured_ms: u64) {
        let misses = self.rests.get(&id).map_or(0, |rest| rest.misses).saturating_add(1);
        let doublings = misses.saturating_sub(1).min(16);
        let ms = (FIRST_REST_MS << doublings).min(MAX_REST_MS);
        let until_ms = captured_ms.saturating_add(ms);
        self.rests.insert(id, Rest { misses, ms, until_ms });
    }

    /// The rest `id` is in at `now_ms`, if any.
    fn resting(&self, id: &str, now_ms: u64) -> Option<Rest> {
        self.rests.get(id).copied().filter(|rest| now_ms < rest.until_ms)
    }
}

impl Director {
    /// Like [`Director::set_project`], where each of `cues` names one of `conditions` that the
    /// vision model judges only while the facts given to [`Director::hint`] report the cue's
    /// fact; see [`Cue`].
    pub fn set_project_with_cues(
        &self,
        conditions: Vec<Condition>,
        cues: Vec<Cue>,
        policy: AiPolicy,
        wants_caption: bool,
    ) {
        self.set_watch(Watch {
            conditions,
            policy,
            wants_caption,
            cues,
        });
    }
}

impl Worker {
    /// The conditions to ask the vision model about now: the enabled ones, cued ones only while
    /// armed.
    pub(super) fn judged(&self) -> Vec<&Condition> {
        let cued = |id: &str| self.watch.cues.iter().any(|cue| cue.condition == id);
        self.watch
            .enabled()
            .into_iter()
            .filter(|condition| !cued(&condition.id) || self.cues.armed.contains(&condition.id))
            .collect()
    }

    /// Whether a look now would ask anything: the caption, or a condition to judge.
    pub(super) fn has_question(&self) -> bool {
        self.watch.wants_caption || !self.judged().is_empty()
    }

    /// Takes the newest facts, observed at `observed_ms` on the host's clock: arms the cues they
    /// have shown for the dwell time unless their conditions rest, and disarms the ones they no
    /// longer show.
    pub(super) fn take_hints(&mut self, hints: Vec<String>, observed_ms: u64) {
        self.hints = hints;
        self.cues.latest_ms = observed_ms;
        let cues = self.watch.cues.clone();
        let mut armed_now = false;
        for cue in &cues {
            let id = &cue.condition;
            if !cue.seen_in(&self.hints) {
                self.cues.seen_since.remove(id);
                if self.cues.armed.remove(id) && self.cues.held.remove(id) {
                    self.sink.verdicts(vec![cue_gone(cue)], observed_ms);
                }
                continue;
            }
            let since = *self.cues.seen_since.entry(id.clone()).or_insert(observed_ms);
            let dwelt = observed_ms.saturating_sub(since) >= CUE_DWELL_MS;
            if dwelt && self.cues.resting(id, observed_ms).is_none() && self.cues.armed.insert(id.clone()) {
                armed_now = true;
            }
        }
        if armed_now {
            self.gate.note(Reason::Novel);
            self.cues.fresh_after_ms = Some(observed_ms);
            self.shared.last_frame_ms.store(NO_FRAME, Ordering::Relaxed);
        }
    }

    /// Drops a frame taken before a cue armed, so the look sees what the cue saw.
    pub(super) fn keep_fresh_candidate(&mut self) {
        let Some(after_ms) = self.cues.fresh_after_ms else {
            return;
        };
        match &self.candidate {
            Some(candidate) if candidate.frame.captured_ms < after_ms => self.candidate = None,
            Some(_) => self.cues.fresh_after_ms = None,
            None => {}
        }
    }

    /// Takes the vision model's answer about a frame captured at `captured_ms`, `None` when it
    /// declined to judge it, for the cued conditions the look asked about, which are the armed
    /// ones: a condition seen to hold, as surely as it asks, stops resting, and one not seen to
    /// hold is disarmed and rests. One the model saw hold before and says nothing about now stays
    /// on until its fact goes away.
    pub(super) fn cues_answered(&mut self, observation: Option<&SceneObservation>, captured_ms: u64) {
        let asked: Vec<String> = self.cues.armed.iter().cloned().collect();
        for id in asked {
            let condition = self.watch.conditions.iter().find(|condition| condition.id == id);
            let min_confidence = condition.map_or(0.0, |condition| condition.min_confidence);
            let holds = observation
                .and_then(|observation| observation.verdicts.iter().find(|verdict| verdict.condition == id))
                .map(|verdict| verdict.holds && verdict.confidence >= min_confidence);
            match holds {
                Some(true) => {
                    self.cues.rests.remove(&id);
                    self.cues.held.insert(id);
                }
                None if self.cues.held.contains(&id) => {}
                _ => {
                    self.cues.held.remove(&id);
                    self.cues.armed.remove(&id);
                    self.cues.rest(id, captured_ms);
                }
            }
        }
    }

    /// Forgets the cue state of conditions no longer cued, and all of it once the director
    /// stops, since the host turns every condition off then.
    pub(super) fn forget_cues(&mut self, stopped: bool) {
        if stopped {
            self.cues = CueWatch::default();
            return;
        }
        let cued: BTreeSet<&String> = self.watch.cues.iter().map(|cue| &cue.condition).collect();
        let keep = |id: &String| cued.contains(id);
        self.cues.seen_since.retain(|id, _| keep(id));
        self.cues.armed.retain(|id| keep(id));
        self.cues.held.retain(|id| keep(id));
        self.cues.rests.retain(|id, _| keep(id));
    }

    /// Why nothing is asked while only cued conditions wait for their facts or rest, for the
    /// status.
    pub(super) fn waiting_note(&self) -> Option<String> {
        if self.has_question() || self.watch.cues.is_empty() {
            return None;
        }
        let facts: Vec<&str> = self.watch.cues.iter().map(|cue| cue.fact.as_str()).collect();
        let mut note = format!("waiting for on-device analysis to see {}", facts.join(" or "));
        for cue in &self.watch.cues {
            if let Some(rest) = self.cues.resting(&cue.condition, self.cues.latest_ms) {
                let condition = self
                    .watch
                    .conditions
                    .iter()
                    .find(|condition| condition.id == cue.condition);
                let name = condition.map_or(cue.condition.as_str(), |condition| condition.name.as_str());
                let seconds = rest.ms / 1_000;
                let _ = write!(note, "; {name} rests {seconds} s after a look found it not to hold");
            }
        }
        Some(note)
    }
}

/// The verdict that turns a cued condition off when its fact goes away.
fn cue_gone(cue: &Cue) -> Verdict {
    Verdict {
        condition: cue.condition.clone(),
        holds: false,
        confidence: 1.0,
        detail: String::new(),
        reason: format!("on-device analysis no longer sees {}", cue.fact),
    }
}
