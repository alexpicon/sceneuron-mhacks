// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The AI assistant's planner: the request that asks a model to turn the streamer's words into a
//! typed edit plan, and the reading of its answer.

use std::fmt::Write as _;
use std::time::Duration;

use sceneuron_core::{EditPlan, Project};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::plan_schema::plan_schema;
use crate::provider::{AiError, ModelRequest};
use crate::settings::AiSettings;

/// Instructions that teach the model Sceneuron's project model; identical on every call so the
/// provider can cache them.
pub const PLANNER_PROMPT: &str = include_str!("prompts/planner.txt");
/// Room for long thinking plus a plan of up to 40 operations.
const MAX_TOKENS: u32 = 16_000;
const TIMEOUT: Duration = Duration::from_secs(120);
const MAX_SUMMARY_CHARS: usize = 600;
/// Deck fields a plan must give although the core fills them in when they are missing:
/// `update_deck` replaces the whole deck, so a delay or camera switch left out would silently
/// fall back to 0 and off.
const DECK_FIELDS: [&str; 2] = ["auto_hide_ms", "camera"];

/// What the streamer asked the assistant for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanMode {
    /// Apply a requested change to the current project.
    #[default]
    Edit,
    /// The user describes their stream, meetings, or classes; propose a complete starter set of
    /// conditions, elements, rules, and card decks.
    Setup,
}

/// Builds the planner request.
///
/// `jpeg` is the current camera picture, when the streamer chose to include it. With
/// `previous_error`, the request also carries the answer that was refused, as the model gave it,
/// and the reason, and asks for a corrected complete plan.
pub fn plan_request(
    project: &Project,
    request: &str,
    mode: PlanMode,
    jpeg: Option<Vec<u8>>,
    previous_error: Option<(&Value, &str)>,
    settings: &AiSettings,
) -> ModelRequest {
    let mut text = String::new();
    text.push_str(match mode {
        PlanMode::Edit => "Mode: edit. Change the project as the streamer asks, with the smallest set of operations.",
        PlanMode::Setup => {
            "Mode: setup. The user describes their stream, meetings, or classes. Propose a complete starter set of \
             2 to 5 useful conditions with matching elements and rules, and the card decks that fit, keeping useful \
             existing items."
        }
    });
    let canvas = project.canvas;
    let _ = write!(
        text,
        "\n\nCanvas: {}x{} pixels.\n\nCurrent project:\n",
        canvas.width, canvas.height
    );
    text.push_str(&compact_project(project));
    if jpeg.is_some() {
        text.push_str(
            "\n\nThe attached image is the streamer's camera right now. Use it to keep overlays away from the \
             person and to suit the scene.",
        );
    }
    let _ = write!(
        text,
        "\n\nThe streamer's request:\n<request>\n{}\n</request>",
        request.trim()
    );
    if let Some((rejected, error)) = previous_error {
        let _ = write!(
            text,
            "\n\nYour previous plan for this request failed validation, so nothing was applied.\n\
             Rejected plan:\n{rejected}\nValidation error: {error}\n\
             Return a corrected, complete plan for the same request, starting again from the current project above."
        );
    }
    ModelRequest {
        model: settings.planner_model.clone(),
        effort: settings.planner_effort.clone(),
        system: PLANNER_PROMPT.to_owned(),
        text,
        images: jpeg.into_iter().collect(),
        schema: plan_schema(),
        schema_name: "edit_plan".to_owned(),
        max_tokens: MAX_TOKENS,
        timeout: TIMEOUT,
    }
}

/// Reads the model's answer into an edit plan. The plan is not applied or validated here.
///
/// When an endpoint refuses the plan schema as structured output, it travels in the
/// instructions, where nothing holds the model to it, so an `add_deck` or `update_deck` that
/// leaves out `auto_hide_ms` or `camera` is refused here instead of taking the core's defaults.
pub fn parse_plan(json: &Value) -> Result<EditPlan, AiError> {
    let mut plan: EditPlan = serde_json::from_value(json.clone())
        .map_err(|error| AiError::Invalid(format!("the plan has the wrong shape: {error}")))?;
    require_deck_fields(json)?;
    let summary: Vec<&str> = plan.summary.split_whitespace().collect();
    plan.summary = summary.join(" ").chars().take(MAX_SUMMARY_CHARS).collect();
    Ok(plan)
}

/// Refuses an `add_deck` or `update_deck` whose deck leaves out one of [`DECK_FIELDS`]. Runs
/// after serde accepted the answer, so every deck is an object with a string id.
fn require_deck_fields(json: &Value) -> Result<(), AiError> {
    let operations = json["operations"].as_array().into_iter().flatten();
    for (index, operation) in operations.enumerate() {
        let op = operation["op"].as_str().unwrap_or_default();
        if !matches!(op, "add_deck" | "update_deck") {
            continue;
        }
        let deck = &operation["deck"];
        if let Some(field) = DECK_FIELDS.iter().find(|field| deck.get(field).is_none()) {
            let id = deck["id"].as_str().unwrap_or_default();
            let number = index + 1;
            let message = format!("operation {number}: {op} {id:?} must give {field}; anything left out is reset");
            return Err(AiError::Invalid(message));
        }
    }
    Ok(())
}

/// The parts of the project a plan can change, as compact JSON.
pub(crate) fn compact_project(project: &Project) -> String {
    json!({
        "canvas": project.canvas,
        "elements": project.elements,
        "conditions": project.conditions,
        "rules": project.rules,
        "decks": project.decks,
    })
    .to_string()
}
