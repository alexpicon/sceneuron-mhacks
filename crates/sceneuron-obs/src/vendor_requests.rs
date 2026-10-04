// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What each obs-websocket vendor request asks for. Request data is checked like the HTTP API's
//! bodies: unknown fields and values of the wrong type are refused, never ignored.

use std::ffi::CStr;

use sceneuron_core::{CardAction, Effect, Engine, EngineError, Outcome, Presence, PropsPatch};
use serde::Deserialize;
use serde::de::DeserializeOwned;

/// Every request the vendor registers with obs-websocket.
pub const REQUESTS: [&CStr; 11] = [
    c"GetState",
    c"SetOverride",
    c"ReleaseOverrides",
    c"TriggerEffect",
    c"SetAutomation",
    c"SimulatePresence",
    c"AnalyzeNow",
    c"RaiseCard",
    c"ToggleCard",
    c"LowerCard",
    c"StepCard",
];

/// One checked vendor request.
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    /// Changes nothing; the reply carries the state.
    GetState,
    /// Asks the vision model to look now.
    AnalyzeNow,
    Change(Change),
}

/// A request that changes the engine.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    SetOverride {
        element: String,
        props: Box<PropsPatch>,
    },
    /// `None` releases every element.
    ReleaseOverrides {
        element: Option<String>,
    },
    TriggerEffect {
        element: String,
        effect: Effect,
    },
    SetAutomation {
        enabled: bool,
    },
    SimulatePresence {
        presence: Presence,
    },
    /// `card` names the card to raise or toggle; lowering and stepping need none.
    Card {
        deck: String,
        card: Option<String>,
        action: CardAction,
    },
    LowerAllCards,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoData {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OverrideData {
    element: String,
    props: PropsPatch,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseData {
    #[serde(default)]
    element: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EffectData {
    element: String,
    effect: Effect,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AutomationData {
    enabled: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PresenceData {
    presence: SimulatedPresence,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SimulatedPresence {
    Unknown,
    Present,
    Away,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CardData {
    deck: String,
    card: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LowerData {
    /// `None` lowers every deck.
    #[serde(default)]
    deck: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StepData {
    deck: String,
    step: Step,
}

/// A step through a deck: 1 raises the next card and -1 the previous one.
#[derive(Clone, Copy, Deserialize)]
#[serde(try_from = "i64")]
struct Step(CardAction);

impl TryFrom<i64> for Step {
    type Error = String;

    fn try_from(step: i64) -> Result<Self, Self::Error> {
        match step {
            1 => Ok(Step(CardAction::Next)),
            -1 => Ok(Step(CardAction::Previous)),
            other => Err(format!("step must be 1 or -1, not {other}")),
        }
    }
}

/// Checks one request. `data` is the JSON text of its request data; empty text means none.
pub fn parse(name: &str, data: &str) -> Result<Request, String> {
    let data = if data.trim().is_empty() { "{}" } else { data };
    let change = match name {
        "GetState" => return read::<NoData>(data).map(|_| Request::GetState),
        "AnalyzeNow" => return read::<NoData>(data).map(|_| Request::AnalyzeNow),
        "SetOverride" => {
            let OverrideData { element, props } = read(data)?;
            Change::SetOverride {
                element,
                props: Box::new(props),
            }
        }
        "ReleaseOverrides" => Change::ReleaseOverrides {
            element: read::<ReleaseData>(data)?.element,
        },
        "TriggerEffect" => {
            let EffectData { element, effect } = read(data)?;
            Change::TriggerEffect { element, effect }
        }
        "SetAutomation" => Change::SetAutomation {
            enabled: read::<AutomationData>(data)?.enabled,
        },
        "SimulatePresence" => {
            let presence = match read::<PresenceData>(data)?.presence {
                SimulatedPresence::Unknown => Presence::Unknown,
                SimulatedPresence::Present => Presence::Present,
                SimulatedPresence::Away => Presence::Away,
            };
            Change::SimulatePresence { presence }
        }
        "RaiseCard" => card(read(data)?, CardAction::Raise),
        "ToggleCard" => card(read(data)?, CardAction::Toggle),
        "LowerCard" => match read::<LowerData>(data)?.deck {
            Some(deck) => Change::Card {
                deck,
                card: None,
                action: CardAction::Lower,
            },
            None => Change::LowerAllCards,
        },
        "StepCard" => {
            let StepData { deck, step } = read(data)?;
            Change::Card {
                deck,
                card: None,
                action: step.0,
            }
        }
        other => return Err(format!("unknown request {other}")),
    };
    Ok(Request::Change(change))
}

impl Change {
    /// Applies the change; the caller runs it as one hub update, so it publishes one revision.
    pub fn apply(&self, engine: &mut Engine, now_ms: u64) -> Result<Outcome, EngineError> {
        match self {
            Change::SetOverride { element, props } => engine.set_override(element, props, now_ms),
            Change::ReleaseOverrides { element } => engine.release_overrides(element.as_deref(), now_ms),
            Change::TriggerEffect { element, effect } => engine.trigger_effect(element, *effect, now_ms),
            Change::SetAutomation { enabled } => Ok(engine.set_automation(*enabled, now_ms)),
            Change::SimulatePresence { presence } => Ok(engine.simulate_presence(*presence, now_ms)),
            Change::Card { deck, card, action } => engine.card(deck, card.as_deref(), *action, now_ms),
            Change::LowerAllCards => Ok(engine.lower_all_cards(now_ms)),
        }
    }
}

fn card(CardData { deck, card }: CardData, action: CardAction) -> Change {
    Change::Card {
        deck,
        card: Some(card),
        action,
    }
}

fn read<T: DeserializeOwned>(data: &str) -> Result<T, String> {
    serde_json::from_str(data).map_err(|error| format!("invalid request data: {error}"))
}
