// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Spend estimates from token usage and per-model prices.
//!
//! Prices are list prices in US dollars per million tokens. Estimates ignore discounts, taxes,
//! and requests the provider does not bill, so they are a guide for the hourly budget rather
//! than an invoice. Writing to a prompt cache costs nothing: Gemini's implicit caching is free,
//! and its explicit caches, which Sceneuron does not create, bill storage by the hour instead.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::provider::Usage;
use crate::provider_kind::ProviderKind;
use crate::settings::AiSettings;

/// Prices in US dollars per million tokens.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Prices {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
}

impl Prices {
    const fn new(input: f64, output: f64, cache_read: f64) -> Prices {
        Prices {
            input,
            output,
            cache_read,
        }
    }
}

/// Cache reads cost a tenth of the input price unless a model's list price says otherwise.
const CACHE_READ_FACTOR: f64 = 0.1;

/// Standard-tier list prices, with thinking billed as output, and Gemini 3.1 Pro's for prompts
/// up to 200k tokens, from <https://ai.google.dev/gemini-api/docs/pricing>.
const GEMINI_PRICES: &[(&str, Prices)] = &[
    ("gemini-3.5-flash", Prices::new(1.50, 9.00, 0.15)),
    ("gemini-3.5-flash-lite", Prices::new(0.30, 2.50, 0.03)),
    ("gemini-3.1-flash-lite", Prices::new(0.25, 1.50, 0.025)),
    ("gemini-3.1-pro-preview", Prices::new(2.00, 12.00, 0.20)),
    ("gemini-3-flash-preview", Prices::new(0.50, 3.00, 0.05)),
    ("gemini-2.5-flash", Prices::new(0.30, 2.50, 0.03)),
    ("gemini-2.5-flash-lite", Prices::new(0.10, 0.40, 0.01)),
];
/// Gemini 3.6 to 3.8 Flash cost their introductory prices through 2026, and their standard ones
/// from `GEMINI_FLASH_STANDARD_FROM`.
const GEMINI_DATED_FLASH: [&str; 3] = ["gemini-3.8-flash", "gemini-3.7-flash", "gemini-3.6-flash"];
const GEMINI_FLASH_INTRODUCTORY: Prices = Prices::new(0.75, 3.75, 0.075);
const GEMINI_FLASH_STANDARD: Prices = Prices::new(1.50, 7.50, 0.15);
/// 2027-01-01T00:00:00Z in seconds since the Unix epoch.
const GEMINI_FLASH_STANDARD_FROM: u64 = 1_798_761_600;

/// List prices of a Gemini model at `when`, by id with or without the `models/` prefix.
pub fn gemini_prices(model: &str, when: SystemTime) -> Option<Prices> {
    let id = model.strip_prefix("models/").unwrap_or(model);
    if GEMINI_DATED_FLASH.contains(&id) {
        let standard_from = UNIX_EPOCH + Duration::from_secs(GEMINI_FLASH_STANDARD_FROM);
        let prices = if when < standard_from {
            GEMINI_FLASH_INTRODUCTORY
        } else {
            GEMINI_FLASH_STANDARD
        };
        return Some(prices);
    }
    GEMINI_PRICES
        .iter()
        .find(|(name, _)| *name == id)
        .map(|(_, prices)| *prices)
}

/// The prices used for `model` now: the streamer's overrides first, then the list prices of
/// known Gemini models, otherwise none (the spend of such calls is counted as zero).
///
/// When the input price is overridden, cache reads are priced from it at the usual ratio;
/// OpenAI-compatible servers report no cached tokens.
pub fn prices_for(settings: &AiSettings, model: &str) -> Option<Prices> {
    let known = match settings.provider {
        ProviderKind::Gemini => gemini_prices(model, SystemTime::now()),
        ProviderKind::OpenAiCompatible => None,
    };
    let (input_override, output_override) = (settings.input_price_per_mtok, settings.output_price_per_mtok);
    if input_override.is_none() && output_override.is_none() {
        return known;
    }
    let input = input_override.or(known.map(|prices| prices.input)).unwrap_or(0.0);
    let output = output_override.or(known.map(|prices| prices.output)).unwrap_or(0.0);
    let cache_read = match (input_override, known) {
        (None, Some(known)) => known.cache_read,
        _ => input * CACHE_READ_FACTOR,
    };
    Some(Prices::new(input, output, cache_read))
}

/// Estimated cost of one call in US dollars.
pub fn cost_usd(usage: &Usage, prices: &Prices) -> f64 {
    let millions = |tokens: u64| tokens as f64 / 1_000_000.0;
    millions(usage.input_tokens) * prices.input
        + millions(usage.output_tokens) * prices.output
        + millions(usage.cache_read_tokens) * prices.cache_read
}
