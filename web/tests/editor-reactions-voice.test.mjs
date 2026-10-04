// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises reactions-text.js on the voice of each preset: which presets have a voice switch and the phrase it names,
// and what saying a preset's trigger phrase does now: off with the switch or its own voice, waiting for voice control
// to be on and to listen always, paused with automation, and on with every phrase to say.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { test } from "node:test";

const require = createRequire(import.meta.url);
const { voiceLabel, voiceStatus } = require("../editor/reactions-text.js");
const { PRESETS } = require("../reaction-presets.js");

const [TONGUE, EMAIL, MIND_BLOWN] = PRESETS;
const ON = { enabled: true, cooldown_ms: 20000, camera_off: [], voice_off: [] };
const LISTENING = { enabled: true, mode: "always_on", listening: true };
const LIVE = { automation_enabled: true, voice: LISTENING };

test("every preset but the tongue has a voice switch that names its first phrase", () => {
  assert.deepEqual(
    PRESETS.map((preset) => Boolean(voiceStatus(preset, ON, LIVE))),
    [false, true, true, true, true, true, true, true, true, true],
  );
  assert.equal(voiceStatus(TONGUE, ON, LIVE), null);
  assert.equal(voiceLabel(EMAIL), 'Voice: "could have been an email"');
  assert.equal(voiceLabel(PRESETS[4]), 'Voice: "coffee break"');
});

test("on, it says every phrase to say", () => {
  assert.deepEqual(voiceStatus(EMAIL, ON, LIVE), {
    state: "on",
    text:
      'On: say "could have been an email" (or "should have been an email") anywhere in a sentence and 📧 plays, at ' +
      "most once per cooldown.",
  });
  assert.equal(
    voiceStatus(PRESETS[3], ON, LIVE).text,
    'On: say "ship it" anywhere in a sentence and 🚀 plays, at most once per cooldown.',
  );
});

test("the switch or the preset's own voice switch turns it off", () => {
  assert.deepEqual(voiceStatus(EMAIL, { ...ON, enabled: false }, LIVE), { state: "off", text: "Off with fun reactions." });
  assert.deepEqual(voiceStatus(EMAIL, { ...ON, voice_off: ["email"] }, LIVE), {
    state: "off",
    text: "Off: saying it plays nothing.",
  });
  assert.equal(voiceStatus(MIND_BLOWN, { ...ON, voice_off: ["email"] }, LIVE).state, "on", "only its own");
  assert.equal(voiceStatus(EMAIL, { enabled: true, cooldown_ms: 20000 }, LIVE).state, "on", "no list yet");
  assert.equal(voiceStatus(EMAIL, undefined, LIVE).state, "off", "a draft not loaded yet");
});

test("it waits for voice control to listen always, and for automation", () => {
  const live = (voice, automation = true) => ({ automation_enabled: automation, voice });
  assert.deepEqual(voiceStatus(EMAIL, ON, live({ ...LISTENING, enabled: false })), {
    state: "paused",
    text: "Waiting for voice control: turn it on in the AI tab.",
  });
  assert.equal(voiceStatus(EMAIL, ON, null).state, "paused", "before the first snapshot");
  assert.deepEqual(voiceStatus(EMAIL, ON, live({ ...LISTENING, mode: "push_to_talk" })), {
    state: "paused",
    text: 'Waiting for voice control to listen always. Holding push to talk, "could have been an email" plays it at once.',
  });
  assert.deepEqual(voiceStatus(EMAIL, ON, live(LISTENING, false)), {
    state: "paused",
    text: "Paused while automation is paused.",
  });
});
