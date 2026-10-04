// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Language names: what the languages subtitles may be translated into are called in English, so
//! a translation request names each one in words, and which of them are constructed or fictional.
//! The editor lists the same languages by the same names, in `web/editor/languages.js`.

use super::long_codes::short_code;

/// Natural languages by BCP 47 code, in the order of their names: every language with 50 million
/// speakers or more and many with tens of millions, every language ElevenLabs Scribe v2 Realtime
/// transcribes, and others often asked for.
const NATURAL: &[(&str, &str)] = &[
    ("af", "Afrikaans"),
    ("sq", "Albanian"),
    ("am", "Amharic"),
    ("ar", "Arabic"),
    ("hy", "Armenian"),
    ("as", "Assamese"),
    ("ast", "Asturian"),
    ("az", "Azerbaijani"),
    ("eu", "Basque"),
    ("be", "Belarusian"),
    ("bn", "Bengali"),
    ("bho", "Bhojpuri"),
    ("bs", "Bosnian"),
    ("br", "Breton"),
    ("bg", "Bulgarian"),
    ("my", "Burmese"),
    ("yue", "Cantonese"),
    ("ca", "Catalan"),
    ("ceb", "Cebuano"),
    ("ckb", "Central Kurdish (Sorani)"),
    ("ny", "Chichewa"),
    ("zh-Hans", "Chinese (Simplified)"),
    ("zh-Hant", "Chinese (Traditional)"),
    ("co", "Corsican"),
    ("hr", "Croatian"),
    ("cs", "Czech"),
    ("da", "Danish"),
    ("nl", "Dutch"),
    ("arz", "Egyptian Arabic"),
    ("en", "English"),
    ("et", "Estonian"),
    ("fil", "Filipino"),
    ("fi", "Finnish"),
    ("fr", "French"),
    ("fr-CA", "French (Canada)"),
    ("ff", "Fulah"),
    ("gl", "Galician"),
    ("lg", "Ganda"),
    ("ka", "Georgian"),
    ("de", "German"),
    ("el", "Greek"),
    ("gn", "Guarani"),
    ("gu", "Gujarati"),
    ("ht", "Haitian Creole"),
    ("hak", "Hakka Chinese"),
    ("ha", "Hausa"),
    ("haw", "Hawaiian"),
    ("he", "Hebrew"),
    ("hi", "Hindi"),
    ("hu", "Hungarian"),
    ("is", "Icelandic"),
    ("ig", "Igbo"),
    ("id", "Indonesian"),
    ("ga", "Irish"),
    ("it", "Italian"),
    ("ja", "Japanese"),
    ("jv", "Javanese"),
    ("kea", "Kabuverdianu"),
    ("kn", "Kannada"),
    ("kk", "Kazakh"),
    ("km", "Khmer"),
    ("rw", "Kinyarwanda"),
    ("ko", "Korean"),
    ("ku", "Kurdish"),
    ("ky", "Kyrgyz"),
    ("lo", "Lao"),
    ("la", "Latin"),
    ("lv", "Latvian"),
    ("apc", "Levantine Arabic"),
    ("ln", "Lingala"),
    ("lt", "Lithuanian"),
    ("luo", "Luo"),
    ("lb", "Luxembourgish"),
    ("mk", "Macedonian"),
    ("mai", "Maithili"),
    ("mg", "Malagasy"),
    ("ms", "Malay"),
    ("ml", "Malayalam"),
    ("mt", "Maltese"),
    ("mi", "Māori"),
    ("mr", "Marathi"),
    ("mn", "Mongolian"),
    ("ne", "Nepali"),
    ("pcm", "Nigerian Pidgin"),
    ("nso", "Northern Sotho"),
    ("no", "Norwegian"),
    ("oc", "Occitan"),
    ("or", "Odia"),
    ("om", "Oromo"),
    ("ps", "Pashto"),
    ("fa", "Persian"),
    ("pl", "Polish"),
    ("pt-BR", "Portuguese (Brazil)"),
    ("pt-PT", "Portuguese (Portugal)"),
    ("pa", "Punjabi"),
    ("qu", "Quechua"),
    ("ro", "Romanian"),
    ("ru", "Russian"),
    ("sa", "Sanskrit"),
    ("gd", "Scottish Gaelic"),
    ("sr", "Serbian"),
    ("sr-Latn", "Serbian (Latin)"),
    ("sn", "Shona"),
    ("sd", "Sindhi"),
    ("si", "Sinhala"),
    ("sk", "Slovak"),
    ("sl", "Slovenian"),
    ("so", "Somali"),
    ("es", "Spanish"),
    ("es-419", "Spanish (Latin America)"),
    ("apd", "Sudanese Arabic"),
    ("su", "Sundanese"),
    ("sw", "Swahili"),
    ("sv", "Swedish"),
    ("tl", "Tagalog"),
    ("tg", "Tajik"),
    ("ta", "Tamil"),
    ("tt", "Tatar"),
    ("te", "Telugu"),
    ("th", "Thai"),
    ("bo", "Tibetan"),
    ("ti", "Tigrinya"),
    ("tr", "Turkish"),
    ("tk", "Turkmen"),
    ("uk", "Ukrainian"),
    ("umb", "Umbundu"),
    ("ur", "Urdu"),
    ("ug", "Uyghur"),
    ("uz", "Uzbek"),
    ("vi", "Vietnamese"),
    ("cy", "Welsh"),
    ("fy", "Western Frisian"),
    ("pnb", "Western Punjabi"),
    ("wo", "Wolof"),
    ("wuu", "Wu Chinese"),
    ("xh", "Xhosa"),
    ("yi", "Yiddish"),
    ("yo", "Yoruba"),
    ("zu", "Zulu"),
];

/// Constructed and fictional languages, in the order of their names: by their ISO 639 code where
/// the ISO 639-3 registry has one, and otherwise by a private-use tag under `art`, the code for
/// artificial languages as a group.
const CONSTRUCTED: &[(&str, &str)] = &[
    ("art-x-dothraki", "Dothraki"),
    ("eo", "Esperanto"),
    ("art-x-valyrian", "High Valyrian"),
    ("io", "Ido"),
    ("ia", "Interlingua"),
    ("ie", "Interlingue (Occidental)"),
    ("tlh", "Klingon"),
    ("ldn", "Láadan"),
    ("jbo", "Lojban"),
    ("art-x-navi", "Na'vi"),
    ("nov", "Novial"),
    ("qya", "Quenya (Elvish)"),
    ("sjn", "Sindarin (Elvish)"),
    ("tok", "Toki Pona"),
    ("vo", "Volapük"),
];

/// Languages the natural list offers only by script or country, named as a whole for their other
/// codes, such as `zh-TW` or `pt-AO`; the editor names them too, and offers none of them.
const SPLIT: &[(&str, &str)] = &[("zh", "Chinese"), ("pt", "Portuguese")];

/// The code for artificial languages as a group, which private-use tags for languages without a
/// code of their own start with.
const ARTIFICIAL: &str = "art";

/// The natural languages by code and name, in the order of their names.
pub fn natural_languages() -> &'static [(&'static str, &'static str)] {
    NATURAL
}

/// The constructed and fictional languages by code and name, in the order of their names.
pub fn constructed_languages() -> &'static [(&'static str, &'static str)] {
    CONSTRUCTED
}

/// The English name of the language `code` names, matched without regard to case: by the whole
/// code, or else by what is left as its last subtag is dropped, one at a time, so `es-MX` is
/// Spanish, `sr-Latn-RS` Serbian (Latin), and `zh-TW` Chinese; a three-letter code of a language
/// listed by its two-letter one, as speech to text reports it, names that language, so `eng` is
/// English and `cmn` Chinese. `None` for a language not listed.
pub fn language_name(code: &str) -> Option<&'static str> {
    let listed = |wanted: &str| {
        let mut every = NATURAL.iter().chain(CONSTRUCTED).chain(SPLIT);
        every
            .find(|(listed, _)| listed.eq_ignore_ascii_case(wanted))
            .map(|(_, name)| *name)
    };
    let mut wanted = code.trim();
    loop {
        if let Some(name) = listed(wanted) {
            return Some(name);
        }
        match wanted.rsplit_once('-') {
            Some((shorter, _)) => wanted = shorter,
            None => return short_code(wanted).and_then(listed),
        }
    }
}

/// Whether `code` names a constructed or fictional language: one listed as such, by the whole code
/// or its language subtag, or any tag under `art`, such as `art-x-dothraki`.
pub fn constructed_language(code: &str) -> bool {
    let code = code.trim();
    let primary = code.split('-').next().unwrap_or_default();
    let listed = |wanted: &str| {
        CONSTRUCTED
            .iter()
            .any(|(listed, _)| listed.eq_ignore_ascii_case(wanted))
    };
    primary.eq_ignore_ascii_case(ARTIFICIAL) || listed(code) || listed(primary)
}

/// The language `code` names, in words with its code, as `Spanish (es)` or `Quenya (Elvish, qya)`;
/// the code alone when its name is not known.
pub fn language_label(code: &str) -> String {
    let code = code.trim();
    match language_name(code) {
        Some(name) if name.ends_with(')') => format!("{}, {code})", &name[..name.len() - 1]),
        Some(name) => format!("{name} ({code})"),
        None => code.to_owned(),
    }
}
