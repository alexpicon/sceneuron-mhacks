// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Tengwar fixtures: expected text spelled by tengwa names, as `lambe+a` for lambë with the a
//! tehta and `|` for a space, and the two modes' writers.

use sceneuron_core::NativeScript;

/// The text `spelling` names: tengwar and signs by name, each followed by its marks after `+`.
pub fn spelled(spelling: &str) -> String {
    let code = |name: &str| -> u32 {
        match name {
            "tinco" => 0xE000,
            "parma" => 0xE001,
            "calma" => 0xE002,
            "quesse" => 0xE003,
            "ando" => 0xE004,
            "umbar" => 0xE005,
            "anga" => 0xE006,
            "ungwe" => 0xE007,
            "thule" => 0xE008,
            "formen" => 0xE009,
            "harma" => 0xE00A,
            "hwesta" => 0xE00B,
            "anto" => 0xE00C,
            "ampa" => 0xE00D,
            "anca" => 0xE00E,
            "unque" => 0xE00F,
            "numen" => 0xE010,
            "malta" => 0xE011,
            "noldo" => 0xE012,
            "nwalme" => 0xE013,
            "ore" => 0xE014,
            "vala" => 0xE015,
            "anna" => 0xE016,
            "vilya" => 0xE017,
            "romen" => 0xE020,
            "arda" => 0xE021,
            "lambe" => 0xE022,
            "alda" => 0xE023,
            "silme" => 0xE024,
            "silme_nuquerna" => 0xE025,
            "esse" => 0xE026,
            "esse_nuquerna" => 0xE027,
            "hyarmen" => 0xE028,
            "hwesta_sindarinwa" => 0xE029,
            "yanta" => 0xE02A,
            "ure" => 0xE02B,
            "long" => 0xE02C,
            "halla" => 0xE02D,
            "short" => 0xE02E,
            "a" => 0xE040,
            "y" => 0xE042,
            "dots_below" => 0xE043,
            "i" => 0xE044,
            "e" => 0xE046,
            "o" => 0xE04A,
            "u" => 0xE04C,
            "nasal" => 0xE050,
            "double" => 0xE051,
            "pusta" => 0xE060,
            "double_pusta" => 0xE061,
            "exclamation" => 0xE065,
            "question" => 0xE066,
            "|" => 0x20,
            other => panic!("no tengwa is named {other}"),
        }
    };
    spelling
        .split_whitespace()
        .flat_map(|tengwa| tengwa.split('+'))
        .map(|name| char::from_u32(code(name)).unwrap())
        .collect()
}

pub fn quenya(text: &str) -> String {
    NativeScript::QuenyaTengwar.write(text)
}

pub fn sindarin(text: &str) -> String {
    NativeScript::SindarinTengwar.write(text)
}
