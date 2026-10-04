// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises reactions-text.js: what the Settings tab says the fun reactions do now, with the cooldown, and what the
// camera does for each preset: the switch and the preset's own camera switch; for the tongue, the AI settings
// loading, missing AI support, the simulated AI, a missing key, consent, scene understanding off in the AI settings or
// the project, a paused or failed vision model, and a camera blind to the mouth; for the facepalm, which needs no AI,
// a camera blind to hands; for both, paused automation and a camera that is not analyzed yet, starting, or failed.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { test } from "node:test";

const require = createRequire(import.meta.url);
const { cameraLabel, cameraStatus, reactionsStatus } = require("../editor/reactions-text.js");
const { PRESETS } = require("../reaction-presets.js");

const [TONGUE, EMAIL] = PRESETS;
const ON = { enabled: true, cooldown_ms: 20000, camera_off: [] };
const POLICY = { enabled: true };
const READY = { available: true, simulated: false, has_key: true, consent: true, enabled: true };
const CAMERA = { state: "running", latency_ms: 30, mouths: true, hands: true };
const LIVE = { automation_enabled: true, vision: CAMERA, ai: { state: "watching", message: "" } };

const status = (preset, overrides = {}) => {
  const { reactions = ON, policy = POLICY, live = LIVE, ai = READY } = overrides;
  return cameraStatus(preset, reactions, policy, live, ai);
};

test("the section says whether the camera and trigger phrases play reactions, and the cooldown", () => {
  assert.deepEqual(reactionsStatus(ON, LIVE), {
    state: "on",
    text:
      "On: the camera, rules, and trigger phrases play at most one reaction every 20 s; Try it and asking " +
      "Sceneuron play one at once.",
  });
  assert.match(reactionsStatus({ ...ON, cooldown_ms: 90000 }, LIVE).text, /every 90 s;/);
  assert.deepEqual(reactionsStatus({ ...ON, enabled: false }, LIVE), {
    state: "off",
    text: "Off: the camera and trigger phrases play no reaction. Rules, Try it, and asking Sceneuron still do.",
  });
  assert.equal(reactionsStatus(undefined, LIVE).state, "off", "a draft not loaded yet");
  assert.deepEqual(reactionsStatus(ON, { ...LIVE, automation_enabled: false }), {
    state: "paused",
    text: "Paused while automation is paused. Try it and asking Sceneuron still play.",
  });
});

test("only presets the camera plays have a camera status and a switch label", () => {
  assert.deepEqual(
    PRESETS.map((preset) => Boolean(status(preset))),
    [true, true, false, false, false, false, false, false, false, false],
  );
  assert.equal(cameraLabel(TONGUE), "Camera: tongue out");
  assert.equal(cameraLabel(EMAIL), "Camera: facepalm for a second");
});

test("on, each says how to play it, and the facepalm needs no AI", () => {
  const tongue = status(TONGUE);
  assert.equal(tongue.state, "on");
  assert.match(tongue.text, /^On: stick your tongue out for a moment and the screen floods with 😜/);
  assert.match(tongue.text, /once the vision model has seen it\.$/);
  assert.deepEqual(status(EMAIL, { ai: { available: false } }), {
    state: "on",
    text:
      'On: hold a hand over your eyes or forehead for a second and 📧 "This meeting could have been an email" ' +
      "plays. It is seen on this computer, with no AI call.",
  });
});

test("the switch or the preset's own camera switch turns its camera off", () => {
  for (const preset of [TONGUE, EMAIL]) {
    assert.deepEqual(status(preset, { reactions: { ...ON, enabled: false } }), {
      state: "off",
      text: "Off with fun reactions.",
    });
    assert.deepEqual(status(preset, { reactions: { ...ON, camera_off: [preset.id] } }), {
      state: "off",
      text: "Off: the camera never plays this one.",
    });
    assert.equal(cameraStatus(preset, undefined, POLICY, LIVE, READY).state, "off", "a draft not loaded yet");
  }
  assert.equal(status(EMAIL, { reactions: { ...ON, camera_off: ["tongue"] } }).state, "on", "only its own");
  assert.equal(status(TONGUE, { reactions: { enabled: true, cooldown_ms: 20000 } }).state, "on", "no list yet");
});

test("without a usable vision model, the tongue says what to do in the AI tab", () => {
  const cases = [
    [null, "checking", "Checking the AI settings..."],
    [{ available: false }, "unavailable", "Unavailable: this Sceneuron has no AI support."],
    [
      { ...READY, simulated: true },
      "unavailable",
      "Unavailable: the simulated AI cannot see a tongue; it needs a real vision model.",
    ],
    [{ ...READY, has_key: false }, "unavailable", "Unavailable: add a vision model API key in the AI tab."],
    [
      { ...READY, consent: false },
      "unavailable",
      "Unavailable: in the AI tab, confirm that camera images may be sent to the provider.",
    ],
    [{ ...READY, enabled: false }, "unavailable", "Unavailable: turn on scene understanding in the AI tab."],
  ];
  for (const [ai, state, text] of cases) {
    assert.deepEqual(status(TONGUE, { ai }), { state, text }, JSON.stringify(ai));
    assert.equal(status(EMAIL, { ai }).state, "on", `the facepalm needs no AI: ${JSON.stringify(ai)}`);
  }
  assert.deepEqual(status(TONGUE, { policy: { enabled: false } }), {
    state: "unavailable",
    text: "Unavailable: scene understanding is off for this project.",
  });
});

test("the tongue waits while the vision model is paused or failed, and the facepalm does not", () => {
  const budget = { state: "paused", message: "hourly budget of $2.00 reached; resumes in 12 min" };
  assert.deepEqual(status(TONGUE, { live: { ...LIVE, ai: budget } }), {
    state: "paused",
    text: "Waiting for the vision model, which is paused: hourly budget of $2.00 reached; resumes in 12 min.",
  });
  const failed = { state: "failed", message: "the API key was refused" };
  assert.equal(
    status(TONGUE, { live: { ...LIVE, ai: failed } }).text,
    "Waiting for the vision model, which failed: the API key was refused.",
  );
  assert.equal(status(EMAIL, { live: { ...LIVE, ai: failed } }).state, "on");
  assert.equal(status(TONGUE, { live: null }).state, "on", "before the first snapshot it trusts the settings");
});

test("both wait while automation is paused or for the camera, and fail with camera analysis", () => {
  for (const preset of [TONGUE, EMAIL]) {
    const camera = (vision) => status(preset, { live: { ...LIVE, vision } });
    assert.deepEqual(status(preset, { live: { ...LIVE, automation_enabled: false } }), {
      state: "paused",
      text: "Paused while automation is paused.",
    });
    assert.deepEqual(camera({ state: "idle" }), {
      state: "paused",
      text: "Waiting for the camera: choose one under Camera source in the Camera & mic tab.",
    });
    assert.deepEqual(camera({ state: "starting" }), {
      state: "paused",
      text: "Waiting for camera analysis to start.",
    });
    assert.deepEqual(camera({ state: "failed", message: "Vision could not load its models" }), {
      state: "unavailable",
      text: "Unavailable: camera analysis failed: Vision could not load its models.",
    });
    assert.equal(camera({ state: "failed" }).text, "Unavailable: camera analysis failed.");
    assert.equal(camera({ state: "running", latency_ms: 30 }).state, "on", "without a word, it trusts the camera");
    assert.equal(camera(undefined).state, "on", "without a camera status, it trusts the camera");
    const both = { ...LIVE, automation_enabled: false, vision: { state: "idle" } };
    assert.equal(status(preset, { live: both }).text, "Paused while automation is paused.", "automation first");
  }
});

test("each is unavailable while camera analysis cannot see what it watches", () => {
  const blind = (vision) => ({ live: { ...LIVE, vision: { ...CAMERA, ...vision } } });
  assert.deepEqual(status(TONGUE, blind({ mouths: false })), {
    state: "unavailable",
    text: "Unavailable: this camera analysis cannot see the mouth.",
  });
  assert.equal(status(EMAIL, blind({ mouths: false })).state, "on");
  assert.deepEqual(status(EMAIL, blind({ hands: false })), {
    state: "unavailable",
    text: "Unavailable: this camera analysis cannot see hands.",
  });
  assert.equal(status(TONGUE, blind({ hands: false })).state, "on");
});
