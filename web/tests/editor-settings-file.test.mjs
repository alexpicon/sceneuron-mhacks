// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/settings-file.js: what a settings file carries, that keys and consents stay out, and the
// errors for files that are not settings files.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { test } from "node:test";

const require = createRequire(import.meta.url);
const F = require("../editor/settings-file.js");

const project = { elements: [{ id: "title" }], rules: [] };
const view = {
  enabled: true,
  consent: true,
  provider: "gemini",
  vision_model: "gemini-3.5-flash-lite",
  has_key: true,
  key_hint: "…abcd",
  jev_model: "jev-latest",
  voice: { enabled: true, consent: true, wake_word: "Sceneuron", tts_price_per_1k_chars: 0.1 },
  subtitles: { enabled: false, consent: true, targets: ["es"], translation_price_known: true },
};

test("a file carries the project and settings but no keys or consents", () => {
  const file = F.build(project, view, "2026-10-03T00:00:00.000Z");
  assert.equal(file.format, "sceneuron-settings");
  assert.deepEqual(file.project, project);
  assert.deepEqual(file.ai, {
    enabled: true,
    provider: "gemini",
    vision_model: "gemini-3.5-flash-lite",
    jev_model: "jev-latest",
    voice: { enabled: true, wake_word: "Sceneuron" },
    subtitles: { enabled: false, targets: ["es"] },
  });
  assert.deepEqual(F.parse(JSON.stringify(file)), { project, ai: file.ai });
});

test("a file that sneaks in keys or consents loses them", () => {
  const file = F.build(project, view, "x");
  file.ai.api_key = "secret";
  file.ai.consent = true;
  file.ai.voice.consent = true;
  const { ai } = F.parse(JSON.stringify(file));
  assert.equal("api_key" in ai, false);
  assert.equal("consent" in ai, false);
  assert.equal("consent" in ai.voice, false);
});

test("other files are refused in plain words", () => {
  assert.throws(() => F.parse("nope"), /not JSON/);
  assert.throws(() => F.parse("{}"), /not a Sceneuron settings file/);
  assert.throws(() => F.parse('{"format":"sceneuron-settings","version":9}'), /version 9/);
  assert.throws(() => F.parse('{"format":"sceneuron-settings","version":1}'), /no project/);
});

test("the file name carries the date", () => {
  assert.equal(F.fileName(new Date("2026-10-03T12:00:00Z")), "sceneuron-settings-2026-10-03.json");
});
