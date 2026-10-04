// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the dependency walk over `cargo metadata` output: normal dependencies are
//! followed transitively, dev and build dependencies and Sceneuron's own crates under the
//! workspace's `crates/` folder are left out, and a path package outside it ships and faces
//! the license gate.

use sceneuron_notices::{refusals, shipped_packages};
use serde_json::{Value, json};

const WORKSPACE: &str = "/work/sceneuron";

fn package(id: &str, name: &str, source: Value) -> Value {
    json!({
        "id": id, "name": name, "version": "1.0.0", "license": "MIT", "repository": "",
        "manifest_path": format!("/registry/{name}/Cargo.toml"), "license_file": null, "source": source,
    })
}

/// One of Sceneuron's own crates, under the workspace's `crates/` folder: proprietary, so it names
/// the repository's LICENSE as its license file and has no license expression.
fn own(id: &str, name: &str) -> Value {
    let mut package = package(id, name, Value::Null);
    package["manifest_path"] = json!(format!("{WORKSPACE}/crates/{name}/Cargo.toml"));
    package["license"] = Value::Null;
    package["license_file"] = json!("../../LICENSE");
    package
}

fn dep(pkg: &str, kind: Value) -> Value {
    json!({ "pkg": pkg, "dep_kinds": [{ "kind": kind, "target": null }] })
}

fn metadata() -> Value {
    let registry = json!("registry+https://github.com/rust-lang/crates.io-index");
    json!({
        "workspace_root": WORKSPACE,
        "packages": [
            own("plugin", "sceneuron-obs"),
            own("core", "sceneuron-core"),
            package("serde", "serde", registry.clone()),
            package("itoa", "itoa", registry.clone()),
            package("cc", "cc", registry.clone()),
            package("proptest", "proptest", registry.clone()),
            package("unused", "unused", registry),
        ],
        "resolve": { "nodes": [
            { "id": "plugin", "deps": [dep("core", Value::Null), dep("cc", json!("build")), dep("proptest", json!("dev"))] },
            { "id": "core", "deps": [dep("serde", Value::Null)] },
            { "id": "serde", "deps": [dep("itoa", Value::Null)] },
            { "id": "itoa", "deps": [] },
            { "id": "cc", "deps": [] },
            { "id": "proptest", "deps": [dep("unused", Value::Null)] },
            { "id": "unused", "deps": [] },
        ]},
    })
}

#[test]
fn follows_normal_dependencies_only_and_skips_sceneurons_own_crates() {
    let names: Vec<String> = shipped_packages(&metadata(), "sceneuron-obs")
        .unwrap()
        .into_iter()
        .map(|package| package.name)
        .collect();
    assert_eq!(names, ["itoa", "serde"]);
}

#[test]
fn sceneurons_own_proprietary_crates_neither_ship_as_third_party_nor_face_the_gate() {
    let shipped = shipped_packages(&metadata(), "sceneuron-obs").unwrap();
    assert!(shipped.iter().all(|package| !package.name.starts_with("sceneuron")));
    assert!(refusals(&shipped).is_empty());
}

#[test]
fn a_path_package_outside_the_crates_folder_ships_and_faces_the_gate() {
    let mut metadata = metadata();
    // Vendored into the repository, or patched in by path: no source, like Sceneuron's own crates.
    let mut vendored = package("bsl", "bsl-crate", Value::Null);
    vendored["manifest_path"] = json!(format!("{WORKSPACE}/vendor/bsl-crate/Cargo.toml"));
    vendored["license"] = json!("BUSL-1.1");
    metadata["packages"].as_array_mut().unwrap().push(vendored);
    let nodes = metadata["resolve"]["nodes"].as_array_mut().unwrap();
    nodes.push(json!({ "id": "bsl", "deps": [] }));
    let core = nodes.iter_mut().find(|node| node["id"] == "core").unwrap();
    core["deps"].as_array_mut().unwrap().push(dep("bsl", Value::Null));
    let shipped = shipped_packages(&metadata, "sceneuron-obs").unwrap();
    let names: Vec<&str> = shipped.iter().map(|package| package.name.as_str()).collect();
    assert_eq!(names, ["bsl-crate", "itoa", "serde"]);
    let refused: Vec<String> = refusals(&shipped)
        .into_iter()
        .map(|refusal| format!("{}: {}", refusal.package.name, refusal.reason))
        .collect();
    assert_eq!(refused, ["bsl-crate: BUSL-1.1 is not on the allowed list"]);
}

#[test]
fn package_fields_come_from_the_metadata() {
    let serde = shipped_packages(&metadata(), "sceneuron-obs").unwrap().pop().unwrap();
    assert_eq!((serde.version.as_str(), serde.license.as_str()), ("1.0.0", "MIT"));
    assert_eq!(serde.dir, std::path::PathBuf::from("/registry/serde"));
}

#[test]
fn reports_missing_roots_and_malformed_metadata() {
    assert!(shipped_packages(&metadata(), "nope").unwrap_err().contains("nope"));
    assert!(shipped_packages(&json!({}), "sceneuron-obs").is_err());
    let mut rootless = metadata();
    rootless["workspace_root"] = Value::Null;
    assert_eq!(
        shipped_packages(&rootless, "sceneuron-obs").unwrap_err(),
        "the metadata has no workspace root"
    );
}
