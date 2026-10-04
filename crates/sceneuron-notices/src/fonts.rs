// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The fonts the plugin serves its overlay to write subtitles in a script of their own, each with
//! the files of its license and notices under `web/fonts/`.

use std::fs;
use std::path::Path;

use crate::render::Extra;

/// Each bundled font: its file under `web/fonts/`, its title in the notices, and the files of its
/// license and notices there.
pub const FONTS: &[(&str, &str, &[&str])] = &[
    (
        "AlcarinTengwarVF.woff2",
        "Alcarin Tengwar 0.83 font by Toshi Omagari, embedded in the plugin for Tengwar on the overlay",
        &["LICENSE-AlcarinTengwar.txt"],
    ),
    (
        "pIqaD-qolqoS.ttf",
        "pIqaD qolqoS 1.0 font by Daniel Dadap, embedded in the plugin for pIqaD on the overlay",
        &["LICENSE-pIqaD-qolqoS.txt", "NOTICE-pIqaD-qolqoS.txt"],
    ),
    (
        "nasin-nanpa-4.0.2-UCSUR.otf",
        "nasin nanpa 4.0.2 font by jan Itan, embedded in the plugin for sitelen pona on the overlay",
        &["LICENSE-nasin-nanpa.txt"],
    ),
];

/// The notices of every bundled font, read from `fonts`, the repository's `web/fonts/`.
pub fn font_notices(fonts: &Path) -> Result<Vec<Extra>, String> {
    FONTS
        .iter()
        .map(|(_, title, files)| {
            let texts: Result<Vec<String>, String> = files
                .iter()
                .map(|file| {
                    let path = fonts.join(file);
                    let text = fs::read_to_string(&path)
                        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
                    Ok(text.replace("\r\n", "\n").trim().to_owned())
                })
                .collect();
            Ok(Extra {
                title: (*title).to_owned(),
                text: texts?.join("\n\n"),
            })
        })
        .collect()
}
