// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Sceneuron's platform-independent core: the overlay model and its themes, presence tracking, the
//! live description of what the camera sees, AI conditions, rules, manual overrides, card decks
//! with their packs, camera signals, fun reactions and their emoji floods and banners, voice
//! control, spoken cards, live subtitles, AI edit plans, project persistence, and the starter
//! project a fresh install begins with. Nothing here depends on OBS.

mod automation;
mod cards;
mod covers;
mod decks;
mod defaults;
mod engine;
mod migrate;
mod model;
mod on_air;
mod packs;
mod persist;
mod plan;
mod presence;
mod props;
mod reactions;
mod scene;
mod seen;
mod settings;
mod signal_context;
mod signals;
mod snapshot;
mod starter;
mod subtitles;
mod template;
mod theme;
mod types;
mod understanding;
mod validate;
mod voice;
mod votes;

pub use decks::{Card, CardAction, Deck, RaisedBy};
pub use defaults::default_project;
pub use engine::Engine;
pub use model::{
    Action, Canvas, Condition, ConditionEdge, Effect, Element, ElementKind, PresenceTrigger, Project, Rule,
    SCHEMA_VERSION, Trigger, ValidationError,
};
pub use on_air::OnAirView;
pub use packs::{Pack, pack_for_canvas, packs};
pub use persist::{Loaded, PersistError, load_or_default, load_project, save_project};
pub use plan::{EditOp, EditPlan, MAX_PLAN_OPERATIONS};
pub use presence::{Presence, PresenceTracker};
pub use props::{ElementProps, EnterAnimation, IdleAnimation, PropsPatch, TextAlign};
pub use reactions::{
    Camera, Cue, DEFAULT_FLOOD_COOLDOWN_MS, FACEPALM_HOLD_MS, Flood, FloodState, MAX_FLOOD_COOLDOWN_MS,
    MIN_FLOOD_COOLDOWN_MS, Preset, Reaction, Reactions, TONGUE_OUT, presets, reactions,
};
pub use scene::{ConditionState, SceneObservation, SceneView, Verdict};
pub use seen::{HandSide, MAX_SEEN_CHARS, MOUTH_LINGER_MS, SEEN_CHANGE_MS, SETTLE_FRAMES, SeenHand, Sighting};
pub use settings::{AiPolicy, DetectionSettings};
pub use signal_context::HandSignalsView;
pub use signals::{CardSignal, HandSign, SignalSummary};
pub use snapshot::{ConditionView, DeckView, ElementView, PresenceView, Snapshot};
pub use starter::{load_or_starter, starter_project};
pub use subtitles::{
    CONTEXT_SEGMENTS, ContextLine, DEFAULT_CLEAR_AFTER_MS, DEFAULT_LINES, FinalSegment, LABEL_SCALE, LATIN_LINE,
    LATIN_SCALE, MAX_LINE_CHARS, MAX_LINES, MAX_TARGETS, NativeScript, SubtitleBar, SubtitleStatus, SubtitlesView,
    TranslationJob, canonical_language_tag, constructed_language, constructed_languages, label_room, language_label,
    language_name, natural_languages, same_language, valid_language_tag, wrap,
};
pub use template::shows_caption;
pub use theme::Theme;
pub use types::{AiStatus, EffectState, EngineError, EventRecord, Outcome, VisionStatus};
pub use voice::{
    DEFAULT_WAKE_WORD, GuideEntry, GuideGroup, Heard, HeardView, Intent, RoomStep, SpeechCue, VOICE_REQUEST_TIMEOUT_MS,
    VoiceAction, VoiceGuide, VoiceMode, VoiceReply, VoiceRequest, VoiceStatus, VoiceStep, VoiceTask, VoiceView,
    interpret, normalize_speech, speakable,
};
pub use votes::{VOTE_WINDOW_MS, VoteBy, VoteView};
