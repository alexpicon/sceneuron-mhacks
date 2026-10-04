// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises spend estimates: Gemini list prices by model id, Gemini Flash's price change on
//! 2027-01-01, no list prices on OpenAI-compatible servers, the streamer's overrides, and cost
//! from token usage.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sceneuron_ai::{AiSettings, DEFAULT_MODEL, Prices, ProviderKind, Usage, cost_usd, gemini_prices, prices_for};

/// 2027-01-01T00:00:00Z in seconds since the Unix epoch.
const NEW_YEAR_2027: u64 = 1_798_761_600;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

/// Input, output, and cache read prices.
fn gemini(model: &str, when: SystemTime) -> Option<(f64, f64, f64)> {
    let prices = gemini_prices(model, when)?;
    Some((prices.input, prices.output, prices.cache_read))
}

#[test]
fn gemini_models_have_their_list_prices() {
    let now = SystemTime::now();
    for (model, expected) in [
        ("gemini-3.5-flash", (1.50, 9.00, 0.15)),
        ("gemini-3.5-flash-lite", (0.30, 2.50, 0.03)),
        ("gemini-3.1-flash-lite", (0.25, 1.50, 0.025)),
        ("gemini-3.1-pro-preview", (2.00, 12.00, 0.20)),
        ("gemini-3-flash-preview", (0.50, 3.00, 0.05)),
        ("gemini-2.5-flash", (0.30, 2.50, 0.03)),
        ("gemini-2.5-flash-lite", (0.10, 0.40, 0.01)),
    ] {
        assert_eq!(gemini(model, now), Some(expected), "{model}");
        assert_eq!(gemini(&format!("models/{model}"), now), Some(expected), "{model}");
    }
    for unknown in ["gemini-3.8-flash-001", "gemini-9", "llama3.2-vision", "models/", ""] {
        assert_eq!(gemini_prices(unknown, now), None, "{unknown}");
    }
}

#[test]
fn gemini_3_6_to_3_8_flash_double_in_price_on_the_first_of_2027() {
    let last_introductory = at(NEW_YEAR_2027 - 1);
    let first_standard = at(NEW_YEAR_2027);
    for model in ["gemini-3.8-flash", "gemini-3.7-flash", "models/gemini-3.6-flash"] {
        assert_eq!(gemini(model, last_introductory), Some((0.75, 3.75, 0.075)), "{model}");
        assert_eq!(gemini(model, first_standard), Some((1.50, 7.50, 0.15)), "{model}");
    }
    let lite = "gemini-3.5-flash-lite";
    assert_eq!(gemini(lite, last_introductory), gemini(lite, first_standard));
}

#[test]
fn gemini_settings_use_gemini_prices_now_and_other_endpoints_have_none() {
    let settings = AiSettings::default();
    assert_eq!(settings.provider, ProviderKind::Gemini);
    let lite = prices_for(&settings, "models/gemini-3.5-flash-lite").unwrap();
    assert_eq!((lite.input, lite.output, lite.cache_read), (0.30, 2.50, 0.03));
    assert_eq!(
        prices_for(&settings, DEFAULT_MODEL),
        gemini_prices(DEFAULT_MODEL, SystemTime::now())
    );
    assert!(prices_for(&settings, DEFAULT_MODEL).is_some());
    assert_eq!(prices_for(&settings, "llama3.2-vision"), None);
    let local = AiSettings {
        provider: ProviderKind::OpenAiCompatible,
        ..AiSettings::default()
    };
    assert_eq!(prices_for(&local, "gemini-3.5-flash-lite"), None);
    assert_eq!(prices_for(&local, "llama3.2-vision"), None);
    // A call with 1,300 prompt tokens of which 1,000 were cached, and 7 + 110 thinking out.
    let usage = Usage {
        input_tokens: 300,
        output_tokens: 117,
        cache_read_tokens: 1_000,
    };
    assert!(close(
        cost_usd(&usage, &lite),
        (300.0 * 0.30 + 117.0 * 2.50 + 1_000.0 * 0.03) / 1e6
    ));
}

#[test]
fn overrides_come_first() {
    let pro = AiSettings {
        planner_model: "gemini-3.1-pro-preview".to_owned(),
        ..AiSettings::default()
    };
    let both = AiSettings {
        input_price_per_mtok: Some(1.0),
        output_price_per_mtok: Some(2.0),
        ..pro.clone()
    };
    let prices = prices_for(&both, &pro.planner_model).unwrap();
    assert_eq!((prices.input, prices.output), (1.0, 2.0));
    assert!(close(prices.cache_read, 0.1));
    let output_only = AiSettings {
        output_price_per_mtok: Some(30.0),
        ..pro.clone()
    };
    let prices = prices_for(&output_only, &pro.planner_model).unwrap();
    assert_eq!((prices.input, prices.output, prices.cache_read), (2.0, 30.0, 0.20));
    let local = AiSettings {
        provider: ProviderKind::OpenAiCompatible,
        input_price_per_mtok: Some(0.5),
        ..AiSettings::default()
    };
    let prices = prices_for(&local, "llama").unwrap();
    assert_eq!((prices.input, prices.output), (0.5, 0.0));
    assert!(close(prices.cache_read, 0.05));
}

#[test]
fn cost_adds_every_kind_of_token_at_its_price() {
    let prices = Prices {
        input: 2.0,
        output: 12.0,
        cache_read: 0.2,
    };
    let million = Usage {
        input_tokens: 1_000_000,
        output_tokens: 1_000_000,
        cache_read_tokens: 1_000_000,
    };
    assert!(close(cost_usd(&million, &prices), 2.0 + 12.0 + 0.2));
    let pro = gemini_prices("gemini-3.1-pro-preview", SystemTime::now()).unwrap();
    let typical = Usage {
        input_tokens: 1_000,
        output_tokens: 200,
        cache_read_tokens: 900,
    };
    assert!(close(cost_usd(&typical, &pro), 0.002 + 0.0024 + 0.00018));
    assert_eq!(cost_usd(&Usage::default(), &pro), 0.0);
}
