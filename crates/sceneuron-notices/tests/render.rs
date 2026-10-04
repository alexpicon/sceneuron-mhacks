// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises license text discovery and the notices document: license file names, the
//! manifest's license-file, shared texts grouped once, borrowed sibling texts, extras, packages
//! without any license file, and the credit the Independent JPEG Group's license asks for.

use std::fs;
use std::path::PathBuf;

use sceneuron_notices::{Extra, Package, borrow_from_siblings, ijg_credit, license_texts, render};

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("notices-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn package(name: &str, dir: PathBuf, repository: &str) -> Package {
    Package {
        name: name.to_owned(),
        version: "1.2.3".to_owned(),
        license: "MIT OR Apache-2.0".to_owned(),
        repository: repository.to_owned(),
        dir,
        license_file: None,
    }
}

#[test]
fn finds_license_files_by_name_and_the_manifest_license_file() {
    let dir = scratch("names");
    fs::write(dir.join("LICENSE-MIT"), "mit text\r\n").unwrap();
    fs::write(dir.join("license-apache"), "apache text").unwrap();
    fs::write(dir.join("COPYING"), "  ").unwrap();
    fs::write(dir.join("README.md"), "readme").unwrap();
    fs::write(dir.join("terms.txt"), "custom terms").unwrap();
    let mut with_file = package("a", dir, "");
    with_file.license_file = Some("terms.txt".to_owned());
    assert_eq!(license_texts(&with_file), ["custom terms", "mit text", "apache text"]);
}

#[test]
fn shared_texts_appear_once_with_every_package_listed() {
    let packages = vec![
        (
            package("alpha", PathBuf::new(), "https://a"),
            vec!["MIT text".to_owned()],
        ),
        (package("beta", PathBuf::new(), ""), vec!["MIT text".to_owned()]),
        (package("gamma", PathBuf::new(), ""), Vec::new()),
    ];
    let extra = Extra {
        title: "Model".to_owned(),
        text: "model license\n".to_owned(),
    };
    let document = render(&[extra], &packages);
    assert_eq!(document.matches("MIT text").count(), 1);
    assert!(document.contains("alpha 1.2.3 (MIT OR Apache-2.0) - https://a\nbeta 1.2.3"));
    assert!(document.find("Model").unwrap() < document.find("alpha").unwrap());
    let missing = document.find("include no license file").unwrap();
    assert!(document[missing..].contains("gamma 1.2.3"));
}

#[test]
fn packages_without_texts_borrow_from_the_same_repository() {
    let mut packages = vec![
        (
            package("tract-core", PathBuf::new(), "https://tract"),
            vec!["tract license".to_owned()],
        ),
        (package("tract-extra", PathBuf::new(), "https://tract"), Vec::new()),
        (package("loner", PathBuf::new(), "https://loner"), Vec::new()),
    ];
    borrow_from_siblings(&mut packages);
    assert_eq!(packages[1].1, ["tract license"]);
    assert!(packages[2].1.is_empty());
}

#[test]
fn a_package_under_the_ijg_license_adds_its_credit() {
    let mut encoder = package("jpeg-encoder", PathBuf::new(), "");
    encoder.license = "(MIT OR Apache-2.0) AND IJG".to_owned();
    let mut lookalike = package("not-ijg", PathBuf::new(), "");
    lookalike.license = "IJG-short".to_owned();
    let credit = ijg_credit(&[encoder, package("serde", PathBuf::new(), ""), lookalike]).unwrap();
    assert_eq!(credit.title, "Independent JPEG Group (jpeg-encoder)");
    assert_eq!(
        credit.text,
        "This software is based in part on the work of the Independent JPEG Group."
    );
    assert_eq!(ijg_credit(&[package("serde", PathBuf::new(), "")]), None);
}
