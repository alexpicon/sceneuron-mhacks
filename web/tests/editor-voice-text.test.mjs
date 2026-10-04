// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/voice-text.js: the write-only ElevenLabs key's state, the voice consent wording and switch,
// which voice settings a form sends, the voice chip and the live heard line, and the checks on trigger phrases
// and speech lines that mirror the server's.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const V = require("../editor/voice-text.js");

const voice = {
  enabled: false,
  consent: false,
  mode: "push_to_talk",
  wake_word: "Sceneuron",
  language: "",
  speak: false,
  voice_id: "JBFqnCBsd6RMkjVDRZzb",
  tts_model: "eleven_flash_v2_5",
  stt_model: "scribe_v2_realtime",
  tts_price_per_1k_chars: 0.04,
  tts_price_listed: true,
  stt_price_per_hour: 0.39,
  stt_price_listed: true,
  keyterm_price_per_hour: 0.05,
};

describe("the ElevenLabs key", () => {
  test("its state shows only the last characters and where it came from", () => {
    assert.equal(V.voiceKeyStatus({}), "No ElevenLabs key; voice control and spoken cards stay off, and everything else works");
    const saved = { has_elevenlabs_key: true, elevenlabs_key_hint: "...wxyz", elevenlabs_key_source: "editor" };
    assert.equal(V.voiceKeyStatus(saved), "Key saved (...wxyz)");
    assert.equal(V.voiceKeyRemovable(saved), true);
    const env = { ...saved, elevenlabs_key_source: "environment" };
    assert.equal(V.voiceKeyStatus(env), "Key from .env or the environment (...wxyz); change it there");
    assert.equal(V.voiceKeyRemovable(env), false);
    assert.equal(V.voiceKeyRemovable(null), false);
  });
});

describe("consent and listening", () => {
  test("the consent points say what is sent, where, when, and what it costs", () => {
    const points = Object.fromEntries(V.voiceConsentPoints(voice));
    assert.equal(points.What, "Audio from your microphone, only while push to talk is held.");
    assert.equal(
      points.Keyterms,
      "With it go the wake word and the names and trigger phrases of cards that voice can raise, so they are " +
        "recognized.",
    );
    assert.equal(points.Where, "api.elevenlabs.io (ElevenLabs), which turns it into text.");
    assert.match(points["Spoken cards"], /once per voice; the audio is kept on this computer/);
    assert.equal(
      points.Commands,
      "Words you say to Sceneuron that it does not know by itself go as text, with your project, to the AI " +
        "provider in this tab, once it has a key.",
    );
    assert.equal(
      points.Cost,
      "Listening costs about $0.39 an hour, $0.05 more with keyterms, and spoken cards about $0.04 per 1,000 " +
        "characters; both count toward the hourly budget.",
    );
    const unlisted = Object.fromEntries(
      V.voiceConsentPoints({ ...voice, tts_model: "eleven_v9", tts_price_per_1k_chars: 0.08, tts_price_listed: false }),
    );
    assert.match(unlisted.Cost, /about \$0\.08 per 1,000 characters/);
    assert.match(unlisted.Cost, / No list price is known for eleven_v9, so it counts at the highest known rate\.$/);
    const always = Object.fromEntries(V.voiceConsentPoints({ ...voice, mode: "always_on", wake_word: "Neuron" }));
    assert.equal(always.What, 'Audio from your microphone, all the time while voice control is on, so "Neuron" and ' +
      "trigger phrases are heard.");
    const unpriced = Object.fromEntries(V.voiceConsentPoints({ ...voice, tts_price_per_1k_chars: 0 }));
    assert.equal(unpriced.Cost, "Voice spend counts toward the hourly budget.");
  });

  test("listening needs consent first", () => {
    assert.deepEqual(V.listenSwitch(voice), { checked: false, disabled: true, note: "Tick the consent box above first." });
    const on = V.listenSwitch({ ...voice, consent: true, enabled: true });
    assert.equal(on.checked && !on.disabled, true);
    assert.match(on.note, /no audio is sent while it is off/);
  });
});

describe("the voice form", () => {
  test("only changed fields and a typed key are sent", () => {
    const values = { ...voice, wake_word: " Neuron ", language: "es", elevenlabs_api_key: "" };
    assert.deepEqual(V.voiceUpdate(voice, values), { voice: { wake_word: "Neuron", language: "es" } });
    assert.deepEqual(V.voiceUpdate(voice, { ...voice, elevenlabs_api_key: " el-key " }), { elevenlabs_api_key: "el-key" });
    assert.equal(V.voiceUpdate(voice, { ...voice, elevenlabs_api_key: "  " }), null);
  });
});

describe("the live view", () => {
  test("the chip names the voice state", () => {
    assert.deepEqual(V.voiceState(null), { state: "off", label: "Off" });
    assert.deepEqual(V.voiceState({ enabled: true }), { state: "ready", label: "Ready" });
    assert.deepEqual(V.voiceState({ enabled: true, listening: true }), { state: "watching", label: "Listening" });
    assert.deepEqual(V.voiceState({ enabled: true, talking: true }), { state: "thinking", label: "Talking" });
  });

  test("the heard line shows partial words, final words, and what they did", () => {
    assert.equal(V.heardLine(null), "Nothing heard yet.");
    assert.equal(V.heardLine({ heard: null }), "Nothing heard yet.");
    assert.equal(V.heardLine({ heard: { text: "Sceneuron wrap", is_final: false } }), 'Hearing "Sceneuron wrap"...');
    const heard = { text: "Sceneuron, wrap it up", is_final: true, result: "raised Meeting signals: Wrap it up" };
    assert.equal(V.heardLine({ heard }), 'Heard "Sceneuron, wrap it up": raised Meeting signals: Wrap it up.');
    assert.equal(V.heardLine({ heard: { ...heard, result: "" } }), 'Heard "Sceneuron, wrap it up".');
    const typed = { text: "hacker theme", is_final: true, typed: true, result: "switched the overlay to the Hacker theme" };
    assert.equal(V.heardLine({ heard: typed }), 'Typed "hacker theme": switched the overlay to the Hacker theme.');
  });

  test("the AI's later answer to the same words is said too", () => {
    const asked = (result) => ({ heard: { text: "make it rain", is_final: true, at_ms: 4, result } });
    let { seen } = V.heardAnnouncement(undefined, { heard: null });
    let message;
    ({ seen, message } = V.heardAnnouncement(seen, asked("asking the AI")));
    assert.equal(message, 'Heard "make it rain": asking the AI.');
    ({ seen, message } = V.heardAnnouncement(seen, asked("the AI found nothing to do")));
    assert.equal(message, 'Heard "make it rain": the AI found nothing to do.');
    assert.equal(V.heardAnnouncement(seen, asked("the AI found nothing to do")).message, null);
  });

  test("screen readers hear each final result once, and never partial words", () => {
    const final = (text, atMs) => ({ heard: { text, is_final: true, at_ms: atMs, result: "lowered Poker" } });
    const partial = { heard: { text: "Sceneuron low", is_final: false, at_ms: 5 } };
    let { seen, message } = V.heardAnnouncement(undefined, final("Sceneuron, lower", 1));
    assert.equal(message, null, "what was there when the editor opened is not said");
    ({ seen, message } = V.heardAnnouncement(seen, partial));
    assert.equal(message, null, "partial words are not said");
    ({ seen, message } = V.heardAnnouncement(seen, final("Sceneuron, lower", 1)));
    assert.equal(message, null, "the same result is not said twice");
    ({ seen, message } = V.heardAnnouncement(seen, final("Sceneuron, lower", 9)));
    assert.equal(message, 'Heard "Sceneuron, lower": lowered Poker.');
    ({ seen, message } = V.heardAnnouncement(seen, { heard: null }));
    assert.equal(message, null);
    assert.equal(V.heardAnnouncement(seen, final("Sceneuron, lower", 9)).message, null, "nothing new");
    const start = V.heardAnnouncement(undefined, partial);
    assert.equal(V.heardAnnouncement(start.seen, final("pause", 12)).message, 'Heard "pause": lowered Poker.');
  });
});

describe("trigger phrases and speech lines", () => {
  test("phrases are one per line, and the server's limits are checked", () => {
    assert.deepEqual(V.phrasesFrom(" let's take this offline \n\n next topic\n"), ["let's take this offline", "next topic"]);
    assert.equal(V.phrasesProblem([]), null);
    assert.equal(V.phrasesProblem(["We’re off-track, folks!", "¿qué tal?", "vote 8?"]), null);
    assert.equal(V.phrasesProblem(["a", "b", "c", "d", "e"]), "Use at most 4 trigger phrases, one per line.");
    for (const bad of ["a", "x".repeat(61), "50% off", "{{scene}}", "?!", "👍👍"]) {
      assert.match(V.phrasesProblem([bad]), /must be 2 to 60 letters/, bad);
    }
  });

  test("letters with vowel signs count as the server counts them", () => {
    for (const phrase of ["मुझे सवाल है", "ขอถาม"]) {
      assert.equal(V.phrasesProblem([phrase]), null, phrase);
    }
    // A Thai tone mark and a Tamil virama are not alphabetic to the server either.
    for (const phrase of ["ขอถามหน่อย", "எனக்கு"]) {
      assert.match(V.phrasesProblem([phrase]), /must be 2 to 60 letters/, phrase);
    }
  });

  test("a speech line is optional and fits on one line", () => {
    assert.equal(V.speechProblem(""), null);
    assert.equal(V.speechProblem("Coffee break"), null);
    assert.equal(V.speechProblem("x".repeat(200)), null);
    assert.equal(V.speechProblem("x".repeat(201)), "Use at most 200 characters on one line.");
    assert.equal(V.speechProblem("two\nlines"), "Use at most 200 characters on one line.");
    assert.equal(V.speechProblem("next\u0085line"), "Use at most 200 characters on one line.", "a C1 control too");
  });
});
