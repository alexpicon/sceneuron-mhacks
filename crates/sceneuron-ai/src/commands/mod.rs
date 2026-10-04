// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Voice commands for the AI: the request that asks the planner model what words addressed to
//! Sceneuron ask for, as typed actions from a catalog of everything the editor and the control
//! API can do, and the reading of its answer. Only the words and the project go out, as with the
//! assistant's plans, and never a camera image.

mod runner;
mod schema;

use std::fmt::Write as _;
use std::time::Duration;

use sceneuron_core::{EditPlan, Project, Theme, VoiceAction, presets};
use serde_json::{Value, json};

use crate::planner::{PLANNER_PROMPT, compact_project, parse_plan};
use crate::provider::{AiError, ModelRequest};
use crate::settings::AiSettings;
pub(crate) use runner::Runner;
pub use schema::command_schema;

/// What voice commands add to the planner's instructions, which teach the project and its edits.
pub const VOICE_PROMPT: &str = include_str!("../prompts/voice.txt");
/// The most actions one answer may carry.
pub const MAX_ACTIONS: usize = 10;
/// Room for brief thinking and a few actions, an edit among them.
const MAX_TOKENS: u32 = 8_000;
/// Someone waits for a spoken command, so it gets less time than a plan.
const TIMEOUT: Duration = Duration::from_secs(45);
/// A spoken command wants a quick answer more than deep reasoning.
const EFFORT: &str = "low";
const MAX_UNDERSTOOD_CHARS: usize = 300;

/// What the AI made of the words: what it understood, and the actions in order.
#[derive(Clone, Debug, PartialEq)]
pub struct CommandAnswer {
    pub understood: String,
    pub actions: Vec<VoiceAction>,
}

/// Builds the request for the words `said`, addressed to Sceneuron. With `previous_error`, it
/// also carries the answer that was refused, as the model gave it, and the reason, and asks for a
/// corrected answer.
pub fn command_request(
    project: &Project,
    said: &str,
    previous_error: Option<(&Value, &str)>,
    settings: &AiSettings,
) -> ModelRequest {
    let reactions: Vec<Value> = presets()
        .iter()
        .map(|preset| json!({"id": preset.flood, "name": preset.name, "banner": preset.banner}))
        .collect();
    let themes: Vec<&str> = Theme::ALL.iter().map(|theme| theme.id()).collect();
    let running = if project.automation_enabled {
        "runs"
    } else {
        "is paused"
    };
    let mut text = format!(
        "The streamer said, to Sceneuron:\n<words>\n{}\n</words>\n\nReactions: {}\nThemes: {}; now {}.\n\
         Automation {running}.\n\nCanvas: {}x{} pixels.\n\nCurrent project:\n{}",
        said.trim(),
        Value::from(reactions),
        themes.join(", "),
        project.theme.id(),
        project.canvas.width,
        project.canvas.height,
        compact_project(project),
    );
    if let Some((rejected, error)) = previous_error {
        let _ = write!(
            text,
            "\n\nYour previous answer for these words could not be used, so nothing was done.\n\
             Rejected answer:\n{rejected}\nReason: {error}\n\
             Return a corrected, complete answer for the same words, starting again from the current project above."
        );
    }
    ModelRequest {
        model: settings.planner_model.clone(),
        effort: EFFORT.to_owned(),
        system: format!("{PLANNER_PROMPT}\n\n{VOICE_PROMPT}"),
        text,
        images: Vec::new(),
        schema: command_schema(),
        schema_name: "voice_command".to_owned(),
        max_tokens: MAX_TOKENS,
        timeout: TIMEOUT,
    }
}

/// Reads the model's answer into actions. A project edit is read as a plan is, with the same
/// checks, but neither applied nor validated here.
pub fn parse_commands(json: &Value) -> Result<CommandAnswer, AiError> {
    let invalid = |message: String| AiError::Invalid(message);
    let understood = json["understood"]
        .as_str()
        .ok_or_else(|| invalid("the answer has no understood text".to_owned()))?;
    let understood: Vec<&str> = understood.split_whitespace().collect();
    let understood = understood.join(" ").chars().take(MAX_UNDERSTOOD_CHARS).collect();
    let listed = json["actions"]
        .as_array()
        .ok_or_else(|| invalid("the answer has no list of actions".to_owned()))?;
    if listed.len() > MAX_ACTIONS {
        return Err(invalid(format!("an answer may have at most {MAX_ACTIONS} actions")));
    }
    let actions = listed
        .iter()
        .enumerate()
        .map(|(index, action)| {
            parse_action(action).map_err(|error| invalid(format!("action {}: {}", index + 1, error.message())))
        })
        .collect::<Result<_, _>>()?;
    Ok(CommandAnswer { understood, actions })
}

fn parse_action(action: &Value) -> Result<VoiceAction, AiError> {
    if action["action"] == "edit_project" {
        let plan = json!({"summary": action["summary"], "operations": action["operations"]});
        let EditPlan { summary, operations } = parse_plan(&plan)?;
        return Ok(VoiceAction::EditProject { summary, operations });
    }
    serde_json::from_value(action.clone())
        .map_err(|error| AiError::Invalid(format!("the action has the wrong shape: {error}")))
}

/// Checks that the answer's project edits apply in turn to `project`; the error is the reason to
/// send back with it.
pub fn check_edits(answer: &CommandAnswer, project: &Project) -> Result<(), String> {
    let mut next = project.clone();
    for action in &answer.actions {
        if let VoiceAction::EditProject { summary, operations } = action {
            let plan = EditPlan {
                summary: summary.clone(),
                operations: operations.clone(),
            };
            next = plan.apply(&next).map_err(|error| error.0)?;
        }
    }
    Ok(())
}
