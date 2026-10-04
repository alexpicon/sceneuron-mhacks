// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the languages subtitles know, natural ones and then constructed and fictional ones, by code and
   name, with what CEA-608 stream captions show of each, and the spoken languages ElevenLabs Scribe hears. The names
   match the server's (crates/sceneuron-core/src/subtitles/names.rs); language-text.js looks them up. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);

  /**
   * Natural languages, in the order of their names: [BCP 47 code, name, what CEA-608 captions show, a letter they leave
   * out]. Captions show "all" of a language, "some" (Latin script with letters CEA-608 lacks, which captions leave
   * out), or "none" (another script, so captions are blank).
   */
  const NATURAL = [
    ["af", "Afrikaans", "all"],
    ["sq", "Albanian", "all"],
    ["am", "Amharic", "none"],
    ["ar", "Arabic", "none"],
    ["hy", "Armenian", "none"],
    ["as", "Assamese", "none"],
    ["ast", "Asturian", "some", "ḥ"],
    ["az", "Azerbaijani", "some", "ə"],
    ["eu", "Basque", "all"],
    ["be", "Belarusian", "none"],
    ["bn", "Bengali", "none"],
    ["bho", "Bhojpuri", "none"],
    ["bs", "Bosnian", "some", "č"],
    ["br", "Breton", "all"],
    ["bg", "Bulgarian", "none"],
    ["my", "Burmese", "none"],
    ["yue", "Cantonese", "none"],
    ["ca", "Catalan", "all"],
    ["ceb", "Cebuano", "all"],
    ["ckb", "Central Kurdish (Sorani)", "none"],
    ["ny", "Chichewa", "all"],
    ["zh-Hans", "Chinese (Simplified)", "none"],
    ["zh-Hant", "Chinese (Traditional)", "none"],
    ["co", "Corsican", "all"],
    ["hr", "Croatian", "some", "č"],
    ["cs", "Czech", "some", "ř"],
    ["da", "Danish", "some", "æ"],
    ["nl", "Dutch", "all"],
    ["arz", "Egyptian Arabic", "none"],
    ["en", "English", "all"],
    ["et", "Estonian", "some", "š"],
    ["fil", "Filipino", "all"],
    ["fi", "Finnish", "all"],
    ["fr", "French", "some", "œ"],
    ["fr-CA", "French (Canada)", "some", "œ"],
    ["ff", "Fulah", "some", "ɓ"],
    ["gl", "Galician", "all"],
    ["lg", "Ganda", "some", "ŋ"],
    ["ka", "Georgian", "none"],
    ["de", "German", "all"],
    ["el", "Greek", "none"],
    ["gn", "Guarani", "some", "ẽ"],
    ["gu", "Gujarati", "none"],
    ["ht", "Haitian Creole", "all"],
    ["hak", "Hakka Chinese", "none"],
    ["ha", "Hausa", "some", "ɓ"],
    ["haw", "Hawaiian", "some", "ā"],
    ["he", "Hebrew", "none"],
    ["hi", "Hindi", "none"],
    ["hu", "Hungarian", "some", "ő"],
    ["is", "Icelandic", "some", "þ"],
    ["ig", "Igbo", "some", "ọ"],
    ["id", "Indonesian", "all"],
    ["ga", "Irish", "all"],
    ["it", "Italian", "all"],
    ["ja", "Japanese", "none"],
    ["jv", "Javanese", "all"],
    ["kea", "Kabuverdianu", "all"],
    ["kn", "Kannada", "none"],
    ["kk", "Kazakh", "none"],
    ["km", "Khmer", "none"],
    ["rw", "Kinyarwanda", "all"],
    ["ko", "Korean", "none"],
    ["ku", "Kurdish", "some", "ş"],
    ["ky", "Kyrgyz", "none"],
    ["lo", "Lao", "none"],
    ["la", "Latin", "all"],
    ["lv", "Latvian", "some", "ā"],
    ["apc", "Levantine Arabic", "none"],
    ["ln", "Lingala", "some", "ɛ"],
    ["lt", "Lithuanian", "some", "ą"],
    ["luo", "Luo", "all"],
    ["lb", "Luxembourgish", "all"],
    ["mk", "Macedonian", "none"],
    ["mai", "Maithili", "none"],
    ["mg", "Malagasy", "all"],
    ["ms", "Malay", "all"],
    ["ml", "Malayalam", "none"],
    ["mt", "Maltese", "some", "ħ"],
    ["mi", "Māori", "some", "ā"],
    ["mr", "Marathi", "none"],
    ["mn", "Mongolian", "none"],
    ["ne", "Nepali", "none"],
    ["pcm", "Nigerian Pidgin", "all"],
    ["nso", "Northern Sotho", "some", "š"],
    ["no", "Norwegian", "some", "æ"],
    ["oc", "Occitan", "all"],
    ["or", "Odia", "none"],
    ["om", "Oromo", "all"],
    ["ps", "Pashto", "none"],
    ["fa", "Persian", "none"],
    ["pl", "Polish", "some", "ł"],
    ["pt-BR", "Portuguese (Brazil)", "all"],
    ["pt-PT", "Portuguese (Portugal)", "all"],
    ["pa", "Punjabi", "none"],
    ["qu", "Quechua", "all"],
    ["ro", "Romanian", "some", "ș"],
    ["ru", "Russian", "none"],
    ["sa", "Sanskrit", "none"],
    ["gd", "Scottish Gaelic", "all"],
    ["sr", "Serbian", "none"],
    ["sr-Latn", "Serbian (Latin)", "some", "č"],
    ["sn", "Shona", "all"],
    ["sd", "Sindhi", "none"],
    ["si", "Sinhala", "none"],
    ["sk", "Slovak", "some", "č"],
    ["sl", "Slovenian", "some", "č"],
    ["so", "Somali", "all"],
    ["es", "Spanish", "all"],
    ["es-419", "Spanish (Latin America)", "all"],
    ["apd", "Sudanese Arabic", "none"],
    ["su", "Sundanese", "all"],
    ["sw", "Swahili", "all"],
    ["sv", "Swedish", "all"],
    ["tl", "Tagalog", "all"],
    ["tg", "Tajik", "none"],
    ["ta", "Tamil", "none"],
    ["tt", "Tatar", "none"],
    ["te", "Telugu", "none"],
    ["th", "Thai", "none"],
    ["bo", "Tibetan", "none"],
    ["ti", "Tigrinya", "none"],
    ["tr", "Turkish", "some", "ş"],
    ["tk", "Turkmen", "some", "ň"],
    ["uk", "Ukrainian", "none"],
    ["umb", "Umbundu", "all"],
    ["ur", "Urdu", "none"],
    ["ug", "Uyghur", "none"],
    ["uz", "Uzbek", "some", "oʻ"],
    ["vi", "Vietnamese", "some", "ơ"],
    ["cy", "Welsh", "some", "ŵ"],
    ["fy", "Western Frisian", "all"],
    ["pnb", "Western Punjabi", "none"],
    ["wo", "Wolof", "some", "ŋ"],
    ["wuu", "Wu Chinese", "none"],
    ["xh", "Xhosa", "all"],
    ["yi", "Yiddish", "none"],
    ["yo", "Yoruba", "some", "ẹ"],
    ["zu", "Zulu", "all"],
  ];

  /**
   * Constructed and fictional languages, in the order of their names, with what captions show of their translations,
   * which are written in Latin script, the romanization the overlay writes those in SCRIPTS from. Their codes are ISO
   * 639 codes where the ISO 639-3 registry has one, and otherwise private-use tags under "art", the code for
   * artificial languages as a group.
   */
  const CONSTRUCTED = [
    ["art-x-dothraki", "Dothraki", "all"],
    ["eo", "Esperanto", "some", "ŝ"],
    ["art-x-valyrian", "High Valyrian", "some", "ā"],
    ["io", "Ido", "all"],
    ["ia", "Interlingua", "all"],
    ["ie", "Interlingue (Occidental)", "all"],
    ["tlh", "Klingon", "all"],
    ["ldn", "Láadan", "all"],
    ["jbo", "Lojban", "all"],
    ["art-x-navi", "Na'vi", "all"],
    ["nov", "Novial", "all"],
    ["qya", "Quenya (Elvish)", "all"],
    ["sjn", "Sindarin (Elvish)", "some", "ŷ"],
    ["tok", "Toki Pona", "all"],
    ["vo", "Volapük", "all"],
  ];

  /**
   * The constructed languages the overlay writes in a script of their own from their romanization, by language
   * subtag, with the script's name (crates/sceneuron-core/src/subtitles/scripts/mod.rs). Captions and the editor
   * keep the romanization.
   */
  const SCRIPTS = { qya: "Tengwar", sjn: "Tengwar", tlh: "pIqaD", tok: "sitelen pona" };

  /**
   * Languages the natural list offers only by script or country, named as a whole for their other codes, such as
   * zh-TW or pt-AO, and as the spoken language, with what captions show of them; the pickers do not offer them.
   */
  const SPLIT = [
    ["zh", "Chinese", "none"],
    ["pt", "Portuguese", "all"],
  ];

  /**
   * The languages ElevenLabs Scribe v2 Realtime transcribes, by the code the spoken language takes, as its
   * documentation lists them (https://elevenlabs.io/docs/overview/capabilities/speech-to-text). Each is named as a
   * natural language above, except Portuguese and Chinese, which that list splits by country or script.
   */
  const HEARD = [
    "af", "am", "ar", "hy", "as", "ast", "az", "be", "bn", "bs", "bg", "my", "yue", "ca", "ceb", "ny", "hr", "cs",
    "da", "nl", "en", "et", "fil", "fi", "fr", "ff", "gl", "lg", "ka", "de", "el", "gu", "ha", "he", "hi", "hu", "is",
    "ig", "id", "ga", "it", "ja", "jv", "kea", "kn", "kk", "km", "ko", "ku", "ky", "lo", "lv", "ln", "lt", "luo", "lb",
    "mk", "ms", "ml", "mt", "zh", "mi", "mr", "mn", "ne", "nso", "no", "oc", "or", "ps", "fa", "pl", "pt", "pa", "ro",
    "ru", "sr", "sn", "sd", "si", "sk", "sl", "so", "es", "sw", "sv", "ta", "tg", "te", "th", "tr", "uk", "umb", "ur",
    "uz", "vi", "cy", "wo", "xh", "zu",
  ];
  const HEARD_NAMES = { pt: "Portuguese", zh: "Chinese (Mandarin)" };

  const languages = { CONSTRUCTED, HEARD, HEARD_NAMES, NATURAL, SCRIPTS, SPLIT };
  if (inNode) {
    module.exports = languages;
  } else {
    window.SceneuronEditor.languages = languages;
  }
})();
