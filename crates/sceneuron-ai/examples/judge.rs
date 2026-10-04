// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Tries a real vision model on one JPEG photo and one condition, and prints what it saw and
//! what the call cost. This sends the photo to the provider and may cost money.
//!
//! ```text
//! cargo run -p sceneuron-ai --example judge -- photo.jpg "A cat is visible." --detail "the animal"
//! cargo run -p sceneuron-ai --example judge -- photo.jpg "A cat is visible." --model gemini-3.1-pro-preview
//! cargo run -p sceneuron-ai --example judge -- photo.jpg "A mug is held up." \
//!     --provider openai-compatible --base-url http://127.0.0.1:11434/v1 --model llama3.2-vision
//! ```
//!
//! The provider is Gemini unless `--provider` names another, and the model is that provider's
//! default unless `--model` names one. Keys come from `SCENEURON_GEMINI_API_KEY` or
//! `SCENEURON_OPENAI_API_KEY`, in the environment or in the `.env` file of the current directory
//! (or the file `SCENEURON_ENV_FILE` names); an OpenAI-compatible server may need none.

use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use sceneuron_ai::{
    AiSettings, Credentials, ProviderKind, connect, cost_usd, env_file_path, judge_request, parse_observation,
    prices_for,
};
use sceneuron_core::Condition;

const USAGE: &str = "usage: judge <photo.jpg> <condition> [--detail TEXT] \
                     [--provider gemini|openai-compatible] [--base-url URL] [--model NAME] \
                     [--effort low|medium|high]";

struct Args {
    photo: String,
    condition: String,
    detail: String,
    settings: AiSettings,
}

fn parse_args() -> Result<Args, String> {
    let mut positional = Vec::new();
    let mut settings = AiSettings {
        enabled: true,
        consent: true,
        ..AiSettings::default()
    };
    let mut detail = String::new();
    let mut base_url = None;
    let mut model = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value\n{USAGE}"));
        match arg.as_str() {
            "--detail" => detail = value()?,
            "--provider" => {
                settings.provider = ProviderKind::try_from(value()?).map_err(|error| format!("{error}\n{USAGE}"))?;
            }
            "--base-url" => base_url = Some(value()?),
            "--model" => model = Some(value()?),
            "--effort" => settings.vision_effort = value()?,
            "--help" | "-h" => return Err(USAGE.to_owned()),
            _ => positional.push(arg),
        }
    }
    let [photo, condition] = <[String; 2]>::try_from(positional).map_err(|_| USAGE.to_owned())?;
    settings.base_url = base_url.unwrap_or_else(|| settings.provider.default_base_url().to_owned());
    let provider_default = settings
        .provider
        .default_models()
        .map(|models| models.vision.to_owned());
    if let Some(model) = model.or(provider_default) {
        settings.vision_model = model;
    }
    let (credentials, warnings) = Credentials::load(Some(&env_file_path(Path::new(".env"))));
    for warning in warnings {
        eprintln!("{warning}");
    }
    settings.api_key = credentials.for_provider(settings.provider).to_owned();
    settings.validate().map_err(|error| error.to_string())?;
    Ok(Args {
        photo,
        condition,
        detail,
        settings,
    })
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let jpeg = fs::read(&args.photo).map_err(|error| format!("cannot read {}: {error}", args.photo))?;
    let condition = Condition {
        id: "condition".to_owned(),
        name: "Condition".to_owned(),
        prompt: args.condition,
        detail: args.detail,
        enabled: true,
        min_confidence: 0.6,
        confirm: 1,
        release: 2,
    };
    let provider = connect(&args.settings).map_err(|error| error.to_string())?;
    let request = judge_request(&[condition], &[], jpeg, &args.settings);
    let reply = provider.complete(&request).map_err(|error| error.to_string())?;
    let observation = parse_observation(&reply.json).map_err(|error| error.to_string())?;
    println!("caption: {}", observation.caption);
    for verdict in &observation.verdicts {
        println!(
            "holds: {} (confidence {:.2}) detail: {:?}\nreason: {}",
            verdict.holds, verdict.confidence, verdict.detail, verdict.reason
        );
    }
    let usage = reply.usage;
    println!(
        "model: {}  latency: {:.0} ms\ntokens: {} in, {} out, {} cache read",
        reply.model, reply.latency_ms, usage.input_tokens, usage.output_tokens, usage.cache_read_tokens
    );
    match prices_for(&args.settings, &reply.model) {
        Some(prices) => println!("estimated cost: ${:.5}", cost_usd(&usage, &prices)),
        None => println!("estimated cost: unknown (no price for {})", reply.model),
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
