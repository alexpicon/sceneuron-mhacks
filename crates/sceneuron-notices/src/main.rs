// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Writes THIRD-PARTY-NOTICES.txt for the Sceneuron plugin, or checks that every crate that ships
//! in the plugin and the development server has a license the plugin can be distributed with.
//!
//! Usage: `sceneuron-notices --target TRIPLE [--target TRIPLE ...] --out FILE`, or
//! `sceneuron-notices --check --target TRIPLE [--target TRIPLE ...]`. Run it with
//! `cargo run -p sceneuron-notices`, which tells it which cargo to ask for metadata.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::{env, fs};

use sceneuron_notices::{
    Extra, Package, borrow_from_siblings, font_notices, ijg_credit, license_texts, refusals, render, shipped_packages,
};

const PLUGIN: &str = "sceneuron-obs";
const DEV_SERVER: &str = "sceneuron-dev";
const MODEL_LICENSE: &str = "crates/sceneuron-vision/models/LICENSE-ultraface.txt";
const FONTS_DIR: &str = "web/fonts";

/// What a run does.
enum Mode {
    /// Writes the plugin's notices to this file.
    Write(PathBuf),
    /// Checks the licenses of what ships.
    Check,
}

fn main() -> ExitCode {
    let outcome = parse_args(env::args().skip(1)).and_then(|(targets, mode)| match mode {
        Mode::Write(out) => write(&targets, &out).map(|()| format!("wrote {}", out.display())),
        Mode::Check => check(&targets),
    });
    match outcome {
        Ok(message) => {
            eprintln!("sceneuron-notices: {message}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("sceneuron-notices: error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn write(targets: &[String], out: &Path) -> Result<(), String> {
    let mut packages = BTreeSet::new();
    for target in targets {
        packages.extend(packages_for(target, &[PLUGIN])?);
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let model = fs::read_to_string(root.join(MODEL_LICENSE))
        .map_err(|error| format!("cannot read {MODEL_LICENSE}: {error}"))?;
    let packages: Vec<Package> = packages.into_iter().collect();
    let model = Extra {
        title: "UltraFace RFB-320 face detection model, embedded in the plugin".to_owned(),
        text: model,
    };
    let fonts = font_notices(&root.join(FONTS_DIR))?;
    let extras: Vec<Extra> = [Some(model), ijg_credit(&packages)]
        .into_iter()
        .flatten()
        .chain(fonts)
        .collect();
    let mut with_texts: Vec<(Package, Vec<String>)> = packages
        .into_iter()
        .map(|package| {
            let texts = license_texts(&package);
            (package, texts)
        })
        .collect();
    borrow_from_siblings(&mut with_texts);
    fs::write(out, render(&extras, &with_texts)).map_err(|error| format!("cannot write {}: {error}", out.display()))
}

/// Lists every license that ships, counted, and fails when one is not allowed.
fn check(targets: &[String]) -> Result<String, String> {
    let mut packages = BTreeSet::new();
    for target in targets {
        packages.extend(packages_for(target, &[PLUGIN, DEV_SERVER])?);
    }
    let packages: Vec<Package> = packages.into_iter().collect();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for package in &packages {
        *counts.entry(package.license.as_str()).or_default() += 1;
    }
    eprintln!(
        "sceneuron-notices: {} crates ship in {PLUGIN} or {DEV_SERVER} on {}:",
        packages.len(),
        targets.join(", ")
    );
    for (license, count) in &counts {
        let license = if license.is_empty() {
            "(no license expression)"
        } else {
            license
        };
        eprintln!("  {count:4}  {license}");
    }
    let refused = refusals(&packages);
    if refused.is_empty() {
        return Ok("every license is on the allowed list".to_owned());
    }
    for refusal in &refused {
        let package = &refusal.package;
        eprintln!("  refused: {} {}: {}", package.name, package.version, refusal.reason);
    }
    Err(format!(
        "{} crates have licenses the plugin cannot ship with; see docs/development.md",
        refused.len()
    ))
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<(Vec<String>, Mode), String> {
    let mut targets = Vec::new();
    let mut out = None;
    let mut checking = false;
    while let Some(arg) = args.next() {
        if arg == "--check" {
            checking = true;
            continue;
        }
        let value = args.next().ok_or_else(|| format!("{arg} needs a value"))?;
        match arg.as_str() {
            "--target" => targets.push(value),
            "--out" => out = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown argument {arg}")),
        }
    }
    if targets.is_empty() {
        return Err("give at least one --target TRIPLE".to_owned());
    }
    let mode = match (checking, out) {
        (true, None) => Mode::Check,
        (true, Some(_)) => return Err("--check writes nothing; leave out --out".to_owned()),
        (false, Some(out)) => Mode::Write(out),
        (false, None) => return Err("missing --out FILE, or --check".to_owned()),
    };
    Ok((targets, mode))
}

/// The packages that ship in each of `roots` on `target`.
fn packages_for(target: &str, roots: &[&str]) -> Result<Vec<Package>, String> {
    let metadata = metadata(target)?;
    let mut packages = Vec::new();
    for root in roots {
        packages.extend(shipped_packages(&metadata, root)?);
    }
    Ok(packages)
}

fn metadata(target: &str) -> Result<serde_json::Value, String> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--filter-platform",
            target,
        ])
        .output()
        .map_err(|error| format!("cannot run cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::from_slice(&output.stdout).map_err(|error| format!("bad cargo metadata: {error}"))
}
