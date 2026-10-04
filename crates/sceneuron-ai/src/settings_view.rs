// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The AI settings as the editor may see them: keys are replaced by short hints and where they
//! came from.

use serde::Serialize;

use crate::cost::prices_for;
use crate::credentials::Credentials;
use crate::provider_kind::ProviderKind;
use crate::settings::AiSettings;
use crate::subtitle_settings::SubtitleSettingsView;
use crate::voice_settings::VoiceSettingsView;

/// The settings without the key, plus what the editor needs to explain spend estimates.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AiSettingsView {
    /// Always true; lets the editor tell this build apart from one without AI support.
    pub available: bool,
    pub enabled: bool,
    pub consent: bool,
    pub provider: ProviderKind,
    pub base_url: String,
    pub vision_model: String,
    pub planner_model: String,
    pub vision_effort: String,
    pub planner_effort: String,
    pub image_width: u32,
    /// The streamer's input price override, if any.
    pub input_price_per_mtok: Option<f64>,
    /// The streamer's output price override, if any.
    pub output_price_per_mtok: Option<f64>,
    pub has_key: bool,
    /// `...` and the key's last four characters, or empty without a key.
    pub key_hint: String,
    /// `editor` when saved in the editor, `environment` when it comes from a `.env` file or the
    /// environment, or empty without a key.
    pub key_source: String,
    pub has_jev_key: bool,
    pub jev_key_hint: String,
    pub jev_key_source: String,
    pub jev_base_url: String,
    pub jev_model: String,
    pub has_elevenlabs_key: bool,
    pub elevenlabs_key_hint: String,
    pub elevenlabs_key_source: String,
    pub voice: VoiceSettingsView,
    pub subtitles: SubtitleSettingsView,
    /// Input price used for the vision model's spend estimate; 0 when unknown.
    pub effective_input_price_per_mtok: f64,
    /// Output price used for the vision model's spend estimate; 0 when unknown.
    pub effective_output_price_per_mtok: f64,
    /// Whether both models have a known or configured price. When false, their spend counts as
    /// zero and the hourly budget cannot limit it.
    pub price_known: bool,
}

impl AiSettings {
    /// What the editor may see: everything but the keys themselves.
    pub fn view(&self) -> AiSettingsView {
        self.view_with(&Credentials::default())
    }

    /// The view when `credentials` supply keys the editor did not set.
    pub fn view_with(&self, credentials: &Credentials) -> AiSettingsView {
        let effective = self.with_credentials(credentials);
        let source = |saved: &str, used: &str| match (saved.is_empty(), used.is_empty()) {
            (false, _) => "editor",
            (true, false) => "environment",
            (true, true) => "",
        };
        let vision = prices_for(self, &self.vision_model);
        let planner = prices_for(self, &self.planner_model);
        let translation = prices_for(self, &self.subtitles.translation_model);
        AiSettingsView {
            available: true,
            enabled: self.enabled,
            consent: self.consent,
            provider: self.provider,
            base_url: self.base_url.clone(),
            vision_model: self.vision_model.clone(),
            planner_model: self.planner_model.clone(),
            vision_effort: self.vision_effort.clone(),
            planner_effort: self.planner_effort.clone(),
            image_width: self.image_width,
            input_price_per_mtok: self.input_price_per_mtok,
            output_price_per_mtok: self.output_price_per_mtok,
            has_key: !effective.api_key.is_empty(),
            key_hint: key_hint(&effective.api_key),
            key_source: source(&self.api_key, &effective.api_key).to_owned(),
            has_jev_key: !effective.jev_api_key.is_empty(),
            jev_key_hint: key_hint(&effective.jev_api_key),
            jev_key_source: source(&self.jev_api_key, &effective.jev_api_key).to_owned(),
            jev_base_url: self.jev_base_url.clone(),
            jev_model: self.jev_model.clone(),
            has_elevenlabs_key: !effective.elevenlabs_api_key.is_empty(),
            elevenlabs_key_hint: key_hint(&effective.elevenlabs_api_key),
            elevenlabs_key_source: source(&self.elevenlabs_api_key, &effective.elevenlabs_api_key).to_owned(),
            voice: self.voice.view(),
            subtitles: SubtitleSettingsView {
                settings: self.subtitles.clone(),
                translation_price_known: translation.is_some(),
                translation_input_price_per_mtok: translation.map_or(0.0, |prices| prices.input),
                translation_output_price_per_mtok: translation.map_or(0.0, |prices| prices.output),
            },
            effective_input_price_per_mtok: vision.map_or(0.0, |prices| prices.input),
            effective_output_price_per_mtok: vision.map_or(0.0, |prices| prices.output),
            price_known: vision.is_some() && planner.is_some(),
        }
    }
}

/// Short keys show no characters at all, so the hint never reveals most of a secret.
pub(crate) fn key_hint(key: &str) -> String {
    let count = key.chars().count();
    match count {
        0 => String::new(),
        1..8 => "...".to_owned(),
        _ => format!("...{}", key.chars().skip(count - 4).collect::<String>()),
    }
}
