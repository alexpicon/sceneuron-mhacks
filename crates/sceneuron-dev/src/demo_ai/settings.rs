// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The simulated assistant's AI settings, kept in memory and checked like the real ones.
//!
//! No key is ever stored: a key that the editor sends only leaves its last four characters as a
//! hint, and the demo starts with a fake key so the editor shows the "key saved" state.

use sceneuron_ai::ProviderKind;
use sceneuron_server::AssistantError;
use serde_json::{Value, json};

const EFFORTS: [&str; 3] = ["low", "medium", "high"];
const PROVIDERS: [&str; 2] = ["gemini", "openai-compatible"];
const MAX_TEXT_CHARS: usize = 500;

/// Settings as the editor sees them through `GET /api/ai`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DemoSettings {
    pub(crate) enabled: bool,
    pub(crate) consent: bool,
    pub(crate) provider: String,
    pub(crate) base_url: String,
    pub(crate) vision_model: String,
    pub(crate) planner_model: String,
    pub(crate) vision_effort: String,
    pub(crate) planner_effort: String,
    /// Last characters of the saved key; `None` when no key is saved.
    pub(crate) key_hint: Option<String>,
    pub(crate) image_width: u64,
    pub(crate) input_price_per_mtok: f64,
    pub(crate) output_price_per_mtok: f64,
}

impl Default for DemoSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            consent: false,
            provider: ProviderKind::default().name().to_owned(),
            base_url: ProviderKind::default().default_base_url().to_owned(),
            vision_model: "simulated-vision".to_owned(),
            planner_model: "simulated-planner".to_owned(),
            vision_effort: "low".to_owned(),
            planner_effort: "medium".to_owned(),
            key_hint: Some("demo".to_owned()),
            image_width: 768,
            input_price_per_mtok: 1.0,
            output_price_per_mtok: 5.0,
        }
    }
}

impl DemoSettings {
    /// The JSON the editor reads; `simulated` tells it that nothing is sent anywhere.
    pub(crate) fn to_json(&self) -> Value {
        json!({
            "available": true,
            "simulated": true,
            "enabled": self.enabled,
            "consent": self.consent,
            "provider": self.provider,
            "base_url": self.base_url,
            "vision_model": self.vision_model,
            "planner_model": self.planner_model,
            "vision_effort": self.vision_effort,
            "planner_effort": self.planner_effort,
            "has_key": self.key_hint.is_some(),
            "key_hint": self.key_hint.clone().unwrap_or_default(),
            "image_width": self.image_width,
            "input_price_per_mtok": self.input_price_per_mtok,
            "output_price_per_mtok": self.output_price_per_mtok,
            "price_known": true,
        })
    }

    /// Applies every field of `update`, or none of them when any field is refused.
    ///
    /// As in the real settings, a new provider without a base URL gets its own. Withdrawing
    /// consent also stops watching, and watching cannot start without consent.
    pub(crate) fn update(&mut self, update: &Value) -> Result<(), AssistantError> {
        let fields = update
            .as_object()
            .ok_or_else(|| refuse("the settings update must be a JSON object"))?;
        let mut next = self.clone();
        for (key, value) in fields {
            next.set(key, value)?;
        }
        if next.provider != self.provider
            && !fields.contains_key("base_url")
            && let Ok(provider) = serde_json::from_value::<ProviderKind>(json!(next.provider))
        {
            provider.default_base_url().clone_into(&mut next.base_url);
        }
        if fields.get("consent") == Some(&Value::Bool(false)) && !fields.contains_key("enabled") {
            next.enabled = false;
        }
        if next.enabled && !next.consent {
            return Err(refuse(
                "confirm consent to send camera images before turning on scene understanding",
            ));
        }
        *self = next;
        Ok(())
    }

    fn set(&mut self, key: &str, value: &Value) -> Result<(), AssistantError> {
        match key {
            "enabled" => self.enabled = boolean(key, value)?,
            "consent" => self.consent = boolean(key, value)?,
            "provider" => self.provider = choice(key, value, &PROVIDERS)?,
            "base_url" => {
                let url = text(key, value)?;
                if !(url.starts_with("http://") || url.starts_with("https://")) {
                    return Err(refuse("base_url must start with http:// or https://"));
                }
                self.base_url = url;
            }
            "vision_model" => self.vision_model = text(key, value)?,
            "planner_model" => self.planner_model = text(key, value)?,
            "vision_effort" => self.vision_effort = choice(key, value, &EFFORTS)?,
            "planner_effort" => self.planner_effort = choice(key, value, &EFFORTS)?,
            "api_key" => self.key_hint = key_hint(value)?,
            "image_width" => {
                self.image_width = value
                    .as_u64()
                    .filter(|width| (256..=2048).contains(width))
                    .ok_or_else(|| refuse("image_width must be a whole number from 256 to 2048"))?;
            }
            "input_price_per_mtok" => self.input_price_per_mtok = price(key, value)?,
            "output_price_per_mtok" => self.output_price_per_mtok = price(key, value)?,
            _ => return Err(refuse(&format!("unknown setting {key:?}"))),
        }
        Ok(())
    }
}

fn refuse(message: &str) -> AssistantError {
    AssistantError::new(400, message)
}

fn boolean(key: &str, value: &Value) -> Result<bool, AssistantError> {
    value
        .as_bool()
        .ok_or_else(|| refuse(&format!("{key} must be true or false")))
}

fn text(key: &str, value: &Value) -> Result<String, AssistantError> {
    let text = value.as_str().map(str::trim).unwrap_or_default();
    let count = text.chars().count();
    if (1..=MAX_TEXT_CHARS).contains(&count) && !text.chars().any(char::is_control) {
        Ok(text.to_owned())
    } else {
        Err(refuse(&format!(
            "{key} must be 1 to {MAX_TEXT_CHARS} characters on one line"
        )))
    }
}

fn choice(key: &str, value: &Value, allowed: &[&str]) -> Result<String, AssistantError> {
    let text = value.as_str().unwrap_or_default();
    if allowed.contains(&text) {
        Ok(text.to_owned())
    } else {
        Err(refuse(&format!("{key} must be one of {}", allowed.join(", "))))
    }
}

fn price(key: &str, value: &Value) -> Result<f64, AssistantError> {
    value
        .as_f64()
        .filter(|price| price.is_finite() && (0.0..=1000.0).contains(price))
        .ok_or_else(|| refuse(&format!("{key} must be a number from 0 to 1000")))
}

/// An empty key removes the saved one; any other key is reduced to its last four characters.
fn key_hint(value: &Value) -> Result<Option<String>, AssistantError> {
    let key = value.as_str().ok_or_else(|| refuse("api_key must be text"))?.trim();
    if key.is_empty() {
        return Ok(None);
    }
    if key.chars().count() > MAX_TEXT_CHARS || key.chars().any(char::is_control) {
        return Err(refuse("api_key must be one line of at most 500 characters"));
    }
    let chars: Vec<char> = key.chars().collect();
    Ok(Some(chars[chars.len().saturating_sub(4)..].iter().collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_off_without_consent_and_with_a_fake_key() {
        let settings = DemoSettings::default().to_json();
        assert_eq!(settings["available"], true);
        assert_eq!(settings["simulated"], true);
        assert_eq!(
            (settings["enabled"].clone(), settings["consent"].clone()),
            (json!(false), json!(false))
        );
        assert_eq!(
            (settings["has_key"].clone(), settings["key_hint"].clone()),
            (json!(true), json!("demo"))
        );
        assert!(settings.get("api_key").is_none());
    }

    #[test]
    fn watching_needs_consent_and_withdrawing_consent_stops_it() {
        let mut settings = DemoSettings::default();
        let refused = settings.update(&json!({ "enabled": true })).unwrap_err();
        assert_eq!(refused.status, 400);
        assert!(!settings.enabled);
        settings.update(&json!({ "consent": true, "enabled": true })).unwrap();
        assert!(settings.enabled && settings.consent);
        settings.update(&json!({ "consent": false })).unwrap();
        assert!(!settings.enabled && !settings.consent);
    }

    #[test]
    fn keys_leave_only_a_hint_and_an_empty_key_removes_it() {
        let mut settings = DemoSettings::default();
        settings.update(&json!({ "api_key": "  sk-secret-abcd  " })).unwrap();
        let json = settings.to_json();
        assert_eq!(json["key_hint"], "abcd");
        assert!(!json.to_string().contains("secret"));
        settings.update(&json!({ "api_key": "" })).unwrap();
        assert_eq!(
            (settings.to_json()["has_key"].clone(), settings.key_hint.clone()),
            (json!(false), None)
        );
    }

    #[test]
    fn a_refused_field_changes_nothing() {
        let mut settings = DemoSettings::default();
        for update in [
            json!({ "vision_model": "a", "provider": "other" }),
            json!({ "vision_effort": "max" }),
            json!({ "base_url": "ftp://x" }),
            json!({ "image_width": 10 }),
            json!({ "input_price_per_mtok": -1 }),
            json!({ "vision_model": "" }),
            json!({ "api_key": 5 }),
            json!({ "surprise": true }),
            json!([1]),
        ] {
            assert!(settings.update(&update).is_err(), "{update}");
        }
        assert_eq!(settings, DemoSettings::default());
        settings
            .update(&json!({ "provider": "openai-compatible", "base_url": "http://localhost:11434/v1" }))
            .unwrap();
        assert_eq!(settings.provider, "openai-compatible");
    }
}
