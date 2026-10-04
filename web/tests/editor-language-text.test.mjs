// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/languages.js and editor/language-text.js: the languages the pickers list, by name and in their
// groups, natural ones covering the most spoken and every one Scribe transcribes, constructed and fictional ones by
// ISO 639 code or private-use tag, what captions show of each, which the overlay writes in a script of their own, the
// spoken languages Scribe hears, and the BCP 47 codes the server accepts.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const { CONSTRUCTED, HEARD, NATURAL, SCRIPTS, SPLIT } = require("../editor/languages.js");
const L = require("../editor/language-text.js");

// The 38 languages with 50 million speakers or more in Ethnologue's 2026 count, as Wikipedia's list of languages by
// total number of speakers shows them, by the code the list takes, and others with tens of millions.
const MOST_SPOKEN = [
  "en", "zh-Hans", "hi", "es", "ar", "fr", "bn", "pt-BR", "id", "ur", "ru", "de", "ja", "pcm", "arz", "mr", "vi", "te",
  "sw", "ha", "tr", "pnb", "tl", "ta", "yue", "wuu", "fa", "ko", "am", "th", "jv", "it", "gu", "kn", "apc", "apd", "yo",
  "bho", "hak", "pl", "my", "su", "or", "ms", "uk", "mai", "uz", "ml", "ro", "nl", "ig",
];
// The letters CEA-608 captions carry besides A to Z (libcaption's eia608_charmap.c in libobs).
const CEA_608 = new Set("áéíóúçÑñàèâêîôûÁÉÓÚÜüÀÂÇÈÊËëÎÏïÔÙùÛÃãÍÌìÒòÕõÄäÖößÅåØø");

describe("the list", () => {
  test("covers the most spoken languages, and every language Scribe transcribes", () => {
    const codes = NATURAL.map(([code]) => code);
    assert.deepEqual(MOST_SPOKEN.filter((code) => !codes.includes(code)), []);
    assert.equal(HEARD.length, 100, "Scribe v2 Realtime's documented languages");
    const unnamed = HEARD.filter((code) => !["pt", "zh"].includes(code) && L.listedCode(code) !== code);
    assert.deepEqual(unnamed, [], "each language Scribe hears is listed under its own code");
    assert.ok(codes.includes("pt-BR") && codes.includes("pt-PT") && codes.includes("zh-Hant"));
  });

  test("groups natural languages, then constructed and fictional ones, each by name", () => {
    assert.deepEqual(L.GROUPS.map(({ label, languages }) => [label, languages.length]), [
      ["Languages", NATURAL.length],
      ["Constructed and fictional", CONSTRUCTED.length],
    ]);
    const [natural, fictional] = L.GROUPS.map(({ languages }) => languages);
    assert.deepEqual(natural.slice(0, 3), [["af", "Afrikaans"], ["sq", "Albanian"], ["am", "Amharic"]]);
    assert.deepEqual(fictional, [
      ["art-x-dothraki", "Dothraki"],
      ["eo", "Esperanto"],
      ["art-x-valyrian", "High Valyrian"],
      ["io", "Ido"],
      ["ia", "Interlingua"],
      ["ie", "Interlingue (Occidental)"],
      ["tlh", "Klingon"],
      ["ldn", "Láadan"],
      ["jbo", "Lojban"],
      ["art-x-navi", "Na'vi"],
      ["nov", "Novial"],
      ["qya", "Quenya (Elvish)"],
      ["sjn", "Sindarin (Elvish)"],
      ["tok", "Toki Pona"],
      ["vo", "Volapük"],
    ]);
    for (const languages of [natural, fictional]) {
      const names = languages.map(([, name]) => name);
      assert.deepEqual(names, [...names].sort((one, other) => one.localeCompare(other, "en")), "in the order of names");
    }
    const all = NATURAL.concat(CONSTRUCTED).map(([code]) => code.toLowerCase());
    assert.equal(new Set(all).size, all.length, "no code is listed twice");
  });

  test("every code is one the server accepts, and says what captions show of it", () => {
    for (const [code, name, captions, letter] of NATURAL.concat(CONSTRUCTED, SPLIT)) {
      assert.ok(L.validTag(code), code);
      assert.ok(["all", "some", "none"].includes(captions), `${name}: ${captions}`);
      if (captions === "some") {
        assert.ok([...letter].some((each) => !CEA_608.has(each) && !/[A-Za-z]/.test(each)), `${name}: ${letter}`);
      } else {
        assert.equal(letter, undefined, name);
      }
    }
  });
});

describe("lookups", () => {
  test("names, labels, and listed codes", () => {
    assert.equal(L.languageName("qya"), "Quenya (Elvish)");
    assert.equal(L.languageName("ART-X-Dothraki"), "Dothraki");
    assert.equal(L.languageName("es-MX"), "Spanish", "by its language");
    assert.equal(L.languageName("sr-Latn-RS"), "Serbian (Latin)", "by the longest listed code it starts with");
    assert.equal(L.languageName("zh-Hant-TW"), "Chinese (Traditional)");
    assert.equal(L.languageName("zh-TW"), "Chinese", "a language listed only by script");
    assert.equal(L.languageName("pt-AO"), "Portuguese", "a language listed only by country");
    assert.equal(L.languageName("nan"), "nan", "unlisted");
    assert.equal(L.languageName("art-x-huttese"), "art-x-huttese", "unlisted");
    assert.equal(L.languageLabel("es"), "Spanish (es)");
    assert.equal(L.languageLabel("qya"), "Quenya (Elvish, qya)");
    assert.equal(L.languageLabel("zh-Hant"), "Chinese (Traditional, zh-Hant)");
    assert.equal(L.languageLabel("art-x-huttese"), "art-x-huttese");
    assert.equal(L.listedCode("PT-br"), "pt-BR");
    assert.equal(L.listedCode("es-MX"), null, "only a listed code itself");
    assert.equal(L.listedCode("zh"), null, "named, but not offered");
    assert.deepEqual(L.language("sjn"), {
      code: "sjn",
      name: "Sindarin (Elvish)",
      captions: "some",
      letter: "ŷ",
      constructed: true,
    });
  });

  test("constructed and fictional languages: the listed ones, by code or language, and any under art", () => {
    for (const code of ["qya", "SJN", "tlh", "tok", "jbo", "eo", "ia", "vo", "ldn", "io", "ie", "nov"]) {
      assert.ok(L.constructed(code), code);
    }
    for (const code of ["art-x-dothraki", "art-x-huttese", "eo-x-old", "art"]) {
      assert.ok(L.constructed(code), code);
    }
    for (const code of ["es", "la", "en-x-elvish", "", "arz"]) {
      assert.ok(!L.constructed(code), code);
    }
  });

  test("the scripts of their own the overlay writes constructed languages in, by language subtag", () => {
    const named = ["qya", "SJN", "tlh", "tok-x-test"].map(L.scriptName);
    assert.deepEqual(named, ["Tengwar", "Tengwar", "pIqaD", "sitelen pona"]);
    for (const code of ["es", "ja", "jbo", "eo", "art-x-dothraki", "constructor", "", undefined]) {
      assert.equal(L.scriptName(code), null, String(code));
    }
    assert.ok(Object.keys(SCRIPTS).every((code) => CONSTRUCTED.some(([listed]) => listed === code)));
  });

  test("the spoken languages are those Scribe hears, by name", () => {
    const spoken = L.spokenLanguages();
    assert.equal(spoken.length, 100);
    assert.deepEqual(spoken.slice(0, 2), [["af", "Afrikaans"], ["am", "Amharic"]]);
    assert.ok(spoken.some(([code, name]) => code === "zh" && name === "Chinese (Mandarin)"));
    assert.ok(L.hears("EN") && L.hears("yue") && !L.hears("qya") && !L.hears("la"));
  });
});

describe("codes", () => {
  test("well-formed BCP 47 tags, private use included, are accepted", () => {
    const good = [
      "es", "pt-BR", "zh-Hans", "es-419", "fil", "sr-Latn-RS", "EN-us", "qya", "sjn", "tlh", "tok", "jbo", "eo", "ia",
      "vo", "ldn", "io", "ie", "nov", "art-x-dothraki", "art-x-valyrian", "art-x-navi", "ART-X-Navi", "en-x-a",
      "zh-yue-HK", "de-1996", "sl-rozaj-biske", "en-a-bbb-x-a-ccc", "en-u-ca-gregory", "de-CH-1901-x-old",
    ];
    for (const code of good) {
      assert.ok(L.validTag(code), code);
    }
  });

  test("malformed tags and anything looser than RFC 5646 are refused", () => {
    const bad = [
      "", "e", "espa", "es_MX", "pt-", "-es", "es--MX", "es-ABCDEFGHI", "1a", "es MX", "ñu", "x-dothraki",
      "i-klingon", "art-x", "art-x-", "art-x-highvalyrian", "art-x-na'vi", "en-x", "en-a", "en-a-x-b", "en-a-b",
      "en-a-bb-a-cc", "de-1996-1996", "en-US-GB", "zh-Hans-Hant", "en-1a", "en-abc-US-def", "es-419-MX",
      "zh-yue-yue-yue-yue", "en-US-aaaaaaaa-bbbbbbbb-cccccccc-dddd",
    ];
    for (const code of bad) {
      assert.ok(!L.validTag(code), code);
    }
  });
});
