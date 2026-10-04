// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises Toki Pona in sitelen pona: a glyph per word at its Under-ConScript code point, glyphs
//! written without spaces, names and words outside the dictionary spelled in a cartouche, a capital
//! at the start of a sentence, words with other letters, and punctuation.

use sceneuron_core::NativeScript;

fn sitelen(text: &str) -> String {
    NativeScript::SitelenPona.write(text)
}

fn glyphs(codes: &[u32]) -> String {
    codes.iter().map(|&code| char::from_u32(code).unwrap()).collect()
}

const START: u32 = 0xF1990;
const END: u32 = 0xF1991;

#[test]
fn each_word_is_its_glyph_and_glyphs_take_no_spaces() {
    assert_eq!(sitelen("toki pona"), glyphs(&[0xF196C, 0xF1954]));
    assert_eq!(sitelen("mi moku"), glyphs(&[0xF1934, 0xF1936]));
    assert_eq!(sitelen("a"), glyphs(&[0xF1900]));
    assert_eq!(
        sitelen("ku"),
        glyphs(&[0xF1988]),
        "the last word of the pu and ku suli list"
    );
    assert_eq!(sitelen("kijetesantakalu"), glyphs(&[0xF1980]));
    assert_eq!(
        sitelen("ali ale"),
        glyphs(&[0xF1904, 0xF1904]),
        "ali is another spelling of ale"
    );
    assert_eq!(sitelen("pake powe"), glyphs(&[0xF19A0, 0xF19A3]));
}

#[test]
fn names_and_words_outside_the_dictionary_are_spelled_in_a_cartouche() {
    // sona ona nena jan a: a glyph for each letter, from the word it starts.
    let sonja = glyphs(&[START, 0xF1961, 0xF1946, 0xF1940, 0xF1911, 0xF1900, END]);
    assert_eq!(sitelen("jan Sonja"), format!("{}{sonja}", glyphs(&[0xF1911])));
    assert_eq!(
        sitelen("ma Kanata"),
        format!(
            "{}{}",
            glyphs(&[0xF1930]),
            glyphs(&[START, 0xF1914, 0xF1900, 0xF1940, 0xF1900, 0xF196C, 0xF1900, END])
        ),
    );
    assert_eq!(
        sitelen("jan Pona"),
        format!(
            "{}{}",
            glyphs(&[0xF1911]),
            glyphs(&[START, 0xF1954, 0xF1946, 0xF1940, 0xF1900, END])
        ),
        "a capital mid-sentence makes a name"
    );
    assert_eq!(
        sitelen("wiwi"),
        glyphs(&[START, 0xF1974, 0xF190E, 0xF1974, 0xF190E, END])
    );
    let letters = [
        ('a', 0xF1900),
        ('e', 0xF190B),
        ('i', 0xF190E),
        ('j', 0xF1911),
        ('k', 0xF1914),
        ('l', 0xF192A),
        ('m', 0xF1931),
        ('n', 0xF1940),
        ('o', 0xF1946),
        ('p', 0xF1954),
        ('s', 0xF1961),
        ('t', 0xF196C),
        ('u', 0xF1970),
        ('w', 0xF1974),
    ];
    for (letter, code) in letters {
        let name = letter.to_uppercase().to_string();
        assert_eq!(
            sitelen(&format!("jan {name}")),
            glyphs(&[0xF1911, START, code, END]),
            "{letter}"
        );
    }
}

#[test]
fn a_capital_starting_a_sentence_is_still_the_word() {
    assert_eq!(sitelen("Mi moku."), glyphs(&[0xF1934, 0xF1936, 0xF199C]));
    assert_eq!(
        sitelen("toki! Sina pona"),
        format!("{}! {}", glyphs(&[0xF196C]), glyphs(&[0xF195E, 0xF1954]))
    );
    assert_eq!(
        sitelen("Sonja li pona"),
        format!(
            "{}{}",
            glyphs(&[START, 0xF1961, 0xF1946, 0xF1940, 0xF1911, 0xF1900, END]),
            glyphs(&[0xF1927, 0xF1954])
        )
    );
}

#[test]
fn other_letters_and_punctuation() {
    assert_eq!(
        sitelen("jan Bob"),
        format!("{} Bob", glyphs(&[0xF1911])),
        "b is not a letter of Toki Pona"
    );
    assert_eq!(
        sitelen("mi pona. sina pona: a"),
        glyphs(&[0xF1934, 0xF1954, 0xF199C, 0xF195E, 0xF1954, 0xF199D, 0xF1900])
    );
    assert_eq!(
        sitelen("suno pona, jan ale!"),
        format!("{}, {}!", glyphs(&[0xF1964, 0xF1954]), glyphs(&[0xF1911, 0xF1904]))
    );
    assert_eq!(sitelen("tu 2"), format!("{} 2", glyphs(&[0xF196E])), "digits stay");
    assert_eq!(sitelen(""), "");
}
