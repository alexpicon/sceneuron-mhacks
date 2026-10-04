// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Control endpoints: packs, cards, element visibility, the control view, and pairing and
//! revoking the control token.

use std::time::Instant;

use sceneuron_core::{CardAction, EngineError, PropsPatch, pack_for_canvas, packs as all_packs};
use serde::Deserialize;
use serde_json::json;

use crate::Hub;
use crate::access::Access;
use crate::api::{self, engine_error};
use crate::body::read_json;
use crate::connection::Request;
use crate::control_access::PAIRING_LIFETIME;
use crate::head::Head;
use crate::http::Reply;
use crate::server::Context;

type ApiResult = Result<Reply, Reply>;

/// What a change answers with: the editor gets the full snapshot, and a control token only
/// the control view, which leaves out captions and the event log.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Answer {
    Snapshot,
    ControlView,
}

impl Answer {
    pub(crate) fn for_access(access: Access) -> Self {
        match access {
            Access::Control { .. } => Self::ControlView,
            Access::None | Access::Overlay | Access::Full => Self::Snapshot,
        }
    }

    pub(crate) fn reply(self, hub: &Hub) -> Reply {
        match self {
            Self::Snapshot => api::state(hub),
            Self::ControlView => state(hub),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CardBody {
    deck: String,
    /// The card to raise or toggle; other actions ignore it.
    #[serde(default)]
    card: Option<String>,
    action: CardAction,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ElementBody {
    element: String,
    action: ElementAction,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ElementAction {
    Show,
    Hide,
    Toggle,
    /// Releases every manual override of the element, so rules decide again.
    Release,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PairBody {
    code: String,
}

/// Every pack, scaled to the saved project's canvas.
pub(crate) fn packs(hub: &Hub) -> Reply {
    let canvas = hub.read(|engine| engine.project().canvas);
    let scaled: Vec<_> = all_packs()
        .iter()
        .filter_map(|pack| pack_for_canvas(&pack.id, &canvas))
        .collect();
    Reply::json(200, &scaled)
}

pub(crate) fn card(hub: &Hub, request: &mut Request, answer: Answer) -> ApiResult {
    let body: CardBody = read_json(request)?;
    hub.try_update(|engine, now| engine.card(&body.deck, body.card.as_deref(), body.action, now))
        .map_err(engine_error)?;
    Ok(answer.reply(hub))
}

/// Shows, hides, toggles, or releases an element. A toggle reads the visibility and holds
/// its opposite in one change, so no other change can come between them.
pub(crate) fn element(hub: &Hub, request: &mut Request, answer: Answer) -> ApiResult {
    let body: ElementBody = read_json(request)?;
    let id = body.element.as_str();
    hub.try_update(|engine, now| {
        let visible = match body.action {
            ElementAction::Release => return engine.release_overrides(Some(id), now),
            ElementAction::Show => true,
            ElementAction::Hide => false,
            ElementAction::Toggle => {
                let props = engine
                    .effective_props(id)
                    .ok_or_else(|| EngineError::UnknownElement(id.to_owned()))?;
                !props.visible
            }
        };
        let patch = PropsPatch {
            visible: Some(visible),
            ..PropsPatch::default()
        };
        engine.set_override(id, &patch, now)
    })
    .map_err(engine_error)?;
    Ok(answer.reply(hub))
}

pub(crate) fn state(hub: &Hub) -> Reply {
    Reply::json(200, &hub.control_view())
}

/// A new pairing code for the editor to show, replacing any earlier one, with how many codes
/// were traded so far: once [`pairings`] counts more, a control surface paired.
pub(crate) fn issue_code(context: &Context) -> Reply {
    let code = context.control.issue_code(Instant::now());
    // Counted after issuing: the old code no longer trades, and nobody has the new one yet.
    let pairings = context.control.pairings();
    Reply::json(
        200,
        &json!({ "code": code, "expires_in_s": PAIRING_LIFETIME.as_secs(), "pairings": pairings }),
    )
}

/// How many codes were traded for the control token, so the editor can say when one paired.
pub(crate) fn pairings(context: &Context) -> Reply {
    Reply::json(200, &json!({ "pairings": context.control.pairings() }))
}

/// Trades the pairing code for the control token.
///
/// Browsers send `Origin` with every POST a page makes, and `Sec-Fetch-Site` other than
/// `none` with every request a page causes, so web pages are refused before the code is even
/// read, and no answer carries CORS headers. A page therefore cannot guess codes, and the
/// token only reaches programs on this computer.
pub(crate) fn pair(context: &Context, request: &mut Request) -> ApiResult {
    if from_a_web_page(&request.head) {
        return Err(Reply::error(
            403,
            "pairing is only for apps on this computer, not web pages",
        ));
    }
    let body: PairBody = read_json(request)?;
    match context.control.pair(&body.code, Instant::now()) {
        Some(token) => Ok(Reply::json(200, &json!({ "token": token }))),
        None => Err(Reply::error(
            403,
            "that code is wrong or has expired; ask the editor for a new one",
        )),
    }
}

fn from_a_web_page(head: &Head) -> bool {
    head.header_values("Origin").next().is_some() || head.header_values("Sec-Fetch-Site").any(|site| site != "none")
}

/// Replaces the control token and drops the pairing code, so every paired control surface has
/// to pair again; event streams opened with the old token end.
pub(crate) fn revoke(context: &Context) -> ApiResult {
    let others = [context.token.as_str(), context.overlay_token.as_str()];
    context
        .control
        .revoke(&others, |token| context.host.save_control_token(token))
        .map_err(|error| {
            let message = format!("the control token was not replaced, because it could not be saved: {error}");
            Reply::error(500, &message)
        })?;
    Ok(Reply::json(200, &json!({ "ok": true })))
}
