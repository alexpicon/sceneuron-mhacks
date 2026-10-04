// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the bundled fonts' notices: every font under `web/fonts/` is listed with its license,
//! each license is the SIL Open Font License or MIT with its copyright line, and the notices carry
//! each text under the font's title.

use std::fs;
use std::path::PathBuf;

use sceneuron_notices::{FONTS, font_notices};

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/fonts")
}

#[test]
fn every_bundled_font_is_listed_and_every_listed_file_is_there() {
    let mut on_disk: Vec<String> = fs::read_dir(fonts_dir())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| [".woff2", ".ttf", ".otf"].iter().any(|kind| name.ends_with(kind)))
        .collect();
    on_disk.sort();
    let mut listed: Vec<String> = FONTS.iter().map(|(file, ..)| (*file).to_owned()).collect();
    listed.sort();
    assert_eq!(on_disk, listed);
    for (_, _, files) in FONTS {
        for file in *files {
            assert!(fonts_dir().join(file).is_file(), "{file}");
        }
    }
}

#[test]
fn each_font_ships_its_open_font_license_or_mit_text_under_its_title() {
    let notices = font_notices(&fonts_dir()).unwrap();
    assert_eq!(notices.len(), FONTS.len());
    for (extra, (_, title, _)) in notices.iter().zip(FONTS) {
        assert_eq!(extra.title, *title);
        let licensed = extra.text.contains("SIL OPEN FONT LICENSE Version 1.1")
            || extra.text.contains("Permission is hereby granted, free of charge");
        assert!(licensed && extra.text.contains("Copyright"), "{title}");
    }
    assert!(
        notices[1].text.contains("https://hol.kag.org"),
        "the attribution qurgh's page asks for"
    );
    let missing = font_notices(&fonts_dir().join("missing"));
    assert!(missing.unwrap_err().starts_with("cannot read "));
}
