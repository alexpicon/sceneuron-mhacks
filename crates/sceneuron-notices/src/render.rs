// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Reads license texts from package directories and writes the notices document.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::graph::Package;

const RULE: &str = "================================================================================";

/// A license that does not come from a crate, such as the bundled model's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Extra {
    pub title: String,
    pub text: String,
}

/// The statement the Independent JPEG Group's license asks documentation of a binary to make, when
/// a package under that license ships.
pub fn ijg_credit(packages: &[Package]) -> Option<Extra> {
    let names: Vec<&str> = packages
        .iter()
        .filter(|package| {
            let mut ids = package.license.split(|c: char| c.is_whitespace() || "()/".contains(c));
            ids.any(|id| id == "IJG")
        })
        .map(|package| package.name.as_str())
        .collect();
    (!names.is_empty()).then(|| Extra {
        title: format!("Independent JPEG Group ({})", names.join(", ")),
        text: "This software is based in part on the work of the Independent JPEG Group.".to_owned(),
    })
}

/// The license texts a package ships: the manifest's `license-file`, then every file whose
/// name starts with LICENSE, LICENCE, COPYING, or NOTICE, in name order.
pub fn license_texts(package: &Package) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(&package.dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| is_license_name(name))
        .collect();
    names.sort();
    if let Some(named) = &package.license_file
        && !names.contains(named)
    {
        names.insert(0, named.clone());
    }
    names
        .iter()
        .filter_map(|name| read_text(&package.dir.join(name)))
        .collect()
}

/// Fills in license texts for packages that ship none from another package published from
/// the same repository, as crates of one project usually share its license files.
pub fn borrow_from_siblings(packages: &mut [(Package, Vec<String>)]) {
    let donors: Vec<(String, Vec<String>)> = packages
        .iter()
        .filter(|(package, texts)| !texts.is_empty() && !package.repository.is_empty())
        .map(|(package, texts)| (package.repository.clone(), texts.clone()))
        .collect();
    for (package, texts) in packages.iter_mut() {
        if texts.is_empty()
            && let Some((_, donor)) = donors.iter().find(|(repository, _)| *repository == package.repository)
        {
            texts.clone_from(donor);
        }
    }
}

fn is_license_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    ["LICENSE", "LICENCE", "COPYING", "NOTICE"]
        .iter()
        .any(|prefix| upper.starts_with(prefix))
}

fn read_text(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let text = String::from_utf8_lossy(&bytes).replace("\r\n", "\n");
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// The notices document: extras first, then one section per distinct license text listing
/// every package that ships it, then packages whose sources hold no license text.
pub fn render(extras: &[Extra], packages: &[(Package, Vec<String>)]) -> String {
    let mut by_text: BTreeMap<&str, Vec<&Package>> = BTreeMap::new();
    let mut without_text = Vec::new();
    for (package, texts) in packages {
        if texts.is_empty() {
            without_text.push(package);
        }
        for text in texts {
            by_text.entry(text).or_default().push(package);
        }
    }
    let mut sections: Vec<(String, &str)> = by_text
        .into_iter()
        .map(|(text, users)| (users.iter().map(|package| describe(package)).collect(), text))
        .collect();
    sections.sort();
    let mut out = String::from(
        "Third-party notices for Sceneuron\n\nSceneuron includes the software below. Each section names the \
         packages that share one license text, followed by that text.\n",
    );
    for extra in extras {
        out.push_str(&format!("\n{RULE}\n{}\n{RULE}\n\n{}\n", extra.title, extra.text.trim()));
    }
    for (users, text) in sections {
        out.push_str(&format!("\n{RULE}\n{users}{RULE}\n\n{text}\n"));
    }
    if !without_text.is_empty() {
        out.push_str(&format!(
            "\n{RULE}\nPackages whose sources include no license file; their declared license applies.\n{RULE}\n\n"
        ));
        for package in without_text {
            out.push_str(&describe(package));
        }
    }
    out
}

fn describe(package: &Package) -> String {
    let license = if package.license.is_empty() {
        "license not declared"
    } else {
        &package.license
    };
    let repository = if package.repository.is_empty() {
        String::new()
    } else {
        format!(" - {}", package.repository)
    };
    format!("{} {} ({license}){repository}\n", package.name, package.version)
}
