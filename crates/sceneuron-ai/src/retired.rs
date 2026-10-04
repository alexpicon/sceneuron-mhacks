// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Anthropic, a provider Sceneuron no longer offers: AI settings saved for it load as Gemini, and
//! an update that names it is refused.

use std::path::Path;

use serde_json::Value;

use crate::provider_kind::ProviderKind;

/// The name Anthropic was saved and sent under.
pub(crate) const ANTHROPIC: &str = "anthropic";
/// Why an update that names Anthropic is refused.
pub(crate) const ANTHROPIC_REFUSED: &str = "Anthropic is no longer offered; use gemini or openai-compatible";

/// Settings that belonged to Anthropic, as JSON pointers: its address, models, key, and prices.
/// Left out, they take Gemini's defaults; an Anthropic key in particular must never reach Google.
const ANTHROPIC_ONLY: [&str; 7] = [
    "/base_url",
    "/vision_model",
    "/planner_model",
    "/api_key",
    "/input_price_per_mtok",
    "/output_price_per_mtok",
    "/subtitles/translation_model",
];
/// Where a file names its models, as JSON pointers.
const MODELS: [&str; 3] = ["/vision_model", "/planner_model", "/subtitles/translation_model"];
/// How every Anthropic API key starts.
const ANTHROPIC_KEY_PREFIX: &str = "sk-ant-";
/// How every Anthropic model id starts.
const CLAUDE_MODEL_PREFIX: &str = "claude-";

/// Moves `stored`, the settings file at `path` as saved, from Anthropic to Gemini, and returns
/// the warning to show; a file on any other provider is left alone and gives `None`.
///
/// Consent to send camera images, and subtitle lines when they are translated, named Anthropic as
/// the destination, so it is withdrawn rather than carried over to Google.
pub(crate) fn leave_anthropic(stored: &mut Value, path: &Path) -> Option<String> {
    if !saved_for_anthropic(stored) {
        return None;
    }
    let mut withdrawn = Vec::new();
    if withdraw_consent(stored) {
        withdrawn.push("camera images");
    }
    let translated = stored
        .pointer("/subtitles/targets")
        .and_then(Value::as_array)
        .is_some_and(|targets| !targets.is_empty());
    if translated && stored.get_mut("subtitles").is_some_and(withdraw_consent) {
        withdrawn.push("subtitle lines");
    }
    for pointer in ANTHROPIC_ONLY {
        remove(stored, pointer);
    }
    stored["provider"] = Value::from(ProviderKind::Gemini.name());
    let mut warning = format!(
        "Anthropic is no longer offered, so the AI settings in {} now use Gemini with its default \
         models, without the Anthropic key or price overrides",
        path.display()
    );
    if !withdrawn.is_empty() {
        let sent = withdrawn.join(" and ");
        warning.push_str(&format!(
            "; consent to send {sent} was for Anthropic, so confirm it again for Google in the editor"
        ));
    }
    Some(warning)
}

/// Whether `stored` was saved for Anthropic: it names Anthropic, or it names no provider and holds
/// an Anthropic key or a Claude model. A file without a provider is on Gemini, and an Anthropic key
/// in it must not be sent to Google.
fn saved_for_anthropic(stored: &Value) -> bool {
    if let Some(provider) = stored.get("provider") {
        return provider.as_str() == Some(ANTHROPIC);
    }
    let text = |pointer: &str| stored.pointer(pointer).and_then(Value::as_str).unwrap_or_default();
    text("/api_key").starts_with(ANTHROPIC_KEY_PREFIX)
        || MODELS
            .iter()
            .any(|pointer| text(pointer).starts_with(CLAUDE_MODEL_PREFIX))
}

/// Withdraws the consent in `settings` and turns off what needed it; returns whether consent had
/// been given.
fn withdraw_consent(settings: &mut Value) -> bool {
    let given = settings.get("consent").and_then(Value::as_bool) == Some(true);
    remove(settings, "/consent");
    remove(settings, "/enabled");
    given
}

/// Removes the field `pointer` names, if it is there.
fn remove(value: &mut Value, pointer: &str) {
    let Some((parent, name)) = pointer.rsplit_once('/') else {
        return;
    };
    if let Some(Value::Object(fields)) = value.pointer_mut(parent) {
        fields.remove(name);
    }
}
