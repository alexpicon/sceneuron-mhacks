// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises Tengwar in the general mode for Sindarin, each vowel over the consonant after it:
//! known words and phrases, carriers, long vowels and diphthongs, nasalized consonants and
//! fricatives; and in either mode what Tengwar cannot show, capitals, apostrophes, and hyphens.

mod common;

use common::tengwar::{quenya, sindarin, spelled};

#[test]
fn sindarin_words_put_each_vowel_over_the_consonant_after_it() {
    // The encoding examples of the ConScript Unicode Registry's Tengwar page: neled, nelthil.
    assert_eq!(sindarin("neled"), spelled("numen lambe+e ando+e"));
    assert_eq!(sindarin("nelthil"), spelled("numen lambe+e thule lambe+i"));
    let words = [
        ("Elbereth", "lambe+e umbar romen+e thule+e"),
        ("Gilthoniel", "ungwe lambe+i thule numen+o short+i lambe+e"),
        ("mellon", "malta lambe+double+e numen+o"),
        ("Aragorn", "romen+a ungwe+a ore+o numen"),
        ("Ithil", "thule+i lambe+i"),
        ("Anor", "numen+a ore+o"),
        ("Gondor", "ungwe ando+nasal+o ore+o"),
        ("Angband", "ungwe+nasal+a umbar ando+nasal+a"),
        ("galadh", "ungwe lambe+a anto+a"),
        ("Rohan", "romen hyarmen+o numen+a"),
        ("lhûn", "alda long+u numen"),
        ("Rhovanion", "arda ampa+o numen+a short+i numen+o"),
        ("ang", "nwalme+a"),
        ("Lothlórien", "lambe thule+o lambe long+o romen short+i numen+e"),
    ];
    for (latin, expected) in words {
        assert_eq!(sindarin(latin), spelled(expected), "{latin}");
    }
}

#[test]
fn sindarin_vowels_carriers_and_diphthongs() {
    assert_eq!(sindarin("le"), spelled("lambe short+e"), "a last vowel takes a carrier");
    assert_eq!(sindarin("mîr"), spelled("malta long+i ore"), "a circumflex is long");
    assert_eq!(sindarin("mír"), spelled("malta long+i ore"), "so is an acute");
    assert_eq!(
        sindarin("ylf"),
        spelled("lambe+y ampa"),
        "y is a vowel, and a last f is said v"
    );
    assert_eq!(sindarin("mellyn"), spelled("malta lambe+double+e numen+y"));
    let diphthongs = [
        ("mae", "malta yanta+a"),
        ("goe", "ungwe yanta+o"),
        ("rain", "romen yanta+a numen"),
        ("ei", "yanta+e"),
        ("ui", "yanta+u"),
        ("naur", "numen ure+a ore"),
        ("naw", "numen ure+a"),
        ("awa", "vala+a short+a"),
    ];
    for (latin, expected) in diphthongs {
        assert_eq!(sindarin(latin), spelled(expected), "{latin}");
    }
}

#[test]
fn sindarin_phrases_and_their_fricatives() {
    assert_eq!(
        sindarin("Mae govannen, mellon!"),
        spelled(
            "malta yanta+a | ungwe ampa+o numen+double+a numen+e pusta | malta lambe+double+e numen+o \
             exclamation"
        ),
    );
    assert_eq!(
        sindarin("Le hannon."),
        spelled("lambe short+e | hyarmen numen+double+a numen+o double_pusta")
    );
    let words = [
        ("ech", "hwesta+e"),
        ("hwest", "hwesta_sindarinwa silme_nuquerna+e tinco"),
        ("pheriannath", "formen romen+e short+i numen+double+a thule+a"),
        ("tegil", "tinco ungwe+e lambe+i"),
        ("cuio", "quesse yanta+u short+o"),
    ];
    for (latin, expected) in words {
        assert_eq!(sindarin(latin), spelled(expected), "{latin}");
    }
}

#[test]
fn what_tengwar_cannot_show_stays_and_word_joiners_drop() {
    assert_eq!(
        quenya("lisse-miruvóreva"),
        quenya("lissemiruvóreva"),
        "a hyphen inside a word"
    );
    assert_eq!(
        quenya("a - e"),
        format!("{} - {}", quenya("a"), quenya("e")),
        "a dash between words stays"
    );
    assert_eq!(quenya("lúmenn'"), quenya("lúmenn"), "an elision mark");
    assert_eq!(quenya("ALDA"), quenya("alda"), "Tengwar has no capitals");
    assert_eq!(quenya("tál 42"), format!("{} 42", quenya("tál")), "digits stay");
    assert_eq!(
        quenya("tál ß"),
        format!("{} ß", quenya("tál")),
        "a letter neither mode has stays"
    );
    assert_eq!(quenya(""), "");
    assert_eq!(sindarin("  "), "  ");
}
