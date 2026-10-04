// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises Tengwar in the classical mode for Quenya: known words and phrases, each vowel over
//! the consonant before it, short, long, and initial vowels, diphthongs on yanta and úrë, doubled
//! consonants and clusters with tengwar of their own, the turned silmë and essë, and punctuation.

mod common;

use common::tengwar::{quenya, spelled};

#[test]
fn quenya_words_put_each_vowel_over_the_consonant_before_it() {
    let words = [
        ("lantar", "lambe+a anto+a ore"),
        ("lassi", "lambe+a esse_nuquerna+i"),
        ("aldaron", "short+a alda+a romen+o numen"),
        ("tintilar", "tinco+i anto+i lambe+a ore"),
        ("eleni", "short+e lambe+e numen+i"),
        ("Valar", "vala+a lambe+a ore"),
        ("ortanë", "short+o ore tinco+a numen+e"),
        ("enquantuva", "short+e unque+a anto+u vala+a"),
        ("undulávë", "short+u ando+u lambe long+a vala+e"),
        ("Vardo", "vala+a arda+o"),
        ("lumbulë", "lambe+u umbar+u lambe+e"),
        ("caita", "calma yanta+a tinco+a"),
        ("Quenya", "quesse+e numen+dots_below+a"),
    ];
    for (latin, expected) in words {
        assert_eq!(quenya(latin), spelled(expected), "{latin}");
    }
}

#[test]
fn quenya_phrases_keep_their_spaces_and_take_tengwar_punctuation() {
    assert_eq!(
        quenya("Elen síla lúmenn' omentielvo!"),
        spelled(
            "short+e lambe+e numen | silme long+i lambe+a | lambe long+u malta+e numen+double | \
             short+o malta+e anto+i short+e lambe vala+o exclamation"
        ),
    );
    assert_eq!(
        quenya("Ai! laurië lantar, Namárië."),
        spelled(
            "yanta+a exclamation | lambe ure+a romen+i short+e | lambe+a anto+a ore pusta | \
             numen+a malta long+a romen+i short+e double_pusta"
        ),
    );
    assert_eq!(
        quenya("Mára arin?"),
        spelled("malta long+a romen+a | short+a romen+i numen question")
    );
}

#[test]
fn quenya_vowels_long_vowels_and_initial_vowels() {
    for (vowel, tehta) in [("a", "a"), ("e", "e"), ("i", "i"), ("o", "o"), ("u", "u")] {
        assert_eq!(
            quenya(&format!("t{vowel}")),
            spelled(&format!("tinco+{tehta}")),
            "t{vowel}"
        );
        assert_eq!(quenya(vowel), spelled(&format!("short+{tehta}")), "{vowel} alone");
    }
    for (vowel, tehta) in [("á", "a"), ("é", "e"), ("í", "i"), ("ó", "o"), ("ú", "u")] {
        assert_eq!(
            quenya(&format!("t{vowel}")),
            spelled(&format!("tinco long+{tehta}")),
            "t{vowel}"
        );
    }
    assert_eq!(quenya("únótimë"), spelled("long+u numen long+o tinco+i malta+e"));
    assert_eq!(quenya("ómaryo"), spelled("long+o malta+a romen+dots_below+o"));
    assert_eq!(
        quenya("Eärendil"),
        spelled("short+e short+a romen+e ando+i lambe"),
        "a diaeresis parts vowels"
    );
}

#[test]
fn quenya_diphthongs_sit_on_yanta_and_ure() {
    let diphthongs = [
        ("lai", "lambe yanta+a"),
        ("loi", "lambe yanta+o"),
        ("lui", "lambe yanta+u"),
        ("lau", "lambe ure+a"),
        ("leu", "lambe ure+e"),
        ("liu", "lambe ure+i"),
        ("lia", "lambe+i short+a"),
        ("laï", "lambe+a short+i"),
        ("Oiolossëo", "yanta+o short+o lambe+o esse_nuquerna+e short+o"),
        ("Aiya", "yanta+a anna+dots_below+a"),
    ];
    for (latin, expected) in diphthongs {
        assert_eq!(quenya(latin), spelled(expected), "{latin}");
    }
}

#[test]
fn quenya_consonants_double_cluster_and_change_with_their_place() {
    let words = [
        ("pella", "parma+e lambe+double+a"),
        ("anna", "short+a numen+double+a"),
        ("atta", "short+a tinco+double+a"),
        ("essë", "short+e esse_nuquerna+e"),
        ("ess", "short+e esse"),
        ("sí", "silme long+i"),
        ("sindarin", "silme_nuquerna+i ando+a romen+i numen"),
        ("hísië", "hyarmen long+i silme_nuquerna+i short+e"),
        ("ohta", "short+o harma tinco+a"),
        ("hyarmen", "hyarmen+dots_below+a ore malta+e numen"),
        ("hlóna", "halla lambe long+o numen+a"),
        ("hwesta", "hwesta+e silme tinco+a"),
        ("Ñoldor", "noldo+o alda+o ore"),
        ("Ngoldo", "noldo+o alda+o"),
        ("anga", "short+a anga+a"),
        ("ungwë", "short+u ungwe+e"),
        ("nwalmë", "nwalme+a lambe malta+e"),
        ("ampa", "short+a ampa+a"),
        ("anca", "short+a anca+a"),
        ("thúlë", "thule long+u lambe+e"),
        ("tyelpë", "tinco+dots_below+e lambe parma+e"),
        ("yulma", "anna+dots_below+u lambe malta+a"),
        ("wilya", "vilya+i lambe+dots_below+a"),
        ("Helcaraxë", "hyarmen+e lambe calma+a romen+a calma silme_nuquerna+e"),
    ];
    for (latin, expected) in words {
        assert_eq!(quenya(latin), spelled(expected), "{latin}");
    }
}
