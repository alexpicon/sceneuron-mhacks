// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Partial changes to the AI settings, as the editor sends them.

use std::fmt;

use serde::{Deserialize, Deserializer};

use crate::provider::AiError;
use crate::provider_kind::ProviderKind;
use crate::settings::{AiSettings, NamedModels};
use crate::subtitle_settings::SubtitleSettingsUpdate;
use crate::voice_settings::VoiceSettingsUpdate;

/// A partial change; unset fields keep their value.
///
/// `api_key: ""`, `jev_api_key: ""`, and `elevenlabs_api_key: ""` clear a key, and a `null`
/// price removes that override. `voice` and `subtitles` change only the settings they name.
#[derive(Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiSettingsUpdate {
    pub enabled: Option<bool>,
    pub consent: Option<bool>,
    pub provider: Option<ProviderKind>,
    pub base_url: Option<String>,
    pub vision_model: Option<String>,
    pub planner_model: Option<String>,
    pub vision_effort: Option<String>,
    pub planner_effort: Option<String>,
    pub api_key: Option<String>,
    pub image_width: Option<u32>,
    #[serde(default, deserialize_with = "present")]
    pub input_price_per_mtok: Option<Option<f64>>,
    #[serde(default, deserialize_with = "present")]
    pub output_price_per_mtok: Option<Option<f64>>,
    pub jev_api_key: Option<String>,
    pub jev_base_url: Option<String>,
    pub jev_model: Option<String>,
    pub elevenlabs_api_key: Option<String>,
    pub voice: Option<VoiceSettingsUpdate>,
    pub subtitles: Option<SubtitleSettingsUpdate>,
}

impl fmt::Debug for AiSettingsUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let shown = |key: &Option<String>| key.as_ref().map(|key| if key.is_empty() { "" } else { "<set>" });
        let (key, jev_key) = (shown(&self.api_key), shown(&self.jev_api_key));
        let elevenlabs_key = shown(&self.elevenlabs_api_key);
        f.debug_struct("AiSettingsUpdate")
            .field("enabled", &self.enabled)
            .field("consent", &self.consent)
            .field("provider", &self.provider)
            .field("base_url", &self.base_url)
            .field("vision_model", &self.vision_model)
            .field("planner_model", &self.planner_model)
            .field("vision_effort", &self.vision_effort)
            .field("planner_effort", &self.planner_effort)
            .field("api_key", &key)
            .field("image_width", &self.image_width)
            .field("input_price_per_mtok", &self.input_price_per_mtok)
            .field("output_price_per_mtok", &self.output_price_per_mtok)
            .field("jev_api_key", &jev_key)
            .field("jev_base_url", &self.jev_base_url)
            .field("jev_model", &self.jev_model)
            .field("elevenlabs_api_key", &elevenlabs_key)
            .field("voice", &self.voice)
            .field("subtitles", &self.subtitles)
            .finish()
    }
}

/// Distinguishes a field set to `null` (`Some(None)`) from a missing one (`None`).
fn present<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Option<f64>>, D::Error> {
    Option::<f64>::deserialize(deserializer).map(Some)
}

impl AiSettings {
    /// Returns these settings with `update` applied, or why the result would be invalid.
    ///
    /// Changing the provider without a base URL switches to that provider's default URL, and
    /// every model the update does not name to that provider's default for its role, when the
    /// provider has defaults. It also removes the saved key and price overrides, which belong to
    /// the old provider, unless the update sets them. Withdrawing consent also turns scene
    /// understanding off.
    pub fn updated(&self, update: &AiSettingsUpdate) -> Result<AiSettings, AiError> {
        let mut next = self.clone();
        if let Some(provider) = update.provider
            && provider != self.provider
        {
            if update.base_url.is_none() {
                next.base_url = provider.default_base_url().to_owned();
            }
            // One provider's key must never reach another, and its prices would misprice the
            // new provider's calls; the update's own key and prices are applied below.
            next.api_key.clear();
            next.input_price_per_mtok = None;
            next.output_price_per_mtok = None;
            next.provider = provider;
            next.default_unnamed_models(NamedModels {
                vision: update.vision_model.is_some(),
                planner: update.planner_model.is_some(),
                translation: update
                    .subtitles
                    .as_ref()
                    .is_some_and(|subtitles| subtitles.translation_model.is_some()),
            });
        }
        if let Some(url) = &update.base_url {
            next.base_url = url.trim().trim_end_matches('/').to_owned();
        }
        set_text(&mut next.vision_model, &update.vision_model);
        set_text(&mut next.planner_model, &update.planner_model);
        set_text(&mut next.vision_effort, &update.vision_effort);
        set_text(&mut next.planner_effort, &update.planner_effort);
        set_text(&mut next.api_key, &update.api_key);
        set(&mut next.image_width, update.image_width);
        set(&mut next.input_price_per_mtok, update.input_price_per_mtok);
        set(&mut next.output_price_per_mtok, update.output_price_per_mtok);
        set_text(&mut next.jev_api_key, &update.jev_api_key);
        if let Some(url) = &update.jev_base_url {
            next.jev_base_url = url.trim().trim_end_matches('/').to_owned();
        }
        set_text(&mut next.jev_model, &update.jev_model);
        set_text(&mut next.elevenlabs_api_key, &update.elevenlabs_api_key);
        if let Some(voice) = &update.voice {
            next.voice = next.voice.updated(voice);
        }
        if let Some(subtitles) = &update.subtitles {
            next.subtitles = next.subtitles.updated(subtitles);
        }
        set(&mut next.consent, update.consent);
        if update.consent == Some(false) && update.enabled.is_none() {
            next.enabled = false;
        }
        set(&mut next.enabled, update.enabled);
        next.validate()?;
        Ok(next)
    }
}

fn set<T>(target: &mut T, value: Option<T>) {
    if let Some(value) = value {
        *target = value;
    }
}

fn set_text(target: &mut String, value: &Option<String>) {
    if let Some(value) = value {
        value.trim().clone_into(target);
    }
}
