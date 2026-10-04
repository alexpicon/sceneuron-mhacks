// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The hotkeys Sceneuron offers: their names, how OBS Settings > Hotkeys describes them, what a
//! press does, and what a sync must change to match the project.
//!
//! Three hotkeys are fixed. The rest follow the project: one per element, one per card, and one
//! per deck to lower it. Names are built from ids, which never contain dots, so names never
//! collide, and a rename keeps the hotkey and its binding and only changes its description.

use std::collections::BTreeMap;

use sceneuron_core::{CardAction, Engine, EngineError, Outcome, Project, PropsPatch};

/// What pressing a hotkey does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    ToggleAutomation,
    ReleaseOverrides,
    /// Voice control listens while the key is held; the host routes it to the voice input.
    PushToTalk,
    /// Holds the element shown when it is hidden, and hidden when it is shown.
    ToggleElement(String),
    /// Raises the card, or lowers it when it is the deck's raised card.
    ToggleCard {
        deck: String,
        card: String,
    },
    LowerDeck(String),
}

/// One hotkey: what OBS shows for it and what a press does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hotkey {
    pub description: String,
    pub action: Action,
}

/// What a sync changes so the registered hotkeys match the wanted ones.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SyncPlan {
    /// Registered hotkeys that are no longer wanted.
    pub remove: Vec<String>,
    /// Registered hotkeys whose description changed: the name and the new description.
    pub describe: Vec<(String, String)>,
    /// Wanted hotkeys that are not registered yet.
    pub add: Vec<(String, Hotkey)>,
}

/// Every hotkey for `project`, by name: the fixed ones and those that follow the project.
pub fn wanted(project: &Project) -> BTreeMap<String, Hotkey> {
    let mut wanted = BTreeMap::new();
    let mut add = |name: String, description: String, action: Action| {
        wanted.insert(name, Hotkey { description, action });
    };
    add(
        "sceneuron.automation".to_owned(),
        "Sceneuron: Pause or resume automatic reactions".to_owned(),
        Action::ToggleAutomation,
    );
    add(
        "sceneuron.release".to_owned(),
        "Sceneuron: Release manual overrides".to_owned(),
        Action::ReleaseOverrides,
    );
    add(
        "sceneuron.voice.talk".to_owned(),
        "Sceneuron: Push to talk".to_owned(),
        Action::PushToTalk,
    );
    for element in &project.elements {
        add(
            format!("sceneuron.element.{}", element.id),
            format!("Sceneuron: Show or hide {}", element.name),
            Action::ToggleElement(element.id.clone()),
        );
    }
    for deck in &project.decks {
        for card in &deck.cards {
            let action = Action::ToggleCard {
                deck: deck.id.clone(),
                card: card.id.clone(),
            };
            add(
                format!("sceneuron.card.{}.{}", deck.id, card.id),
                format!("Sceneuron: Raise or lower {}: {}", deck.name, card.name),
                action,
            );
        }
        add(
            format!("sceneuron.deck.{}.lower", deck.id),
            format!("Sceneuron: Lower {}", deck.name),
            Action::LowerDeck(deck.id.clone()),
        );
    }
    wanted
}

/// Compares the registered hotkeys, by name with their descriptions, with the wanted ones.
pub fn plan_sync(registered: &BTreeMap<String, String>, wanted: BTreeMap<String, Hotkey>) -> SyncPlan {
    let remove = registered
        .keys()
        .filter(|name| !wanted.contains_key(*name))
        .cloned()
        .collect();
    let mut plan = SyncPlan {
        remove,
        ..SyncPlan::default()
    };
    for (name, hotkey) in wanted {
        match registered.get(&name) {
            None => plan.add.push((name, hotkey)),
            Some(description) if *description != hotkey.description => {
                plan.describe.push((name, hotkey.description));
            }
            Some(_) => {}
        }
    }
    plan
}

impl Action {
    /// Applies one press; the caller runs it as one hub update, so it publishes one revision.
    /// Push to talk changes nothing here.
    pub fn apply(&self, engine: &mut Engine, now_ms: u64) -> Result<Outcome, EngineError> {
        match self {
            Action::ToggleAutomation => {
                let enabled = !engine.project().automation_enabled;
                Ok(engine.set_automation(enabled, now_ms))
            }
            Action::ReleaseOverrides => engine.release_overrides(None, now_ms),
            Action::PushToTalk => Ok(Outcome::default()),
            Action::ToggleElement(element) => {
                let visible = engine.effective_props(element).is_some_and(|props| props.visible);
                let patch = PropsPatch {
                    visible: Some(!visible),
                    ..PropsPatch::default()
                };
                engine.set_override(element, &patch, now_ms)
            }
            Action::ToggleCard { deck, card } => engine.card(deck, Some(card), CardAction::Toggle, now_ms),
            Action::LowerDeck(deck) => engine.card(deck, None, CardAction::Lower, now_ms),
        }
    }
}
