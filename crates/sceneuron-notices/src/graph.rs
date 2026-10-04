// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Finds the packages that ship inside a workspace crate, from `cargo metadata` output.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value;

/// A third-party package compiled into the shipped binary.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Package {
    pub name: String,
    pub version: String,
    /// The SPDX license expression from the manifest, or empty when it declares none.
    pub license: String,
    pub repository: String,
    /// The directory that holds the package's manifest and license files.
    pub dir: PathBuf,
    /// A license file the manifest names explicitly, relative to `dir`.
    pub license_file: Option<String>,
}

/// Every package reachable from `root` through normal (not dev or build) dependencies,
/// except Sceneuron's own crates, sorted by name and version.
///
/// Sceneuron's own crates are the ones under the workspace's `crates/` folder. Any other
/// package ships under its own license, including one vendored into the repository or
/// patched in by path, which has no source either. Build dependencies only run while
/// compiling and dev dependencies only in tests, so neither ships. Errors name what is
/// missing from the metadata.
pub fn shipped_packages(metadata: &Value, root: &str) -> Result<Vec<Package>, String> {
    let workspace = metadata["workspace_root"]
        .as_str()
        .ok_or("the metadata has no workspace root")?;
    let crates = Path::new(workspace).join("crates");
    let own = |package: &Value| {
        package["manifest_path"]
            .as_str()
            .is_some_and(|manifest| Path::new(manifest).starts_with(&crates))
    };
    let packages: BTreeMap<&str, &Value> = metadata["packages"]
        .as_array()
        .ok_or("the metadata has no packages")?
        .iter()
        .filter_map(|package| Some((package["id"].as_str()?, package)))
        .collect();
    let nodes: BTreeMap<&str, &Value> = metadata["resolve"]["nodes"]
        .as_array()
        .ok_or("the metadata has no resolve graph")?
        .iter()
        .filter_map(|node| Some((node["id"].as_str()?, node)))
        .collect();
    let root_id = packages
        .iter()
        .find(|(_, package)| package["name"] == root && own(package))
        .map(|(id, _)| *id)
        .ok_or_else(|| format!("no workspace package named {root}"))?;
    let mut seen = BTreeSet::from([root_id]);
    let mut pending = vec![root_id];
    while let Some(id) = pending.pop() {
        let Some(node) = nodes.get(id) else {
            continue;
        };
        for dependency in node["deps"].as_array().into_iter().flatten() {
            let normal = dependency["dep_kinds"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|kind| kind["kind"].is_null());
            if let Some(next) = dependency["pkg"].as_str().filter(|_| normal)
                && seen.insert(next)
            {
                pending.push(next);
            }
        }
    }
    let mut shipped: Vec<Package> = seen
        .into_iter()
        .filter_map(|id| packages.get(id))
        .filter(|package| !own(package))
        .map(|package| package_of(package))
        .collect();
    shipped.sort();
    Ok(shipped)
}

fn package_of(package: &Value) -> Package {
    let text = |key: &str| package[key].as_str().unwrap_or_default().to_owned();
    let manifest = PathBuf::from(text("manifest_path"));
    Package {
        name: text("name"),
        version: text("version"),
        license: text("license"),
        repository: text("repository"),
        dir: manifest.parent().map(PathBuf::from).unwrap_or_default(),
        license_file: package["license_file"].as_str().map(str::to_owned),
    }
}
