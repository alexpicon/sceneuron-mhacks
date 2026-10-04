// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What the server needs from the AI assistant: its settings, edit plans, on-demand looks, and
//! the audio of spoken cards.

use serde::{Deserialize, Serialize};

use sceneuron_core::{EditPlan, Project};

const NOT_AVAILABLE: &str = "AI features are not available in this build";

/// A refusal from the assistant, answered with this HTTP status and message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssistantError {
    pub status: u16,
    pub message: String,
}

impl AssistantError {
    pub fn new(status: u16, message: impl Into<String>) -> AssistantError {
        AssistantError {
            status,
            message: message.into(),
        }
    }
}

/// Whether the streamer asks for a change or describes a whole stream to set up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanMode {
    #[default]
    Edit,
    Setup,
}

/// The editor's request for an edit plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanRequest {
    /// The streamer's words.
    pub request: String,
    #[serde(default)]
    pub mode: PlanMode,
    /// Whether the planner may see the latest camera image.
    #[serde(default)]
    pub use_camera: bool,
}

/// A plan and the project it produces, already validated against the project it was made for.
#[derive(Clone, Debug, PartialEq)]
pub struct ProposedPlan {
    pub plan: EditPlan,
    pub project: Project,
    pub model: String,
    pub cost_usd: f64,
}

/// The generated audio of a spoken card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Speech {
    pub audio: Vec<u8>,
    /// Such as `audio/mpeg`.
    pub content_type: &'static str,
}

/// AI services behind the editor's assistant and AI settings.
///
/// The server calls these from its request thread, outside every hub lock; `plan` may take
/// a minute or more.
pub trait Assistant: Send + Sync {
    /// The AI settings as JSON for the editor; never includes the API key.
    fn settings(&self) -> serde_json::Value;

    /// Applies a partial settings update and returns the new settings.
    fn update_settings(&self, update: serde_json::Value) -> Result<serde_json::Value, AssistantError>;

    /// Proposes an edit plan for `project`.
    fn plan(&self, project: &Project, request: &PlanRequest) -> Result<ProposedPlan, AssistantError>;

    /// Asks the vision model to look at the camera as soon as the budget allows.
    fn analyze_now(&self) -> Result<(), AssistantError>;

    /// The audio of a spoken card by the id in the snapshot's speech cue, once it was
    /// generated. `id` is 64 lowercase hexadecimal digits.
    fn speech(&self, _id: &str) -> Option<Speech> {
        None
    }
}

/// The assistant of a build without AI: settings report it as unavailable.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullAssistant;

impl Assistant for NullAssistant {
    fn settings(&self) -> serde_json::Value {
        serde_json::json!({ "available": false })
    }

    fn update_settings(&self, _update: serde_json::Value) -> Result<serde_json::Value, AssistantError> {
        Err(AssistantError::new(400, NOT_AVAILABLE))
    }

    fn plan(&self, _project: &Project, _request: &PlanRequest) -> Result<ProposedPlan, AssistantError> {
        Err(AssistantError::new(400, NOT_AVAILABLE))
    }

    fn analyze_now(&self) -> Result<(), AssistantError> {
        Err(AssistantError::new(400, NOT_AVAILABLE))
    }
}
