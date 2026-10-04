// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the license gate: SPDX expressions with OR, AND, WITH, parentheses, and the older `/`,
//! the licenses the plugin's crates use today, refusals of crates that declare only a license file
//! (as SpacetimeDB's SDK did) or no license, and the allowed list matching docs/development.md.

use std::path::PathBuf;

use sceneuron_notices::{ALLOWED, Package, allows, refusals};

fn package(name: &str, license: &str, license_file: Option<&str>) -> Package {
    Package {
        name: name.to_owned(),
        version: "1.0.0".to_owned(),
        license: license.to_owned(),
        repository: String::new(),
        dir: PathBuf::new(),
        license_file: license_file.map(str::to_owned),
    }
}

#[test]
fn every_license_the_plugin_ships_today_is_allowed() {
    for expression in [
        "MIT",
        "MIT OR Apache-2.0",
        "Apache-2.0",
        "Apache-2.0 AND ISC",
        "(MIT OR Apache-2.0) AND Unicode-3.0",
        "(MIT OR Apache-2.0) AND IJG",
        "MIT/Apache-2.0",
        "MIT / Apache-2.0",
        "Unlicense OR MIT",
        "Unlicense/MIT",
        "BlueOak-1.0.0 OR MIT OR Apache-2.0",
        "Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT",
        "BSD-2-Clause OR Apache-2.0 OR MIT",
        "Zlib OR Apache-2.0 OR MIT",
        "CDLA-Permissive-2.0",
        "BSD-3-Clause",
    ] {
        assert_eq!(allows(expression), Ok(true), "{expression}");
    }
}

#[test]
fn and_needs_every_part_and_with_an_exception_is_not_allowed() {
    for expression in [
        "BUSL-1.1",
        "MPL-2.0",
        "GPL-3.0-only",
        "Unlicense",
        "MIT AND BUSL-1.1",
        "(MIT OR Apache-2.0) AND SSPL-1.0",
        "Apache-2.0 WITH LLVM-exception",
        "mit",
    ] {
        assert_eq!(allows(expression), Ok(false), "{expression}");
    }
    assert_eq!(allows("MIT or Apache-2.0"), Ok(true), "operators in any case");
}

#[test]
fn expressions_that_cannot_be_read_are_errors() {
    for expression in [
        "",
        "MIT OR",
        "(MIT",
        "MIT)",
        "AND MIT",
        "MIT WITH",
        "MIT Apache-2.0",
        "()",
    ] {
        assert!(allows(expression).is_err(), "{expression:?}");
    }
}

#[test]
fn crates_with_only_a_license_file_or_none_are_refused_with_the_reason() {
    let packages = [
        package("serde", "MIT OR Apache-2.0", None),
        package("spacetimedb-sdk", "", Some("LICENSE")),
        package("mystery", "", None),
        package("server-side", "SSPL-1.0", None),
        package("garbled", "MIT OR (Apache-2.0", None),
        package("both", "MIT", Some("LICENSE-EXTRA")),
    ];
    let refused: Vec<(String, String)> = refusals(&packages)
        .into_iter()
        .map(|refusal| (refusal.package.name, refusal.reason))
        .collect();
    let expected = [
        (
            "spacetimedb-sdk",
            "declares only a license file, LICENSE, which the gate cannot read",
        ),
        ("mystery", "declares no license"),
        ("server-side", "SSPL-1.0 is not on the allowed list"),
        (
            "garbled",
            "cannot read the license \"MIT OR (Apache-2.0\": a parenthesis is not closed",
        ),
    ];
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(name, reason)| ((*name).to_owned(), (*reason).to_owned()))
        .collect();
    assert_eq!(refused, expected);
}

#[test]
fn the_development_guide_explains_every_allowed_license() {
    let guide = include_str!("../../../docs/development.md");
    let start = guide.find("### License gate").expect("a License gate section");
    let section = &guide[start..];
    let end = section[4..].find("\n## ").map_or(section.len(), |at| at + 4);
    let section = &section[..end];
    for id in ALLOWED {
        assert!(
            section.contains(&format!("`{id}`")),
            "docs/development.md does not explain `{id}`"
        );
    }
}
