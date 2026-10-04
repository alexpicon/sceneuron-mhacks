// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The shared overlay state that manual controls, the camera, and the AI all change.
//!
//! Effective element properties are layered: the saved base properties, then
//! the automatic layer written by rules, then the raised card of the deck the
//! element shows (or its deck's cover, while the deck is covered), then the
//! manual layer. A manual override holds until it is released, so neither
//! automation nor cards replace it.

use std::collections::{BTreeMap, VecDeque};

use crate::cards::RaisedCard;
use crate::covers::Cover;
use crate::model::{Effect, Project, ValidationError};
use crate::on_air::OnAir;
use crate::presence::{Presence, PresenceTracker};
use crate::props::{ElementProps, PropsPatch};
use crate::reactions::{self, FacepalmWatch, FloodState};
use crate::scene::{ConditionState, SceneView};
use crate::seen::LiveDescription;
use crate::signals::CameraWatch;
use crate::subtitles::SubtitleState;
use crate::types::{AiStatus, EffectState, EngineError, EventRecord, Outcome, VisionStatus};
use crate::voice::VoiceState;
use crate::votes::Votes;

const MAX_EVENTS: usize = 50;

/// Overlay state machine. All methods take the current time so behavior is deterministic.
#[derive(Clone, Debug)]
pub struct Engine {
    pub(crate) project: Project,
    pub(crate) auto: BTreeMap<String, PropsPatch>,
    pub(crate) manual: BTreeMap<String, PropsPatch>,
    pub(crate) effects: BTreeMap<String, EffectState>,
    /// The latest overlay-wide flood; kept in memory only, like effects.
    pub(crate) flood: Option<FloodState>,
    /// What the camera has shown of a hand over the face, for the facepalm reaction.
    pub(crate) facepalm: FacepalmWatch,
    /// The raised card of each deck, keyed by deck id; kept in memory only, like manual holds.
    pub(crate) raised: BTreeMap<String, RaisedCard>,
    /// What each covered deck shows instead of its raised card, keyed by deck id; in memory only.
    pub(crate) covers: BTreeMap<String, Cover>,
    /// What the camera has been showing each deck, for raising cards by their signals.
    pub(crate) camera: CameraWatch,
    /// What holds votes open, which finger counts go to.
    pub(crate) votes: Votes,
    pub(crate) presence: PresenceTracker,
    pub(crate) conditions: BTreeMap<String, ConditionState>,
    pub(crate) scene: SceneView,
    /// What on-device analysis found lately, as the `{{seen}}` placeholder shows it.
    pub(crate) live: LiveDescription,
    /// What the `{{room}}` placeholder shows, as the shared room client last set it.
    pub(crate) room_summary: String,
    /// OBS's program and whether it is on air, as the host last reported them.
    pub(crate) on_air: OnAir,
    /// Time of the newest evidence any verdict described, for turning conditions off when
    /// verdicts stop.
    pub(crate) last_report_ms: u64,
    pub(crate) vision: VisionStatus,
    pub(crate) ai: AiStatus,
    /// Voice control's setup, push-to-talk, what it last heard, and the card to speak.
    pub(crate) voice: VoiceState,
    /// Live subtitles: their setup, recent lines, and translations by segment.
    pub(crate) subtitles: SubtitleState,
    pub(crate) events: VecDeque<EventRecord>,
    pub(crate) project_revision: u64,
    revision: u64,
    next_nonce: u64,
}

impl Engine {
    pub fn new(project: Project, now_ms: u64) -> Result<Self, ValidationError> {
        project.validate()?;
        Ok(Self {
            project,
            auto: BTreeMap::new(),
            manual: BTreeMap::new(),
            effects: BTreeMap::new(),
            flood: None,
            facepalm: FacepalmWatch::default(),
            raised: BTreeMap::new(),
            covers: BTreeMap::new(),
            camera: CameraWatch::default(),
            votes: Votes::default(),
            presence: PresenceTracker::new(now_ms),
            conditions: BTreeMap::new(),
            scene: SceneView::default(),
            live: LiveDescription::default(),
            room_summary: String::new(),
            on_air: OnAir::default(),
            last_report_ms: 0,
            vision: VisionStatus::Idle,
            ai: AiStatus::default(),
            voice: VoiceState::default(),
            subtitles: SubtitleState::default(),
            events: VecDeque::new(),
            project_revision: 1,
            revision: 1,
            next_nonce: 1,
        })
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    /// Increases on every change to the effective overlay state.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Increases on every change to the saved project; editors send it back to detect conflicts.
    pub fn project_revision(&self) -> u64 {
        self.project_revision
    }

    pub fn presence(&self) -> Presence {
        self.presence.state()
    }

    /// Whether a condition is currently on.
    pub fn condition_on(&self, id: &str) -> bool {
        self.conditions.get(id).is_some_and(|state| state.on)
    }

    /// Base properties with the automatic, card, and manual layers applied and placeholders
    /// rendered; text that shows subtitles hides while they are empty.
    pub fn effective_props(&self, element_id: &str) -> Option<ElementProps> {
        let mut props = self.layered_props(element_id)?;
        self.render_placeholders(&mut props);
        Some(props)
    }

    /// Base properties with the automatic, card, and manual layers applied, placeholders and all.
    pub(crate) fn layered_props(&self, element_id: &str) -> Option<ElementProps> {
        let mut props = self.project.element(element_id)?.props.clone();
        if let Some(patch) = self.auto.get(element_id) {
            props.apply(patch);
        }
        if let Some(card) = self.card_layer(element_id) {
            props.apply(card);
            props.visible = true;
        }
        if let Some(patch) = self.manual.get(element_id) {
            props.apply(patch);
        }
        Some(props)
    }

    /// Sets what the `{{room}}` placeholder shows, such as "Voted: 3 of 5".
    pub fn set_room_summary(&mut self, summary: &str) -> Outcome {
        if self.room_summary == summary {
            return Outcome::NONE;
        }
        summary.clone_into(&mut self.room_summary);
        self.bump(Outcome::CHANGED)
    }

    /// Replaces the saved layout, conditions, rules, and settings.
    ///
    /// The live automation switch is kept, because only [`Engine::set_automation`] changes it.
    /// State for removed elements, conditions, decks, and cards, and for built-in reactions that
    /// stop watching, is dropped and the automatic layer is recomputed, so rules that no longer
    /// exist stop affecting the overlay. A raised
    /// card whose deck's auto-hide delay changed starts the new delay now.
    pub fn replace_project(&mut self, project: Project, now_ms: u64) -> Result<Outcome, ValidationError> {
        self.adopt_project(project, now_ms)?;
        self.record(now_ms, "editor", "saved the layout and rules");
        Ok(self.bump(Outcome::SAVED))
    }

    /// [`Engine::replace_project`] without logging or publishing it, for a caller that does.
    pub(crate) fn adopt_project(&mut self, mut project: Project, now_ms: u64) -> Result<(), ValidationError> {
        project.automation_enabled = self.project.automation_enabled;
        project.validate()?;
        let keep = |id: &String| project.element(id).is_some();
        self.manual.retain(|id, _| keep(id));
        self.effects.retain(|id, _| keep(id));
        self.conditions
            .retain(|id, _| project.condition(id).is_some() || reactions::watched_condition(&project, id).is_some());
        let previous = std::mem::replace(&mut self.project, project);
        self.keep_raised_cards(&previous, now_ms);
        self.keep_presses();
        self.project_revision += 1;
        self.rebuild_auto_layer();
        Ok(())
    }

    /// Updates the camera-analysis health shown in the editor. Analysis that stops or fails clears
    /// the live description.
    pub fn set_vision_status(&mut self, status: VisionStatus, now_ms: u64) -> Outcome {
        if self.vision == status {
            return Outcome::NONE;
        }
        let state_changed = std::mem::discriminant(&self.vision) != std::mem::discriminant(&status);
        if let VisionStatus::Failed { message } = &status {
            self.record(now_ms, "vision", &format!("analysis failed: {message}"));
        }
        if matches!(status, VisionStatus::Idle | VisionStatus::Failed { .. }) {
            self.forget_seen(now_ms);
        }
        self.vision = status;
        if state_changed {
            self.bump(Outcome::CHANGED)
        } else {
            Outcome {
                telemetry: true,
                ..Outcome::NONE
            }
        }
    }

    /// Holds manual values for an element until they are released.
    pub fn set_override(&mut self, element: &str, patch: &PropsPatch, now_ms: u64) -> Result<Outcome, EngineError> {
        self.require_element(element)?;
        if patch.is_empty() {
            return Err(EngineError::EmptyPatch);
        }
        patch.validate().map_err(EngineError::Invalid)?;
        self.manual.entry(element.to_owned()).or_default().merge(patch);
        let detail = format!("{element}: held {}", patch.field_names().join(", "));
        self.record(now_ms, "manual", &detail);
        Ok(self.bump(Outcome::CHANGED))
    }

    /// Releases the manual overrides of one element, or of all elements.
    pub fn release_overrides(&mut self, element: Option<&str>, now_ms: u64) -> Result<Outcome, EngineError> {
        let released = match element {
            Some(id) => {
                self.require_element(id)?;
                self.manual.remove(id).is_some()
            }
            None => {
                let any = !self.manual.is_empty();
                self.manual.clear();
                any
            }
        };
        if !released {
            return Ok(Outcome::NONE);
        }
        let detail = format!("released {}", element.unwrap_or("all overrides"));
        self.record(now_ms, "manual", &detail);
        Ok(self.bump(Outcome::CHANGED))
    }

    /// Plays a one-shot effect on an element at the user's request.
    pub fn trigger_effect(&mut self, element: &str, effect: Effect, now_ms: u64) -> Result<Outcome, EngineError> {
        self.require_element(element)?;
        self.start_effect(element, effect);
        self.record(now_ms, "manual", &format!("{element}: {effect:?} effect"));
        Ok(self.bump(Outcome::CHANGED))
    }

    /// Adds a note to the event log, such as a save failure the streamer should know about.
    pub fn note(&mut self, source: &str, detail: &str, now_ms: u64) -> Outcome {
        self.record(now_ms, source, detail);
        self.bump(Outcome::CHANGED)
    }

    pub(crate) fn start_effect(&mut self, element: &str, effect: Effect) {
        let nonce = self.take_nonce();
        self.effects.insert(element.to_owned(), EffectState { effect, nonce });
    }

    /// A nonce for an effect or a speech cue; each is larger than the last.
    pub(crate) fn take_nonce(&mut self) -> u64 {
        let nonce = self.next_nonce;
        self.next_nonce += 1;
        nonce
    }

    fn require_element(&self, element: &str) -> Result<(), EngineError> {
        match self.project.element(element) {
            Some(_) => Ok(()),
            None => Err(EngineError::UnknownElement(element.to_owned())),
        }
    }

    pub(crate) fn record(&mut self, at_ms: u64, source: &str, detail: &str) {
        if self.events.len() == MAX_EVENTS {
            self.events.pop_front();
        }
        let event = EventRecord {
            at_ms,
            source: source.to_owned(),
            detail: detail.to_owned(),
        };
        self.events.push_back(event);
    }

    pub(crate) fn bump(&mut self, outcome: Outcome) -> Outcome {
        if outcome.changed {
            self.revision += 1;
        }
        outcome
    }
}
