// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the languages subtitles translate into: BCP 47 tags as RFC 5646 writes them, private
//! use included and nothing looser, their usual case, the names of natural, constructed, and
//! fictional languages by the longest listed code a tag starts with, which are constructed, and
//! that the editor lists and names the same languages by the same names.

use sceneuron_core::{
    canonical_language_tag, constructed_language, constructed_languages, language_label, language_name,
    natural_languages, valid_language_tag,
};

/// Constructed languages whose codes the ISO 639-3 registry (iso639-3.sil.org) lists as active
/// codes for constructed languages, by the ISO 639-1 code where one exists, as BCP 47 asks.
const ISO_CONSTRUCTED: [(&str, &str); 12] = [
    ("qya", "Quenya (Elvish)"),
    ("sjn", "Sindarin (Elvish)"),
    ("tlh", "Klingon"),
    ("tok", "Toki Pona"),
    ("jbo", "Lojban"),
    ("eo", "Esperanto"),
    ("ia", "Interlingua"),
    ("vo", "Volapük"),
    ("ldn", "Láadan"),
    ("io", "Ido"),
    ("ie", "Interlingue (Occidental)"),
    ("nov", "Novial"),
];

/// Fictional languages without an ISO 639 code, as private-use tags under `art`.
const PRIVATE_USE: [(&str, &str); 3] = [
    ("art-x-dothraki", "Dothraki"),
    ("art-x-valyrian", "High Valyrian"),
    ("art-x-navi", "Na'vi"),
];

#[test]
fn constructed_languages_take_their_iso_code_or_a_private_use_tag() {
    for (code, name) in ISO_CONSTRUCTED.into_iter().chain(PRIVATE_USE) {
        assert!(valid_language_tag(code), "{code}");
        assert_eq!(language_name(code), Some(name), "{code}");
        assert!(constructed_language(code), "{code}");
        assert_eq!(canonical_language_tag(&code.to_uppercase()).as_deref(), Some(code));
    }
    let mut expected: Vec<(&str, &str)> = ISO_CONSTRUCTED.into_iter().chain(PRIVATE_USE).collect();
    let mut listed = constructed_languages().to_vec();
    expected.sort();
    listed.sort();
    assert_eq!(listed, expected, "exactly these");
}

#[test]
fn well_formed_tags_are_accepted_with_every_part_rfc_5646_allows() {
    for good in [
        "es",
        "zh-yue-HK",
        "zh-Hans",
        "sr-Latn-RS",
        "es-419",
        "de-1996",
        "sl-rozaj-biske",
        "en-u-ca-gregory",
        "en-a-bbb-x-a-ccc",
        "de-CH-1901-x-old",
        "en-x-a",
        "art-x-12345678",
    ] {
        assert!(valid_language_tag(good), "{good}");
    }
}

#[test]
fn malformed_tags_and_anything_looser_are_refused() {
    for bad in [
        "x-dothraki",
        "i-klingon",
        "art-x",
        "art-x-",
        "art-x-highvalyrian",
        "art-x-na'vi",
        "art-x-dothraki-",
        "en-x",
        "en-a",
        "en-a-x-b",
        "en-a-b",
        "en-a-bb-a-cc",
        "de-1996-1996",
        "en-US-GB",
        "zh-Hans-Hant",
        "en-1a",
        "en-abc-US-def",
        "es-419-MX",
        "zh-yue-yue-yue-yue",
        "en-Latn-yue",
        "qya-",
        "q-ya",
    ] {
        assert!(!valid_language_tag(bad), "{bad}");
    }
}

#[test]
fn private_use_and_extensions_stay_in_lowercase() {
    assert_eq!(canonical_language_tag("ART-X-Navi").as_deref(), Some("art-x-navi"));
    assert_eq!(
        canonical_language_tag("en-x-US").as_deref(),
        Some("en-x-us"),
        "no region after x"
    );
    assert_eq!(canonical_language_tag("sr-latn-x-ab").as_deref(), Some("sr-Latn-x-ab"));
    assert_eq!(
        canonical_language_tag("en-u-CA-gregory").as_deref(),
        Some("en-u-ca-gregory")
    );
    assert_eq!(canonical_language_tag("x-dothraki"), None);
}

#[test]
fn languages_are_named_by_the_longest_listed_code_they_start_with() {
    assert_eq!(language_name("ES-mx"), Some("Spanish"), "a region of a listed language");
    assert_eq!(language_name("pt-BR"), Some("Portuguese (Brazil)"));
    assert_eq!(
        language_name("sr-Latn-RS"),
        Some("Serbian (Latin)"),
        "a region of a listed script"
    );
    assert_eq!(language_name("zh-Hant-TW"), Some("Chinese (Traditional)"));
    assert_eq!(
        language_name("zh-TW"),
        Some("Chinese"),
        "a language listed only by script"
    );
    assert_eq!(
        language_name("pt-AO"),
        Some("Portuguese"),
        "a language listed only by country"
    );
    assert_eq!(language_name("qya-x-old"), Some("Quenya (Elvish)"));
    assert_eq!(language_name("art-x-huttese"), None);
    assert_eq!(language_name("nan"), None, "only listed languages");
    assert_eq!(language_label("sr-Latn-RS"), "Serbian (Latin, sr-Latn-RS)");
    assert_eq!(language_label("es"), "Spanish (es)");
    assert_eq!(language_label("qya"), "Quenya (Elvish, qya)");
    assert_eq!(language_label(" art-x-huttese "), "art-x-huttese");
    assert!(constructed_language("art-x-huttese") && constructed_language("ART"));
    for natural in ["es", "la", "en-x-elvish", "", "arz"] {
        assert!(!constructed_language(natural), "{natural}");
    }
}

#[test]
fn natural_languages_cover_the_most_spoken_and_never_repeat() {
    let codes: Vec<&str> = natural_languages().iter().map(|(code, _)| *code).collect();
    for spoken in [
        "en", "zh-Hans", "hi", "es", "ar", "fr", "bn", "pt-BR", "id", "ur", "ru", "de", "ja", "pcm",
    ] {
        assert!(codes.contains(&spoken), "{spoken}");
    }
    let mut every: Vec<String> = natural_languages()
        .iter()
        .chain(constructed_languages())
        .map(|(code, _)| code.to_ascii_lowercase())
        .collect();
    let count = every.len();
    every.sort();
    every.dedup();
    assert_eq!(every.len(), count, "no code is listed twice");
    for (code, name) in natural_languages() {
        assert!(valid_language_tag(code) && !constructed_language(code), "{code}");
        assert_eq!(language_name(code), Some(*name));
    }
}

/// The `[code, name, ...]` entries of the list `name` in the editor's `languages.js`.
fn editor_list(name: &str) -> Vec<(String, String)> {
    let script = include_str!("../../../web/editor/languages.js");
    let start = script
        .find(&format!("const {name} = ["))
        .expect("languages.js has the list");
    let list = &script[start..];
    let list = &list[..list.find("\n  ];").expect("the list ends")];
    list.lines()
        .filter_map(|line| line.trim().strip_prefix("[\""))
        .map(|entry| {
            let mut fields = entry.split("\", \"");
            let code = fields.next().unwrap().to_owned();
            (code, fields.next().unwrap().to_owned())
        })
        .collect()
}

#[test]
fn the_editor_lists_the_same_languages_by_the_same_names() {
    let owned = |list: &[(&str, &str)]| -> Vec<(String, String)> {
        list.iter()
            .map(|(code, name)| ((*code).to_owned(), (*name).to_owned()))
            .collect()
    };
    assert_eq!(editor_list("NATURAL"), owned(natural_languages()));
    assert_eq!(editor_list("CONSTRUCTED"), owned(constructed_languages()));
    let split = editor_list("SPLIT");
    assert!(!split.is_empty());
    for (code, name) in split {
        assert_eq!(
            language_name(&code),
            Some(name.as_str()),
            "{code} is named, though not listed"
        );
    }
}
