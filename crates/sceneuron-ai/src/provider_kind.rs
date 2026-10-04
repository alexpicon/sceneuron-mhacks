// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The model providers Sceneuron can call, and the base URL and models each one starts with.

use serde::{Deserialize, Serialize};

use crate::retired::{ANTHROPIC, ANTHROPIC_REFUSED};

/// The model used for scene understanding unless the streamer picks another, on the default
/// provider.
pub const DEFAULT_MODEL: &str = GEMINI_MODELS.vision;
/// The model that plans setups unless the streamer picks another, on the default provider.
pub const DEFAULT_PLANNER_MODEL: &str = GEMINI_MODELS.planner;
/// The model that translates subtitles unless the streamer picks another, on the default
/// provider.
pub const DEFAULT_TRANSLATION_MODEL: &str = GEMINI_MODELS.translation;

/// Gemini 3.5 Flash-Lite looks, the fastest and cheapest at equal accuracy in the vision
/// benchmark, and translates; Gemini 3.8 Flash plans. From
/// <https://ai.google.dev/gemini-api/docs/models>.
const GEMINI_MODELS: DefaultModels = DefaultModels {
    vision: "gemini-3.5-flash-lite",
    planner: "gemini-3.8-flash",
    translation: "gemini-3.5-flash-lite",
};

/// Which API family the provider speaks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub enum ProviderKind {
    /// Google's Gemini API.
    #[default]
    #[serde(rename = "gemini")]
    Gemini,
    /// Any server with an OpenAI-style chat completions API, such as NVIDIA NIM, Ollama,
    /// llama.cpp, or vLLM.
    #[serde(rename = "openai-compatible")]
    OpenAiCompatible,
}

/// The model each role uses unless the streamer picks another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefaultModels {
    /// Scene understanding.
    pub vision: &'static str,
    /// The assistant's plans.
    pub planner: &'static str,
    /// Subtitle translations.
    pub translation: &'static str,
}

impl ProviderKind {
    const ALL: [ProviderKind; 2] = [ProviderKind::Gemini, ProviderKind::OpenAiCompatible];

    /// The base URL used when the streamer has not set one.
    pub fn default_base_url(self) -> &'static str {
        match self {
            ProviderKind::Gemini => "https://generativelanguage.googleapis.com",
            ProviderKind::OpenAiCompatible => "http://127.0.0.1:11434/v1",
        }
    }

    /// The name used in JSON and in the status display.
    pub fn name(self) -> &'static str {
        match self {
            ProviderKind::Gemini => "gemini",
            ProviderKind::OpenAiCompatible => "openai-compatible",
        }
    }

    /// The models this provider starts with, or `None` for an OpenAI-compatible server, whose
    /// models only the streamer knows.
    pub fn default_models(self) -> Option<DefaultModels> {
        match self {
            ProviderKind::Gemini => Some(GEMINI_MODELS),
            ProviderKind::OpenAiCompatible => None,
        }
    }
}

/// Reads a provider by [`ProviderKind::name`]; a provider Sceneuron no longer offers is refused
/// with a message that says so.
impl TryFrom<String> for ProviderKind {
    type Error = String;

    fn try_from(name: String) -> Result<ProviderKind, String> {
        if name == ANTHROPIC {
            return Err(ANTHROPIC_REFUSED.to_owned());
        }
        ProviderKind::ALL
            .into_iter()
            .find(|kind| kind.name() == name)
            .ok_or_else(|| format!("unknown provider {name:?}; use gemini or openai-compatible"))
    }
}
