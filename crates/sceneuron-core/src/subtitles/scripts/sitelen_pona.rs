// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Toki Pona in sitelen pona: each word of the dictionary is one glyph, from U+F1900, written
//! without spaces between glyphs, as sitelen pona is. A name, which Toki Pona capitalizes, and any
//! other word made of Toki Pona's letters is spelled in a cartouche, a glyph for each letter whose
//! word starts with it, as in `jan [sona ona nena jan a]` for jan Sonja. A word with other letters
//! stays in Latin letters. A full stop and a colon have their own signs; other punctuation stays.

/// Every word sitelen pona has a glyph for, by its code point; `ali` is another spelling of `ale`.
const WORDS: &[(&str, u32)] = &[
    ("a", 0xF1900),
    ("akesi", 0xF1901),
    ("ala", 0xF1902),
    ("alasa", 0xF1903),
    ("ale", 0xF1904),
    ("ali", 0xF1904),
    ("anpa", 0xF1905),
    ("ante", 0xF1906),
    ("anu", 0xF1907),
    ("awen", 0xF1908),
    ("e", 0xF1909),
    ("en", 0xF190A),
    ("esun", 0xF190B),
    ("ijo", 0xF190C),
    ("ike", 0xF190D),
    ("ilo", 0xF190E),
    ("insa", 0xF190F),
    ("jaki", 0xF1910),
    ("jan", 0xF1911),
    ("jelo", 0xF1912),
    ("jo", 0xF1913),
    ("kala", 0xF1914),
    ("kalama", 0xF1915),
    ("kama", 0xF1916),
    ("kasi", 0xF1917),
    ("ken", 0xF1918),
    ("kepeken", 0xF1919),
    ("kili", 0xF191A),
    ("kiwen", 0xF191B),
    ("ko", 0xF191C),
    ("kon", 0xF191D),
    ("kule", 0xF191E),
    ("kulupu", 0xF191F),
    ("kute", 0xF1920),
    ("la", 0xF1921),
    ("lape", 0xF1922),
    ("laso", 0xF1923),
    ("lawa", 0xF1924),
    ("len", 0xF1925),
    ("lete", 0xF1926),
    ("li", 0xF1927),
    ("lili", 0xF1928),
    ("linja", 0xF1929),
    ("lipu", 0xF192A),
    ("loje", 0xF192B),
    ("lon", 0xF192C),
    ("luka", 0xF192D),
    ("lukin", 0xF192E),
    ("lupa", 0xF192F),
    ("ma", 0xF1930),
    ("mama", 0xF1931),
    ("mani", 0xF1932),
    ("meli", 0xF1933),
    ("mi", 0xF1934),
    ("mije", 0xF1935),
    ("moku", 0xF1936),
    ("moli", 0xF1937),
    ("monsi", 0xF1938),
    ("mu", 0xF1939),
    ("mun", 0xF193A),
    ("musi", 0xF193B),
    ("mute", 0xF193C),
    ("nanpa", 0xF193D),
    ("nasa", 0xF193E),
    ("nasin", 0xF193F),
    ("nena", 0xF1940),
    ("ni", 0xF1941),
    ("nimi", 0xF1942),
    ("noka", 0xF1943),
    ("o", 0xF1944),
    ("olin", 0xF1945),
    ("ona", 0xF1946),
    ("open", 0xF1947),
    ("pakala", 0xF1948),
    ("pali", 0xF1949),
    ("palisa", 0xF194A),
    ("pan", 0xF194B),
    ("pana", 0xF194C),
    ("pi", 0xF194D),
    ("pilin", 0xF194E),
    ("pimeja", 0xF194F),
    ("pini", 0xF1950),
    ("pipi", 0xF1951),
    ("poka", 0xF1952),
    ("poki", 0xF1953),
    ("pona", 0xF1954),
    ("pu", 0xF1955),
    ("sama", 0xF1956),
    ("seli", 0xF1957),
    ("selo", 0xF1958),
    ("seme", 0xF1959),
    ("sewi", 0xF195A),
    ("sijelo", 0xF195B),
    ("sike", 0xF195C),
    ("sin", 0xF195D),
    ("sina", 0xF195E),
    ("sinpin", 0xF195F),
    ("sitelen", 0xF1960),
    ("sona", 0xF1961),
    ("soweli", 0xF1962),
    ("suli", 0xF1963),
    ("suno", 0xF1964),
    ("supa", 0xF1965),
    ("suwi", 0xF1966),
    ("tan", 0xF1967),
    ("taso", 0xF1968),
    ("tawa", 0xF1969),
    ("telo", 0xF196A),
    ("tenpo", 0xF196B),
    ("toki", 0xF196C),
    ("tomo", 0xF196D),
    ("tu", 0xF196E),
    ("unpa", 0xF196F),
    ("uta", 0xF1970),
    ("utala", 0xF1971),
    ("walo", 0xF1972),
    ("wan", 0xF1973),
    ("waso", 0xF1974),
    ("wawa", 0xF1975),
    ("weka", 0xF1976),
    ("wile", 0xF1977),
    ("namako", 0xF1978),
    ("kin", 0xF1979),
    ("oko", 0xF197A),
    ("kipisi", 0xF197B),
    ("leko", 0xF197C),
    ("monsuta", 0xF197D),
    ("tonsi", 0xF197E),
    ("jasima", 0xF197F),
    ("kijetesantakalu", 0xF1980),
    ("soko", 0xF1981),
    ("meso", 0xF1982),
    ("epiku", 0xF1983),
    ("kokosila", 0xF1984),
    ("lanpan", 0xF1985),
    ("n", 0xF1986),
    ("misikeke", 0xF1987),
    ("ku", 0xF1988),
    ("pake", 0xF19A0),
    ("apeja", 0xF19A1),
    ("majuna", 0xF19A2),
    ("powe", 0xF19A3),
];

/// The word whose glyph spells each of Toki Pona's fourteen letters in a cartouche.
const LETTERS: &[(char, &str)] = &[
    ('a', "a"),
    ('e', "esun"),
    ('i', "ilo"),
    ('j', "jan"),
    ('k', "kala"),
    ('l', "lipu"),
    ('m', "mama"),
    ('n', "nena"),
    ('o', "ona"),
    ('p', "pona"),
    ('s', "sona"),
    ('t', "toki"),
    ('u', "uta"),
    ('w', "waso"),
];
const START_OF_CARTOUCHE: char = '\u{F1990}';
const END_OF_CARTOUCHE: char = '\u{F1991}';
const MIDDLE_DOT: char = '\u{F199C}';
const COLON: char = '\u{F199D}';

/// `text` in sitelen pona. A capital at the start of a sentence, which Toki Pona does not use but
/// translations often do, still reads as the word it starts.
pub(super) fn write(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    // Whether the last thing written is a glyph, which the next glyph follows without a space.
    let mut after_glyph = false;
    let mut pending_space = false;
    let mut sentence_start = true;
    let mut rest = text;
    while let Some(first) = rest.chars().next() {
        let length = rest
            .find(|character: char| !character.is_alphabetic())
            .unwrap_or(rest.len())
            .max(first.len_utf8());
        let (piece, after) = rest.split_at(length);
        rest = after;
        if first.is_whitespace() {
            pending_space = true;
            continue;
        }
        let written = if first.is_alphabetic() {
            glyphs(piece, sentence_start)
        } else {
            None
        };
        let glyph = written.is_some() || matches!(first, '.' | ':');
        if pending_space && !(glyph && after_glyph) {
            out.push(' ');
        }
        pending_space = false;
        match (written, first) {
            (Some(glyphs), _) => out.push_str(&glyphs),
            (None, '.') => out.push(MIDDLE_DOT),
            (None, ':') => out.push(COLON),
            (None, _) => out.push_str(piece),
        }
        after_glyph = glyph;
        sentence_start = matches!(first, '.' | ':' | '!' | '?') || (sentence_start && !first.is_alphabetic());
    }
    out
}

/// A word's glyph, or its letters spelled in a cartouche when it is a name or not in the
/// dictionary; `None` for a word with letters Toki Pona does not have. A capital makes a name,
/// except at `sentence_start` on a word of the dictionary.
fn glyphs(word: &str, sentence_start: bool) -> Option<String> {
    let lower = word.to_lowercase();
    let named = word.chars().next().is_some_and(char::is_uppercase) && !sentence_start;
    if !named && let Some(glyph) = glyph(&lower) {
        return Some(glyph.to_string());
    }
    let spelled: Option<String> = lower
        .chars()
        .map(|letter| {
            let (_, word) = LETTERS.iter().find(|(listed, _)| *listed == letter)?;
            glyph(word)
        })
        .collect();
    spelled.map(|letters| format!("{START_OF_CARTOUCHE}{letters}{END_OF_CARTOUCHE}"))
}

fn glyph(word: &str) -> Option<char> {
    let (_, code) = WORDS.iter().find(|(listed, _)| *listed == word)?;
    char::from_u32(*code)
}
