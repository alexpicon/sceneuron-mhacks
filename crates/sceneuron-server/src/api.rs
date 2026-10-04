// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! JSON API handlers. Each returns the reply, or the error reply as `Err`.

use sceneuron_core::{Effect, EngineError, Flood, Presence, Project, PropsPatch};
use serde::Deserialize;

use crate::api_control::Answer;
use crate::body::read_json;
use crate::connection::Request;
use crate::http::Reply;
use crate::{Host, Hub};

type ApiResult = Result<Reply, Reply>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OverrideBody {
    element: String,
    props: PropsPatch,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseBody {
    /// `None` releases every element.
    #[serde(default)]
    element: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EffectBody {
    element: String,
    effect: Effect,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FloodBody {
    flood: Flood,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AutomationBody {
    enabled: bool,
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
struct SimulateBody {
    presence: SimulatedPresence,
}

/// Names the source to attach a filter to.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AttachBody {
    pub(crate) uuid: String,
}

/// Unknown elements, decks, and cards are missing resources; every other refusal is a bad request.
pub(crate) fn engine_error(error: EngineError) -> Reply {
    let status = match error {
        EngineError::UnknownElement(_) | EngineError::UnknownDeck(_) | EngineError::UnknownCard { .. } => 404,
        EngineError::EmptyPatch | EngineError::Invalid(_) | EngineError::MissingCard => 400,
    };
    Reply::error(status, &error.to_string())
}

pub(crate) fn state(hub: &Hub) -> Reply {
    Reply::json_text(200, hub.snapshot_json())
}

pub(crate) fn project(hub: &Hub) -> Reply {
    let project = hub.read(|engine| engine.project().clone());
    Reply::json(200, &project)
}

/// Why a project was not saved.
enum SaveRefusal {
    /// The project changed after the editor loaded it; carries the current revision.
    Conflict(u64),
    Invalid(String),
}

/// Saves a project. With `If-Match: <revision>` the save only happens when the project is
/// still at that revision, so an editor never overwrites changes it has not seen.
pub(crate) fn replace_project(hub: &Hub, request: &mut Request) -> ApiResult {
    let expected = match request.head.header_values("If-Match").next() {
        Some(value) => Some(
            value
                .trim()
                .trim_matches('"')
                .parse::<u64>()
                .map_err(|_| Reply::error(400, "If-Match must be a project revision number"))?,
        ),
        None => None,
    };
    let project: Project = read_json(request)?;
    let saved = hub.try_update(|engine, now| {
        let current = engine.project_revision();
        if expected.is_some_and(|expected| expected != current) {
            return Err(SaveRefusal::Conflict(current));
        }
        engine
            .replace_project(project, now)
            .map_err(|error| SaveRefusal::Invalid(error.to_string()))
    });
    match saved {
        Ok(_) => Ok(state(hub)),
        Err(SaveRefusal::Invalid(message)) => Err(Reply::error(400, &message)),
        Err(SaveRefusal::Conflict(current)) => {
            let message = "the project changed in another editor or by the assistant; reload it and try again";
            Err(Reply::json(
                409,
                &serde_json::json!({ "error": message, "project_revision": current }),
            ))
        }
    }
}

pub(crate) fn set_override(hub: &Hub, request: &mut Request) -> ApiResult {
    let body: OverrideBody = read_json(request)?;
    hub.try_update(|engine, now| engine.set_override(&body.element, &body.props, now))
        .map_err(engine_error)?;
    Ok(state(hub))
}

pub(crate) fn release(hub: &Hub, request: &mut Request, answer: Answer) -> ApiResult {
    let body: ReleaseBody = read_json(request)?;
    hub.try_update(|engine, now| engine.release_overrides(body.element.as_deref(), now))
        .map_err(engine_error)?;
    Ok(answer.reply(hub))
}

pub(crate) fn effect(hub: &Hub, request: &mut Request, answer: Answer) -> ApiResult {
    let body: EffectBody = read_json(request)?;
    hub.try_update(|engine, now| engine.trigger_effect(&body.element, body.effect, now))
        .map_err(engine_error)?;
    Ok(answer.reply(hub))
}

/// Plays a reaction preset on the overlay at once, whatever the cooldown that rules and the camera
/// wait for.
pub(crate) fn flood(hub: &Hub, request: &mut Request, answer: Answer) -> ApiResult {
    let body: FloodBody = read_json(request)?;
    hub.update(|engine, now| engine.trigger_flood(body.flood, now));
    Ok(answer.reply(hub))
}

pub(crate) fn automation(hub: &Hub, request: &mut Request, answer: Answer) -> ApiResult {
    let body: AutomationBody = read_json(request)?;
    hub.update(|engine, now| engine.set_automation(body.enabled, now));
    Ok(answer.reply(hub))
}

pub(crate) fn simulate(hub: &Hub, request: &mut Request) -> ApiResult {
    let body: SimulateBody = read_json(request)?;
    let presence = match body.presence {
        SimulatedPresence::Unknown => Presence::Unknown,
        SimulatedPresence::Present => Presence::Present,
        SimulatedPresence::Away => Presence::Away,
    };
    hub.update(|engine, now| engine.simulate_presence(presence, now));
    Ok(state(hub))
}

pub(crate) fn sources(host: &dyn Host) -> Reply {
    Reply::json(200, &host.list_sources())
}

pub(crate) fn attach(host: &dyn Host, request: &mut Request) -> ApiResult {
    let body: AttachBody = read_json(request)?;
    host.attach_filter(&body.uuid)
        .map_err(|message| Reply::error(400, &message))?;
    Ok(Reply::json(200, &serde_json::json!({ "ok": true })))
}

pub(crate) fn add_overlay(host: &dyn Host) -> ApiResult {
    let name = host.add_overlay().map_err(|message| Reply::error(400, &message))?;
    Ok(Reply::json(200, &serde_json::json!({ "name": name })))
}
