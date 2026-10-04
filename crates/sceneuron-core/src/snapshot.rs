// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The serializable view of the overlay state sent to the editor, overlay pages, and API clients.

use serde::Serialize;

use crate::decks::RaisedBy;
use crate::engine::Engine;
use crate::model::{Canvas, ElementKind};
use crate::on_air::OnAirView;
use crate::presence::Presence;
use crate::props::ElementProps;
use crate::reactions::FloodState;
use crate::scene::SceneView;
use crate::signal_context::HandSignalsView;
use crate::subtitles::{SubtitleBar, SubtitlesView};
use crate::template::{shows_clock, shows_on_air, shows_subtitles};
use crate::theme::Theme;
use crate::types::{AiStatus, EffectState, EventRecord, VisionStatus};
use crate::voice::{SpeechCue, VoiceView};
use crate::votes::VoteView;

/// One element with its effective properties and where they come from.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ElementView {
    pub id: String,
    pub name: String,
    pub kind: ElementKind,
    pub props: ElementProps,
    /// Whether its text shows live subtitles, whose words change as they are said; the overlay
    /// sizes such text for as much as its box holds rather than for the words it shows.
    pub subtitles: bool,
    /// Whether its text shows OBS's scene and whether it is live (`{{on-air}}`).
    pub on_air: bool,
    /// Whether its text shows the time on air (`{{on-air-time}}`), which the overlay fills in and
    /// counts every second from the snapshot's `on_air`.
    pub clock: bool,
    /// What the overlay adds to text that shows subtitles: a label naming their language, the
    /// script they are written in, and Latin lines under them; `None` for other elements.
    pub subtitle_bar: Option<SubtitleBar>,
    /// Fields held by a manual override.
    pub overridden: Vec<&'static str>,
    /// Fields set by rules.
    pub automated: Vec<&'static str>,
    pub effect: Option<EffectState>,
}

/// Presence and the evidence behind it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PresenceView {
    pub state: Presence,
    pub since_ms: u64,
    pub last_score: Option<f64>,
    pub last_seen_ms: Option<u64>,
}

/// One condition with what the vision model last said about it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ConditionView {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub on: bool,
    /// Whether the model has judged it since the project loaded.
    pub judged: bool,
    pub since_ms: u64,
    pub confidence: f64,
    pub detail: String,
    pub reason: String,
}

/// One deck with the card it shows.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DeckView {
    pub id: String,
    pub name: String,
    pub element: String,
    /// Id of the raised card; `None` while the deck is lowered.
    pub raised: Option<String>,
    pub raised_by: Option<RaisedBy>,
    /// When the raised card went up, on the same clock as `now_ms`; themes mark the newest.
    pub raised_at_ms: Option<u64>,
    /// When the raised card lowers by itself, on the same clock as `now_ms`.
    pub hide_at_ms: Option<u64>,
    /// Whether the raised card shows its deck's cover instead of itself.
    pub covered: bool,
    /// The vote open for the deck, which its finger counts go to; `None` while none is.
    pub vote: Option<VoteView>,
    /// What the shared room says of the deck's open round, such as "Voted: 3 of 5"; `None` unless
    /// the room is joined with this deck and its round is open.
    pub summary: Option<String>,
}

/// Complete state for rendering and for the editor.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Snapshot {
    pub revision: u64,
    /// Revision of the saved project; editors send it back so a stale save is refused.
    pub project_revision: u64,
    pub now_ms: u64,
    pub canvas: Canvas,
    /// How the overlay draws every element, from the saved project.
    pub theme: Theme,
    pub automation_enabled: bool,
    pub presence: PresenceView,
    pub vision: VisionStatus,
    pub ai: AiStatus,
    pub scene: SceneView,
    /// OBS's program and whether it is on air.
    pub on_air: OnAirView,
    /// The live description `{{seen}}` shows: what on-device analysis found lately, in one short
    /// line; empty while it reports nothing.
    pub seen: String,
    pub conditions: Vec<ConditionView>,
    pub elements: Vec<ElementView>,
    /// The latest overlay-wide flood; pages play it when its nonce is new to them.
    pub flood: Option<FloodState>,
    pub decks: Vec<DeckView>,
    /// Where hand signals go while decks compete for the poses that are both a sign and a finger
    /// count; `None` while none do.
    pub hand_signals: Option<HandSignalsView>,
    pub voice: VoiceView,
    /// The latest card to speak; clients play its audio once it is set.
    pub speech: Option<SpeechCue>,
    /// Live subtitles, which are meant to be shown, unlike what voice control heard.
    pub subtitles: SubtitlesView,
    pub events: Vec<EventRecord>,
}

impl Engine {
    /// Builds the current view; `now_ms` lets clients compute ages.
    pub fn snapshot(&self, now_ms: u64) -> Snapshot {
        let elements = self
            .project
            .elements
            .iter()
            .map(|element| {
                let names = |layer: &std::collections::BTreeMap<String, crate::props::PropsPatch>| {
                    layer
                        .get(&element.id)
                        .map(|patch| patch.field_names())
                        .unwrap_or_default()
                };
                let mut props = self.layered_props(&element.id).unwrap_or_default();
                let subtitles = shows_subtitles(&props.text);
                let (on_air, clock) = (shows_on_air(&props.text), shows_clock(&props.text));
                let subtitle_bar = self.render_placeholders(&mut props);
                ElementView {
                    id: element.id.clone(),
                    name: element.name.clone(),
                    kind: element.kind,
                    props,
                    subtitles,
                    on_air,
                    clock,
                    subtitle_bar,
                    overridden: names(&self.manual),
                    automated: names(&self.auto),
                    effect: self.effects.get(&element.id).copied(),
                }
            })
            .collect();
        let conditions = self
            .project
            .conditions
            .iter()
            .map(|condition| {
                let state = self.conditions.get(&condition.id).cloned().unwrap_or_default();
                ConditionView {
                    id: condition.id.clone(),
                    name: condition.name.clone(),
                    enabled: condition.enabled,
                    on: state.on,
                    judged: state.judged,
                    since_ms: state.since_ms,
                    confidence: state.confidence,
                    detail: state.detail,
                    reason: state.reason,
                }
            })
            .collect();
        let decks = self
            .project
            .decks
            .iter()
            .map(|deck| {
                let raised = self.raised.get(&deck.id);
                DeckView {
                    id: deck.id.clone(),
                    name: deck.name.clone(),
                    element: deck.element.clone(),
                    raised: raised.map(|raised| raised.card.clone()),
                    raised_by: raised.map(|raised| raised.by),
                    raised_at_ms: raised.map(|raised| raised.at_ms),
                    hide_at_ms: raised.and_then(|raised| raised.hide_at_ms),
                    covered: raised.is_some() && self.covers.contains_key(&deck.id),
                    vote: self.vote(&deck.id, now_ms),
                    summary: self.room_summary_for(&deck.id),
                }
            })
            .collect();
        Snapshot {
            revision: self.revision(),
            project_revision: self.project_revision,
            now_ms,
            canvas: self.project.canvas,
            theme: self.project.theme,
            automation_enabled: self.project.automation_enabled,
            presence: PresenceView {
                state: self.presence.state(),
                since_ms: self.presence.since_ms(),
                last_score: self.presence.last_score(),
                last_seen_ms: self.presence.last_seen_ms(),
            },
            vision: self.vision.clone(),
            ai: self.ai.clone(),
            scene: self.scene.clone(),
            on_air: self.on_air_view(),
            seen: self.live.seen.clone(),
            conditions,
            elements,
            flood: self.flood,
            decks,
            hand_signals: self.hand_signals_view(now_ms),
            voice: self.voice_view(),
            speech: self.voice.speech.clone(),
            subtitles: self.subtitles_view(),
            events: self.events.iter().rev().cloned().collect(),
        }
    }
}
