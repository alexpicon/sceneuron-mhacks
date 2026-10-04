// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Tries TypeSafe's Jev on the default project's conditions with a few scenes described the way
//! on-device perception describes them, and prints each probability, the latency, and what the
//! calls cost. Only text is sent, never an image, but each call may cost money.
//!
//! ```text
//! cargo run -p sceneuron-ai --example quick
//! cargo run -p sceneuron-ai --example quick -- --fact "face in view (0.97)" --fact "cat (0.91)"
//! ```
//!
//! The key comes from `SCENEURON_JEV_API_KEY`, in the environment or in the `.env` file of the
//! current directory (or the file `SCENEURON_ENV_FILE` names).

use std::env;
use std::path::Path;
use std::process::ExitCode;

use sceneuron_ai::{AiSettings, Credentials, JevClient, env_file_path, quick_verdict};
use sceneuron_core::{Condition, default_project};

const USAGE: &str = "usage: quick [--fact TEXT]...";

/// Scenes in the words of the on-device perception facts.
const SCENES: [(&str, &[&str]); 4] = [
    (
        "at the desk",
        &["face in view (0.98)", "right hand in view", "keyboard (0.64)"],
    ),
    (
        "holding up a mug",
        &[
            "face in view (0.97)",
            "right hand raised",
            "cup (0.86)",
            "text: \"OAT MILK\"",
        ],
    ),
    (
        "the cat on the desk",
        &["face in view (0.95)", "cat (0.93)", "keyboard (0.58)"],
    ),
    ("an empty room", &["chair (0.71)", "wall (0.52)"]),
];

fn parse_args() -> Result<Vec<String>, String> {
    let mut facts = Vec::new();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--fact" => facts.push(args.next().ok_or_else(|| format!("--fact needs a value\n{USAGE}"))?),
            _ => return Err(USAGE.to_owned()),
        }
    }
    Ok(facts)
}

fn run() -> Result<(), String> {
    let custom = parse_args()?;
    let (credentials, warnings) = Credentials::load(Some(&env_file_path(Path::new(".env"))));
    for warning in warnings {
        eprintln!("{warning}");
    }
    let settings = AiSettings {
        jev_api_key: credentials.jev,
        ..AiSettings::default()
    };
    let client = JevClient::new(&settings).ok_or("no Jev key: set SCENEURON_JEV_API_KEY")?;
    let project = default_project();
    let conditions: Vec<&Condition> = project.conditions.iter().collect();
    let scenes: Vec<(&str, Vec<String>)> = if custom.is_empty() {
        SCENES
            .iter()
            .map(|(name, facts)| (*name, facts.iter().map(|fact| (*fact).to_owned()).collect()))
            .collect()
    } else {
        vec![("your facts", custom)]
    };
    let (mut tokens, mut cost, mut latencies) = (0, 0.0, Vec::new());
    for (name, facts) in &scenes {
        let judgment = client.judge(&conditions, facts).map_err(|error| error.to_string())?;
        println!("{name}: {}", facts.join("; "));
        for (id, probability) in &judgment.probabilities {
            let verdict = quick_verdict(id, *probability);
            println!(
                "  {id}: {probability:.2} -> holds {} ({:.2})",
                verdict.holds, verdict.confidence
            );
        }
        println!(
            "  model {}, {:.0} ms, {} input tokens, ${:.6}",
            judgment.model,
            judgment.latency_ms,
            judgment.usage.input_tokens,
            judgment.cost_usd()
        );
        tokens += judgment.usage.input_tokens;
        cost += judgment.cost_usd();
        latencies.push(judgment.latency_ms);
    }
    latencies.sort_by(f64::total_cmp);
    println!(
        "{} calls, median {:.0} ms, {tokens} input tokens, ${cost:.6} in total",
        latencies.len(),
        latencies[latencies.len() / 2]
    );
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
