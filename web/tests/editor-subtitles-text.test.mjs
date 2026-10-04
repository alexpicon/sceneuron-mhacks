// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/subtitles-text.js: language names, the codes the server accepts, and the spoken languages offered,
// the subtitle consent wording, where translations go, and the switch, which settings a form sends, the subtitles
// chip, the preview, which final segments a screen reader hears, and what stream captions can carry, constructed and
// fictional languages included.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const S = require("../editor/subtitles-text.js");

const subtitles = {
  enabled: false,
  consent: false,
  language: "",
  targets: [],
  translation_model: "meta/llama-3.3-70b-instruct",
  lines: 2,
  clear_after_ms: 4000,
  hide_commands: true,
  stream_captions: false,
  stream_language: "",
  translation_price_known: true,
  translation_input_price_per_mtok: 1,
  translation_output_price_per_mtok: 5,
};
const ai = {
  provider: "openai-compatible",
  base_url: "https://integrate.api.nvidia.com/v1",
  voice: { stt_price_per_hour: 0.39 },
};

describe("languages", () => {
  test("listed codes have names, and others show as typed", () => {
    assert.equal(S.languageName("es"), "Spanish");
    assert.equal(S.languageName("PT-br"), "Portuguese (Brazil)");
    assert.equal(S.languageName("es-MX"), "Spanish", "a region of a listed language");
    assert.equal(S.languageName("tlh"), "Klingon");
    assert.equal(S.languageName("art-x-huttese"), "art-x-huttese");
  });

  test("targets are checked like the server checks them", () => {
    const notACode = (code) => `"${code}" is not a language code such as es, pt-BR, zh-Hans, or art-x-dothraki.`;
    assert.equal(S.targetsProblem([]), null);
    assert.equal(S.targetsProblem(["es", "pt-BR", "zh-Hans"]), null);
    assert.equal(S.targetsProblem(["qya", "tlh", "art-x-dothraki"]), null);
    assert.equal(S.targetsProblem(["es", "fr", "de", "ja"]), "Choose at most 3 languages.");
    assert.equal(S.targetsProblem(["es_MX"]), notACode("es_MX"));
    assert.equal(S.targetsProblem(["español"]), notACode("español"));
    assert.equal(S.targetsProblem(["x-dothraki"]), notACode("x-dothraki"));
    assert.equal(S.targetsProblem(["art-x-highvalyrian"]), notACode("art-x-highvalyrian"));
    assert.equal(S.targetsProblem(["es", "ES"]), "ES is listed twice.");
  });

  test("the spoken language is detected or one Scribe hears, never a constructed one", () => {
    assert.equal(S.spokenProblem(""), null);
    assert.equal(S.spokenProblem("spa"), null);
    assert.equal(S.spokenProblem("English"), "Use an ISO 639 code such as en or spa, or leave it empty.");
    assert.equal(S.spokenProblem("EN"), "Use an ISO 639 code such as en or spa, or leave it empty.");
    assert.equal(
      S.spokenProblem("qya"),
      "Speech to text does not hear Quenya (Elvish, qya); choose it as a language to translate into instead.",
    );
    assert.equal(
      S.spokenProblem("art"),
      "Speech to text does not hear art; choose it as a language to translate into instead.",
    );
    const options = S.spokenOptions("");
    assert.deepEqual(options[0], ["", "Detect it automatically"]);
    assert.equal(options.length, 101, "detecting it and the 100 languages Scribe hears");
    assert.ok(options.some(([code, name]) => code === "pt" && name === "Portuguese"));
    assert.ok(!options.some(([code]) => ["qya", "tlh", "eo", "la"].includes(code)), "only languages Scribe hears");
    assert.deepEqual(S.spokenOptions("es"), options, "a listed language needs no option of its own");
    const kept = S.spokenOptions("spa");
    assert.deepEqual(kept.slice(0, 3), [["", "Detect it automatically"], ["spa", "spa"], options[1]], "a typed one");
  });
});

describe("consent", () => {
  test("the points say what is sent, where, when, and what it costs, with or without translations", () => {
    const alone = Object.fromEntries(S.subtitleConsentPoints(subtitles, ai));
    assert.equal(
      alone.What,
      "Audio from your microphone, all the time while subtitles are on, even with voice control off.",
    );
    assert.equal(alone.Where, "api.elevenlabs.io (ElevenLabs), which turns it into text.");
    assert.equal(alone.Translations, "None until you choose languages below; until then no text leaves this computer.");
    assert.equal(alone.Cost, "Listening costs about $0.39 an hour; all of it counts toward the hourly budget.");
    const translated = Object.fromEntries(S.subtitleConsentPoints({ ...subtitles, targets: ["es", "pt-BR"] }, ai));
    assert.equal(
      translated.Translations,
      "Each final line, never words still being said, goes to integrate.api.nvidia.com (an OpenAI-compatible " +
        "service) with meta/llama-3.3-70b-instruct, with the three lines before it, to be translated into Spanish, " +
        "Portuguese (Brazil).",
    );
    assert.equal(
      translated.Cost,
      "Listening costs about $0.39 an hour, translations $1.00 per million tokens read and $5.00 written; all of it " +
        "counts toward the hourly budget.",
    );
    const local = { provider: "openai-compatible", base_url: "http://127.0.0.1:11434/v1" };
    const unpriced = { ...subtitles, targets: ["es"], translation_price_known: false };
    const points = Object.fromEntries(S.subtitleConsentPoints(unpriced, local));
    assert.match(points.Translations, /goes to 127\.0\.0\.1:11434, a model on your own computer with /);
    assert.equal(
      points.Cost,
      "Listening costs what ElevenLabs charges, and translations at a price Sceneuron does not know, which count as " +
        "$0; all of it counts toward the hourly budget.",
    );
    assert.ok(points.Shown.includes("never written to the event log"));
    assert.equal(translated.Google, undefined, "a service away from Google's address gets no Gemini note");
  });

  test("translations go to the address the AI settings name, with Google's free-tier note for Gemini", () => {
    const gemini = { provider: "gemini", base_url: "https://generativelanguage.googleapis.com" };
    const flash = { ...subtitles, translation_model: "gemini-3.5-flash-lite" };
    const points = S.subtitleConsentPoints({ ...flash, targets: ["es"] }, gemini);
    assert.deepEqual(points.map(([label]) => label), ["What", "Where", "Translations", "Google", "Shown", "Cost"]);
    const named = Object.fromEntries(points);
    assert.equal(
      named.Translations,
      "Each final line, never words still being said, goes to generativelanguage.googleapis.com (Google Gemini) " +
        "with gemini-3.5-flash-lite, with the three lines before it, to be translated into Spanish.",
    );
    assert.equal(
      named.Google,
      "With a free-tier key (a Google project without billing), Google may use what is sent to Gemini to improve " +
        "its products, and people may review it. A key from a project with billing is not used that way.",
    );
    assert.equal(Object.fromEntries(S.subtitleConsentPoints(flash, gemini)).Google, named.Google, "before languages");
    const standIn = { provider: "gemini", base_url: "http://127.0.0.1:8090" };
    assert.equal(Object.fromEntries(S.subtitleConsentPoints(subtitles, standIn)).Google, undefined);
    const proxy = { provider: "gemini", base_url: "https://gemini-proxy.example.com" };
    const proxied = Object.fromEntries(S.subtitleConsentPoints({ ...flash, targets: ["es"] }, proxy));
    assert.match(proxied.Translations, / goes to gemini-proxy\.example\.com \(Google Gemini\) with /, "its address");
    assert.equal(proxied.Google, undefined, "a stand-in away from Google's address");
  });

  test("the points name constructed and fictional languages in words", () => {
    const fictional = { ...subtitles, targets: ["qya", "art-x-dothraki", "art-x-huttese"] };
    const named = Object.fromEntries(S.subtitleConsentPoints(fictional, ai));
    assert.ok(named.Translations.endsWith(" to be translated into Quenya (Elvish), Dothraki, art-x-huttese."),
      named.Translations);
  });

  test("the switch needs consent first", () => {
    assert.deepEqual(S.subtitleSwitch(subtitles), {
      checked: false,
      disabled: true,
      note: "Tick the consent box above first.",
    });
    assert.equal(S.subtitleSwitch({ ...subtitles, consent: true, enabled: true }).checked, true);
  });
});

describe("the form", () => {
  test("only changed fields are sent, with seconds as milliseconds and empty targets left out", () => {
    const same = {
      language: "",
      targets: ["", "", ""],
      translation_model: "meta/llama-3.3-70b-instruct",
      lines: "2",
      clear_after_s: "4",
      hide_commands: true,
    };
    assert.equal(S.subtitleUpdate(subtitles, same), null);
    const changed = { ...same, language: " en ", targets: ["es", "", "fr"], lines: "1", clear_after_s: "6.5" };
    assert.deepEqual(S.subtitleUpdate(subtitles, changed), {
      subtitles: { language: "en", targets: ["es", "fr"], lines: 1, clear_after_ms: 6500 },
    });
    assert.deepEqual(S.subtitleUpdate(subtitles, { ...same, hide_commands: false }), {
      subtitles: { hide_commands: false },
    });
  });

  test("the languages kept in Latin letters and the Latin lines are sent when they change", () => {
    const scripted = { ...subtitles, targets: ["qya", "tlh"], latin_targets: ["tlh"], latin_lines: false };
    const same = { targets: ["qya", "tlh"], latin_targets: ["tlh"], latin_lines: false };
    assert.equal(S.subtitleUpdate(scripted, same), null);
    assert.deepEqual(S.subtitleUpdate(scripted, { ...same, latin_targets: ["qya", "tlh"], latin_lines: true }), {
      subtitles: { latin_targets: ["qya", "tlh"], latin_lines: true },
    });
    assert.deepEqual(S.subtitleUpdate(scripted, { ...same, latin_targets: [] }), { subtitles: { latin_targets: [] } });
    assert.equal(S.subtitleUpdate(scripted, { latin_lines: false }), null, "fields left out keep their values");
  });
});

describe("live", () => {
  const view = (state, extra) => ({
    lines: [],
    live: "",
    translations: {},
    status: {
      enabled: state !== "off",
      state,
      message: state === "paused" ? "the hourly budget is spent" : "",
      targets: [],
    },
    ...extra,
  });

  test("the chip follows the status", () => {
    assert.deepEqual(S.subtitleState(null), { state: "off", label: "Off" });
    assert.deepEqual(S.subtitleState(view("listening")), { state: "watching", label: "Listening" });
    assert.deepEqual(S.subtitleState(view("paused")), { state: "paused", label: "Paused" });
    assert.deepEqual(S.subtitleState(view("needs_key")), { state: "needs_setup", label: "Needs a key" });
  });

  test("the preview keeps live words apart and says why nothing shows", () => {
    const shown = view("listening", {
      lines: ["Good morning."],
      live: "Let's",
      translations: { es: ["Buenos días."] },
      status: { enabled: true, state: "listening", targets: ["es", "fr"] },
    });
    assert.deepEqual(S.previewText(shown), {
      lines: ["Good morning."],
      live: "Let's",
      translations: [
        { code: "es", name: "Spanish", lines: ["Buenos días."] },
        { code: "fr", name: "French", lines: [] },
      ],
      note: "",
    });
    assert.equal(S.previewText(view("listening")).note, "Waiting for speech.");
    const status = { enabled: false, state: "paused", message: "the hourly budget is spent" };
    const paused = { ...view("paused"), status };
    assert.equal(S.previewText(paused).note, "Subtitles are off: the hourly budget is spent.");
    assert.equal(S.previewText(null).note, "Subtitles are off.");
  });

  // A view whose final segments are `texts`, numbered from `first`, shown as `lines`.
  const finals = (first, texts, lines = texts) =>
    view("listening", { lines, finals: texts.map((text, index) => ({ id: first + index, text })) });

  test("each new final segment is announced once, never live words or what was there before", () => {
    const before = S.finalAnnouncement(undefined, null);
    assert.deepEqual(before, { seen: undefined, message: null }, "no live view yet");
    const first = S.finalAnnouncement(before.seen, finals(1, ["Earlier."]));
    assert.equal(first.message, null, "nothing for segments already shown when the editor opened");
    const partial = S.finalAnnouncement(first.seen, { ...finals(1, ["Earlier."]), live: "Good" });
    assert.equal(partial.message, null);
    const next = S.finalAnnouncement(partial.seen, finals(1, ["Earlier.", "Good morning."]));
    assert.equal(next.message, "Subtitle: Good morning.");
    const cleared = S.finalAnnouncement(next.seen, view("listening"));
    assert.equal(cleared.message, null);
    const again = S.finalAnnouncement(cleared.seen, finals(3, ["Hello."]));
    assert.equal(again.message, "Subtitle: Hello.");
  });

  test("a segment that wraps is announced whole, not only its last line", () => {
    const sentence = "Today we are planning the next sprint together and we will look at every story in the backlog " +
      "before lunch.";
    const wrapped = [sentence.slice(0, 84).trim(), sentence.slice(84).trim()];
    const opened = S.finalAnnouncement(undefined, view("listening"));
    const heard = S.finalAnnouncement(opened.seen, finals(1, [sentence], wrapped));
    assert.equal(heard.message, `Subtitle: ${sentence}`);
  });

  test("a sentence said again is announced again", () => {
    const once = S.finalAnnouncement([], finals(1, ["Yes."]));
    assert.equal(once.message, "Subtitle: Yes.");
    const twice = S.finalAnnouncement(once.seen, finals(1, ["Yes.", "Yes."]));
    assert.equal(twice.message, "Subtitle: Yes.");
    const thrice = S.finalAnnouncement(twice.seen, finals(2, ["Yes.", "Yes."]));
    assert.equal(thrice.message, "Subtitle: Yes.", "even when the lines look the same");
  });

  test("two segments that arrive between renders are both announced, in order", () => {
    const opened = S.finalAnnouncement(undefined, finals(1, ["One."]));
    const both = S.finalAnnouncement(opened.seen, finals(1, ["One.", "Two.", "Three."]));
    assert.equal(both.message, "Subtitle: Two. Three.");
  });
});

describe("stream captions", () => {
  test("they can be in the spoken language or a target, and say what CEA-608 carries", () => {
    assert.deepEqual(S.streamOptions({ targets: ["es", "ja"] }), [
      ["", "The spoken language"],
      ["es", "Spanish (es)"],
      ["ja", "Japanese (ja)"],
    ]);
    const latin = S.captionNote({ stream_language: "es" });
    assert.match(latin, /CEA-608, which carries Latin-script text only; viewers turn captions on in their player/);
    assert.doesNotMatch(latin, /blank|left out/);
  });

  test("other scripts are blank, and Latin script with letters CEA-608 lacks loses only those letters", () => {
    for (const [code, name] of [["ru", "Russian"], ["uk", "Ukrainian"], ["ar", "Arabic"], ["hi", "Hindi"],
      ["ja", "Japanese"], ["ko", "Korean"], ["zh-Hans", "Chinese (Simplified)"], ["zh-TW", "Chinese"],
      ["zh-Hant-TW", "Chinese (Traditional)"]]) {
      const note = S.captionNote({ stream_language: code });
      assert.ok(note.endsWith(`${name} uses letters CEA-608 cannot show, so most of its captions would be blank.`), note);
    }
    const latin = [["pl", "Polish", "ł"], ["tr", "Turkish", "ş"], ["vi", "Vietnamese", "ơ"], ["fr", "French", "œ"],
      ["sr-Latn-RS", "Serbian (Latin)", "č"]];
    for (const [code, name, letter] of latin) {
      const note = S.captionNote({ stream_language: code });
      assert.ok(note.endsWith(`In ${name}, some letters, such as ${letter}, are left out of captions.`), note);
      assert.doesNotMatch(note, /blank/);
    }
    assert.match(S.captionNote({ stream_language: "", language: "ru" }), /Russian uses letters/, "the spoken language");
    assert.match(S.captionNote({ stream_language: "", language: "pl" }), /In Polish, some letters/);
    assert.match(S.captionNote({ stream_language: "", language: "zh" }), /Chinese uses letters/, "spoken Chinese");
    for (const code of ["en", "de", "it", "pt-BR", "sv"]) {
      assert.doesNotMatch(S.captionNote({ stream_language: code }), /blank|left out|without/, code);
    }
  });

  test("constructed and fictional languages are named with their code, and captions carry their Latin script", () => {
    assert.deepEqual(S.streamOptions({ targets: ["qya", "art-x-dothraki", "art-x-huttese"] }), [
      ["", "The spoken language"],
      ["qya", "Quenya (Elvish, qya)"],
      ["art-x-dothraki", "Dothraki (art-x-dothraki)"],
      ["art-x-huttese", "art-x-huttese"],
    ]);
    const note = (code) => S.captionNote({ stream_language: code });
    for (const [code, name] of [["qya", "Quenya (Elvish)"], ["tlh", "Klingon"], ["art-x-navi", "Na'vi"]]) {
      assert.ok(note(code).endsWith(`${name} is translated in Latin script, so captions carry all of it.`), note(code));
    }
    for (const [code, name, letter] of [["eo", "Esperanto", "ŝ"], ["sjn", "Sindarin (Elvish)", "ŷ"],
      ["art-x-valyrian", "High Valyrian", "ā"]]) {
      const expected = `${name} is translated in Latin script, so captions carry it without some letters, such as ` +
        `${letter}.`;
      assert.ok(note(code).endsWith(expected), note(code));
    }
    const unlisted = note("art-x-huttese");
    assert.ok(unlisted.endsWith("art-x-huttese is translated in Latin script, so captions carry it."), unlisted);
    assert.doesNotMatch(note("qya"), /blank/);
  });
});
