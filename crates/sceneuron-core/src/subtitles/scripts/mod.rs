// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Native scripts: a constructed language with a script of its own is translated in its usual
//! romanization and written in that script here, line by line, by fixed rules, for the overlay
//! only: Quenya and Sindarin in Tengwar, Klingon in pIqaD, and Toki Pona in sitelen pona. The
//! characters are the ones the Under-ConScript Unicode Registry assigns these scripts in the
//! Private Use Area, which the overlay's bundled fonts draw. Captions, the editor, and spoken
//! cards keep the romanization.

mod piqad;
mod sitelen_pona;
mod tengwar;

/// A script of its own that subtitles in a constructed language are written in on the overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeScript {
    /// Tengwar in the classical mode for Quenya: a vowel sits over the consonant before it.
    QuenyaTengwar,
    /// Tengwar in the general mode for Sindarin: a vowel sits over the consonant after it.
    SindarinTengwar,
    /// Klingon's pIqaD, a letter for each letter of the romanization.
    Piqad,
    /// Toki Pona's sitelen pona, a glyph for each word, and names spelled in a cartouche.
    SitelenPona,
}

/// Languages with a script of their own, by language subtag.
const LANGUAGES: &[(&str, NativeScript)] = &[
    ("qya", NativeScript::QuenyaTengwar),
    ("sjn", NativeScript::SindarinTengwar),
    ("tlh", NativeScript::Piqad),
    ("tok", NativeScript::SitelenPona),
];

impl NativeScript {
    /// The script the language `code` names is written in, by its language subtag without regard
    /// to case, so `tlh` and `TLH-x-test` are Klingon; `None` for a language written in Latin
    /// letters or in a script of its own that Unicode already has, such as Japanese.
    pub fn of(code: &str) -> Option<NativeScript> {
        let language = code.trim().split('-').next().unwrap_or_default();
        LANGUAGES
            .iter()
            .find(|(listed, _)| listed.eq_ignore_ascii_case(language))
            .map(|(_, script)| *script)
    }

    /// How the overlay names the script, for its font and look.
    pub fn id(self) -> &'static str {
        match self {
            NativeScript::QuenyaTengwar | NativeScript::SindarinTengwar => "tengwar",
            NativeScript::Piqad => "piqad",
            NativeScript::SitelenPona => "sitelen-pona",
        }
    }

    /// The script's name in words.
    pub fn name(self) -> &'static str {
        match self {
            NativeScript::QuenyaTengwar | NativeScript::SindarinTengwar => "Tengwar",
            NativeScript::Piqad => "pIqaD",
            NativeScript::SitelenPona => "sitelen pona",
        }
    }

    /// `text`, in the language's romanization, written in this script. Spaces stay, and what the
    /// script has no letter for, such as digits and unknown letters, stays as it is.
    pub fn write(self, text: &str) -> String {
        match self {
            NativeScript::QuenyaTengwar => tengwar::write(text, tengwar::Mode::Quenya),
            NativeScript::SindarinTengwar => tengwar::write(text, tengwar::Mode::Sindarin),
            NativeScript::Piqad => piqad::write(text),
            NativeScript::SitelenPona => sitelen_pona::write(text),
        }
    }
}
