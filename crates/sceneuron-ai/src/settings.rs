// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! AI settings: which provider and models to use, the API keys, consent, price overrides, and
//! the voice and subtitle settings.
//!
//! These live in their own file (`ai.json`), apart from the project, because they hold a secret
//! and belong to the computer rather than to a layout.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::credentials::Credentials;
use crate::provider::AiError;
use crate::provider_kind::{DEFAULT_MODEL, DEFAULT_PLANNER_MODEL, ProviderKind};
use crate::settings_view::key_hint;
use crate::subtitle_settings::SubtitleSettings;
use crate::voice_settings::VoiceSettings;

/// TypeSafe's API, for quick judgments from on-device facts.
pub const DEFAULT_JEV_BASE_URL: &str = "https://api.typesafe.ai";
/// The Jev model asked unless the streamer picks another.
pub const DEFAULT_JEV_MODEL: &str = "jev-latest";
/// Largest camera image width the settings accept, in pixels.
pub const MAX_IMAGE_WIDTH: u32 = 1568;
/// Smallest camera image width the settings accept, in pixels.
pub const MIN_IMAGE_WIDTH: u32 = 256;
const MAX_MODEL_CHARS: usize = 200;
const MAX_KEY_CHARS: usize = 400;
const MAX_URL_CHARS: usize = 500;
const MAX_PRICE_PER_MTOK: f64 = 1000.0;
const CONSENT_FIRST: &str = "confirm that camera images may be sent to the provider first";

/// Everything Sceneuron needs to call a model.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AiSettings {
    /// Whether scene understanding may send camera images to the provider.
    pub enabled: bool,
    /// The streamer confirmed that camera images may leave the computer.
    pub consent: bool,
    pub provider: ProviderKind,
    pub base_url: String,
    pub vision_model: String,
    pub planner_model: String,
    /// Reasoning depth for scene understanding: `low`, `medium`, or `high`.
    pub vision_effort: String,
    /// Reasoning depth for the assistant: `low`, `medium`, or `high`.
    pub planner_effort: String,
    /// Secret; never shown, logged, or included in errors.
    pub api_key: String,
    /// Width in pixels of the camera images sent to the model.
    pub image_width: u32,
    /// Overrides the input price in US dollars per million tokens.
    pub input_price_per_mtok: Option<f64>,
    /// Overrides the output price in US dollars per million tokens.
    pub output_price_per_mtok: Option<f64>,
    /// Optional secret for TypeSafe's Jev; without it, quick judgments stay off.
    pub jev_api_key: String,
    pub jev_base_url: String,
    pub jev_model: String,
    /// Optional secret for ElevenLabs; without it, voice control and spoken cards stay off.
    pub elevenlabs_api_key: String,
    pub voice: VoiceSettings,
    pub subtitles: SubtitleSettings,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            consent: false,
            provider: ProviderKind::Gemini,
            base_url: ProviderKind::Gemini.default_base_url().to_owned(),
            vision_model: DEFAULT_MODEL.to_owned(),
            planner_model: DEFAULT_PLANNER_MODEL.to_owned(),
            vision_effort: "low".to_owned(),
            planner_effort: "medium".to_owned(),
            api_key: String::new(),
            image_width: 768,
            input_price_per_mtok: None,
            output_price_per_mtok: None,
            jev_api_key: String::new(),
            jev_base_url: DEFAULT_JEV_BASE_URL.to_owned(),
            jev_model: DEFAULT_JEV_MODEL.to_owned(),
            elevenlabs_api_key: String::new(),
            voice: VoiceSettings::default(),
            subtitles: SubtitleSettings::default(),
        }
    }
}

impl fmt::Debug for AiSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AiSettings")
            .field("enabled", &self.enabled)
            .field("consent", &self.consent)
            .field("provider", &self.provider)
            .field("base_url", &self.base_url)
            .field("vision_model", &self.vision_model)
            .field("planner_model", &self.planner_model)
            .field("vision_effort", &self.vision_effort)
            .field("planner_effort", &self.planner_effort)
            .field("api_key", &key_hint(&self.api_key))
            .field("image_width", &self.image_width)
            .field("input_price_per_mtok", &self.input_price_per_mtok)
            .field("output_price_per_mtok", &self.output_price_per_mtok)
            .field("jev_api_key", &key_hint(&self.jev_api_key))
            .field("jev_base_url", &self.jev_base_url)
            .field("jev_model", &self.jev_model)
            .field("elevenlabs_api_key", &key_hint(&self.elevenlabs_api_key))
            .field("voice", &self.voice)
            .field("subtitles", &self.subtitles)
            .finish()
    }
}

impl AiSettings {
    /// Checks every value; the error says what to fix.
    pub fn validate(&self) -> Result<(), AiError> {
        check_base_url("base_url", &self.base_url)?;
        check_base_url("jev_base_url", &self.jev_base_url)?;
        check_model("vision_model", &self.vision_model)?;
        check_model("jev_model", &self.jev_model)?;
        check_model("planner_model", &self.planner_model)?;
        check_effort("vision_effort", &self.vision_effort)?;
        check_effort("planner_effort", &self.planner_effort)?;
        check_key(&self.api_key)?;
        check_key(&self.jev_api_key)?;
        check_key(&self.elevenlabs_api_key)?;
        self.voice.validate()?;
        self.subtitles.validate()?;
        if !(MIN_IMAGE_WIDTH..=MAX_IMAGE_WIDTH).contains(&self.image_width) {
            let message = format!("image_width must be {MIN_IMAGE_WIDTH} to {MAX_IMAGE_WIDTH} pixels");
            return Err(AiError::Config(message));
        }
        check_price("input_price_per_mtok", self.input_price_per_mtok)?;
        check_price("output_price_per_mtok", self.output_price_per_mtok)?;
        if self.enabled && !self.consent {
            return Err(AiError::Config(CONSENT_FIRST.to_owned()));
        }
        Ok(())
    }

    /// These settings with keys from `credentials` wherever the editor set none; this is what
    /// the models are called with. The result is never saved.
    pub fn with_credentials(&self, credentials: &Credentials) -> AiSettings {
        let mut effective = self.clone();
        if effective.api_key.is_empty() {
            credentials
                .for_provider(self.provider)
                .clone_into(&mut effective.api_key);
        }
        if effective.jev_api_key.is_empty() {
            credentials.jev.clone_into(&mut effective.jev_api_key);
        }
        if effective.elevenlabs_api_key.is_empty() {
            credentials.elevenlabs.clone_into(&mut effective.elevenlabs_api_key);
        }
        effective
    }

    /// Gives every model role that `named` leaves out the provider's default model for it, so
    /// a provider is never paired with another provider's models. A provider without defaults,
    /// an OpenAI-compatible server, keeps the models as they are.
    pub(crate) fn default_unnamed_models(&mut self, named: NamedModels) {
        let Some(models) = self.provider.default_models() else {
            return;
        };
        if !named.vision {
            models.vision.clone_into(&mut self.vision_model);
        }
        if !named.planner {
            models.planner.clone_into(&mut self.planner_model);
        }
        if !named.translation {
            models.translation.clone_into(&mut self.subtitles.translation_model);
        }
    }
}

/// Which model roles a settings file or a settings update names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct NamedModels {
    pub vision: bool,
    pub planner: bool,
    pub translation: bool,
}

fn check_base_url(field: &str, url: &str) -> Result<(), AiError> {
    let invalid = || AiError::Config(format!("{field} must be an http or https URL with a host, got {url:?}"));
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(invalid)?;
    let authority = rest.split('/').next().unwrap_or_default();
    let host = authority.rsplit_once(':').map_or(authority, |(host, _port)| host);
    let forbidden = |c: char| c.is_whitespace() || c.is_control() || matches!(c, '@' | '?' | '#' | '\\');
    if host.is_empty() || url.chars().count() > MAX_URL_CHARS || url.chars().any(forbidden) {
        return Err(invalid());
    }
    Ok(())
}

pub(crate) fn check_model(field: &str, model: &str) -> Result<(), AiError> {
    let count = model.chars().count();
    if (1..=MAX_MODEL_CHARS).contains(&count) && !model.chars().any(|c| c.is_whitespace() || c.is_control()) {
        Ok(())
    } else {
        let message = format!("{field} must be a model name of 1 to {MAX_MODEL_CHARS} characters without spaces");
        Err(AiError::Config(message))
    }
}

fn check_effort(field: &str, effort: &str) -> Result<(), AiError> {
    if matches!(effort, "low" | "medium" | "high") {
        Ok(())
    } else {
        Err(AiError::Config(format!("{field} must be low, medium, or high")))
    }
}

fn check_key(key: &str) -> Result<(), AiError> {
    if key.chars().count() <= MAX_KEY_CHARS && !key.chars().any(|c| c.is_whitespace() || c.is_control()) {
        Ok(())
    } else {
        let message = format!("the API key must be at most {MAX_KEY_CHARS} characters without spaces");
        Err(AiError::Config(message))
    }
}

fn check_price(field: &str, price: Option<f64>) -> Result<(), AiError> {
    match price {
        Some(value) if !(value.is_finite() && (0.0..=MAX_PRICE_PER_MTOK).contains(&value)) => {
            let message = format!("{field} must be 0 to {MAX_PRICE_PER_MTOK} US dollars per million tokens");
            Err(AiError::Config(message))
        }
        _ => Ok(()),
    }
}
