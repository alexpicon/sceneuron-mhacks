// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! AI endpoints: settings, edit plans from the assistant, and on-demand camera analysis.

use serde_json::json;

use crate::Hub;
use crate::assistant::{Assistant, AssistantError, PlanRequest};
use crate::body::read_json;
use crate::connection::Request;
use crate::http::Reply;

type ApiResult = Result<Reply, Reply>;

/// Longest request the assistant accepts, in characters.
const MAX_REQUEST_CHARS: usize = 2000;

pub(crate) fn settings(assistant: &dyn Assistant) -> Reply {
    Reply::json(200, &assistant.settings())
}

pub(crate) fn update_settings(assistant: &dyn Assistant, request: &mut Request) -> ApiResult {
    let update: serde_json::Value = read_json(request)?;
    if !update.is_object() {
        return Err(Reply::error(400, "the settings update must be a JSON object"));
    }
    let settings = assistant.update_settings(update).map_err(refusal)?;
    Ok(Reply::json(200, &settings))
}

/// Answers with the plan, the project it produces, and the project revision it was made
/// for, which the editor sends back when it saves so a plan never overwrites newer edits.
pub(crate) fn plan(hub: &Hub, assistant: &dyn Assistant, request: &mut Request) -> ApiResult {
    let body: PlanRequest = read_json(request)?;
    let length = body.request.trim().chars().count();
    if length == 0 || length > MAX_REQUEST_CHARS {
        let message = format!("describe the change in 1 to {MAX_REQUEST_CHARS} characters");
        return Err(Reply::error(400, &message));
    }
    let (project, base_revision) = hub.read(|engine| (engine.project().clone(), engine.project_revision()));
    let proposed = assistant.plan(&project, &body).map_err(refusal)?;
    Ok(Reply::json(
        200,
        &json!({
            "summary": proposed.plan.summary,
            "operations": proposed.plan.operations,
            "project": proposed.project,
            "base_revision": base_revision,
            "model": proposed.model,
            "cost_usd": proposed.cost_usd,
        }),
    ))
}

pub(crate) fn analyze(assistant: &dyn Assistant) -> ApiResult {
    assistant.analyze_now().map_err(refusal)?;
    Ok(Reply::json(202, &json!({ "ok": true })))
}

/// Keeps the assistant's status when it is an error status; anything else becomes 500.
fn refusal(error: AssistantError) -> Reply {
    let status = if (400..600).contains(&error.status) {
        error.status
    } else {
        500
    };
    Reply::error(status, &error.message)
}
