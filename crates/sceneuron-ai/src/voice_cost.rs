// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Spend estimates for ElevenLabs: characters of text to speech and seconds of realtime speech
//! to text, at the list prices on https://elevenlabs.io/pricing/api (checked 2026-10-02). The
//! price per unit is the same on every paid plan; characters or hours a plan includes, and
//! limited-time discounts, are not subtracted, so the estimate is an upper bound for the hourly
//! budget. A model without a list price here is counted at the highest known rate of its kind,
//! so its spend is never counted as zero.

use serde::Serialize;

/// US dollars per 1,000 characters for the Flash, Turbo, v4 Turbo, and v3 Conversational models.
const FLASH_PER_1K_CHARS: f64 = 0.04;
/// US dollars per 1,000 characters for Multilingual v2, v3, and v4.
const MULTILINGUAL_PER_1K_CHARS: f64 = 0.08;
/// US dollars per hour of audio for Scribe v2 Realtime.
const REALTIME_PER_HOUR: f64 = 0.39;
/// US dollars per hour of audio for keyterm prompting. The pricing page lists this add-on for
/// Scribe v2; the keyterm guide says realtime keyterms also cost extra
/// (https://elevenlabs.io/docs/eleven-api/guides/how-to/speech-to-text/batch/keyterm-prompting),
/// so the same rate is assumed.
const KEYTERMS_PER_HOUR: f64 = 0.05;

const TTS_PRICES: &[(&str, f64)] = &[
    ("eleven_flash_v2_5", FLASH_PER_1K_CHARS),
    ("eleven_flash_v2", FLASH_PER_1K_CHARS),
    ("eleven_turbo_v2_5", FLASH_PER_1K_CHARS),
    ("eleven_turbo_v2", FLASH_PER_1K_CHARS),
    ("eleven_v4_turbo", FLASH_PER_1K_CHARS),
    ("eleven_v3_conversational", FLASH_PER_1K_CHARS),
    ("eleven_multilingual_v2", MULTILINGUAL_PER_1K_CHARS),
    ("eleven_v3", MULTILINGUAL_PER_1K_CHARS),
    ("eleven_v4", MULTILINGUAL_PER_1K_CHARS),
];

const STT_PRICES: &[(&str, f64)] = &[("scribe_v2_realtime", REALTIME_PER_HOUR)];

/// What voice features used: characters spoken and seconds of audio transcribed, of which
/// `keyterm_seconds` were sent with keyterms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct VoiceUsage {
    pub characters: u64,
    pub audio_seconds: f64,
    pub keyterm_seconds: f64,
}

/// The list price of a text to speech model per 1,000 characters, if known.
pub fn tts_price_per_1k_chars(model: &str) -> Option<f64> {
    price(TTS_PRICES, model)
}

/// The list price of a realtime speech to text model per hour of audio, if known.
pub fn stt_price_per_hour(model: &str) -> Option<f64> {
    price(STT_PRICES, model)
}

/// The price a text to speech model's spend is counted at: its list price, or else the highest
/// known one.
pub fn tts_rate_per_1k_chars(model: &str) -> f64 {
    tts_price_per_1k_chars(model).unwrap_or_else(|| highest(TTS_PRICES))
}

/// The price a speech to text model's spend is counted at per hour, keyterms aside: its list
/// price, or else the highest known one.
pub fn stt_rate_per_hour(model: &str) -> f64 {
    stt_price_per_hour(model).unwrap_or_else(|| highest(STT_PRICES))
}

/// The extra price per hour of audio sent with keyterms.
pub fn keyterm_rate_per_hour() -> f64 {
    KEYTERMS_PER_HOUR
}

/// Estimated cost of `usage` in US dollars, with unknown models at the highest known rate.
pub fn voice_cost_usd(usage: &VoiceUsage, tts_model: &str, stt_model: &str) -> f64 {
    let speech = usage.characters as f64 / 1_000.0 * tts_rate_per_1k_chars(tts_model);
    let hours = |seconds: f64| {
        if seconds.is_finite() {
            seconds.max(0.0) / 3_600.0
        } else {
            0.0
        }
    };
    let listening = hours(usage.audio_seconds) * stt_rate_per_hour(stt_model);
    let keyterms = hours(usage.keyterm_seconds) * KEYTERMS_PER_HOUR;
    speech + listening + keyterms
}

fn price(prices: &[(&str, f64)], model: &str) -> Option<f64> {
    prices.iter().find(|(name, _)| *name == model).map(|(_, price)| *price)
}

fn highest(prices: &[(&str, f64)]) -> f64 {
    prices.iter().map(|(_, price)| *price).fold(0.0, f64::max)
}
