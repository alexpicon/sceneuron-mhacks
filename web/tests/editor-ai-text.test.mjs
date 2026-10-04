// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/ai-text.js: AI status labels, costs and ages, the latest scene, the write-only key, which
// settings a form sends, condition badges, and assistant errors in plain words.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const A = require("../editor/ai-text.js");

const settings = {
  available: true,
  enabled: false,
  consent: false,
  provider: "gemini",
  base_url: "https://generativelanguage.googleapis.com",
  vision_model: "gemini-3.8-flash",
  planner_model: "gemini-3.8-flash",
  vision_effort: "low",
  planner_effort: "medium",
  has_key: true,
  key_hint: "abcd",
  image_width: 768,
  input_price_per_mtok: 0.75,
  output_price_per_mtok: 3.75,
  price_known: true,
};

describe("status words and numbers", () => {
  test("the chip names every state and shows the message only for failures", () => {
    assert.equal(A.aiChipLabel({ state: "watching", message: "fine" }), "AI watching");
    assert.equal(A.aiChipLabel({ state: "needs_setup" }), "AI needs setup");
    const failed = { state: "failed", message: "the key was rejected" };
    assert.equal(A.aiChipLabel(failed), "AI failed: the key was rejected");
    assert.equal(A.aiChipLabel({ state: "failed", message: "x".repeat(100) }).length, "AI failed: ".length + 60);
    assert.equal(A.aiChipLabel(null), "AI off");
    assert.equal(A.aiState({ state: "dancing" }), "off");
  });

  test("costs keep small spends visible", () => {
    assert.equal(A.formatCost(0), "$0.00");
    assert.equal(A.formatCost(0.0042), "$0.004");
    assert.equal(A.formatCost(1.5), "$1.50");
    assert.equal(A.formatCost(undefined), "$0.00");
  });

  test("counts, ages, and seconds", () => {
    assert.deepEqual([A.formatCount(950), A.formatCount(14400), A.formatCount(2.5e6)], ["950", "14.4k", "2.5M"]);
    assert.equal(A.formatAge(2000), "just now");
    assert.equal(A.formatAge(12000), "12 s ago");
    assert.equal(A.formatAge(5 * 60000), "5 min ago");
    assert.equal(A.formatAge(3 * 3600000), "3 h ago");
    assert.equal(A.formatSeconds(2500), "2.5 s");
    assert.equal(A.formatSeconds(20000), "20 s");
  });

  test("the latest scene is the caption, or says scene understanding is not working once a failure cleared it", () => {
    assert.equal(A.sceneText("A person waves.", { state: "failed" }), "A person waves.");
    const broken = "Scene understanding is not working right now, so there is no scene description.";
    assert.equal(A.sceneText("", { state: "failed", message: "the key was rejected" }), broken);
    for (const state of ["off", "watching", "thinking", "paused", "needs_setup", "unknown"]) {
      assert.equal(A.sceneText("", { state }), "Nothing seen yet.", state);
    }
    assert.equal(A.sceneText("", null), "Nothing seen yet.");
  });
});

describe("the write-only key and the settings form", () => {
  test("the key's state shows only its last characters", () => {
    assert.equal(A.keyStatus(settings), "Key saved (...abcd)");
    assert.equal(A.keyStatus({ ...settings, key_hint: "...abcd" }), "Key saved (...abcd)");
    assert.equal(A.keyStatus({ ...settings, key_hint: "" }), "Key saved");
    assert.equal(A.keyStatus({ ...settings, has_key: false }), "No key saved");
    assert.equal(A.keyStatus(null), "No key saved");
    const fromEnv = { ...settings, key_source: "environment", key_hint: "...wxyz" };
    assert.equal(A.keyStatus(fromEnv), "Key from .env or the environment (...wxyz); change it there");
    assert.equal(A.keyRemovable(fromEnv), false);
    assert.equal(A.keyRemovable({ ...settings, key_source: "editor" }), true);
    const jev = { has_jev_key: true, jev_key_hint: "...6789", jev_key_source: "editor" };
    assert.equal(A.keyStatus(jev, "jev"), "Key saved (...6789)");
    assert.match(A.keyStatus({}, "jev"), /^No Jev key; quick judgments stay off/);
    assert.equal(A.keyRemovable(jev, "jev"), true);
  });

  test("only changed fields are sent, and the key only when one was typed", () => {
    const values = { ...settings, vision_model: "gemini-3.5-flash-lite", api_key: "" };
    assert.deepEqual(A.settingsUpdate(settings, values), { vision_model: "gemini-3.5-flash-lite" });
    assert.deepEqual(A.settingsUpdate(settings, { ...settings, api_key: "  sk-new  " }), { api_key: "sk-new" });
    assert.equal(A.settingsUpdate(settings, { ...settings, api_key: "   " }), null);
    assert.equal(A.settingsUpdate(settings, { provider: "gemini" }), null);
  });

  test("fields the form does not own are never sent", () => {
    const update = A.settingsUpdate(settings, { enabled: true, consent: true, has_key: false, key_hint: "zzzz" });
    assert.equal(update, null);
  });

  test("switching provider leaves out the models kept as they were, so the server picks the new provider's", () => {
    const base_url = "http://localhost:11434/v1";
    const values = { ...settings, provider: "openai-compatible", base_url, api_key: "" };
    assert.deepEqual(A.settingsUpdate(settings, values), { provider: "openai-compatible", base_url });
  });
});

describe("conditions and errors", () => {
  test("condition badges never rely on color alone", () => {
    const view = { judged: true, on: true };
    assert.deepEqual(A.conditionState(view, { enabled: true }), { state: "on", label: "On" });
    assert.deepEqual(A.conditionState({ judged: true, on: false }, {}), { state: "off", label: "Off" });
    assert.deepEqual(A.conditionState({ judged: false }, {}), { state: "waiting", label: "Not judged yet" });
    assert.deepEqual(A.conditionState(undefined, {}), { state: "unsaved", label: "Not saved yet" });
    assert.deepEqual(A.conditionState(view, { enabled: false }), { state: "disabled", label: "Turned off" });
  });

  test("assistant errors keep the server's words and say what to do", () => {
    const error = (status, message) => Object.assign(new Error(message), { status });
    const refused = A.planErrorMessage(error(502, "invalid key"));
    assert.match(refused, /refused the request: invalid key\. Check the provider/);
    assert.match(A.planErrorMessage(error(429, "slow down")), /busy or the hourly budget/);
    assert.match(A.planErrorMessage(error(504, "timeout")), /took too long/);
    assert.match(A.planErrorMessage(error(503, "no key")), /not ready: no key/);
    assert.equal(A.planErrorMessage(error(0, "Cannot reach Sceneuron.")), "Cannot reach Sceneuron.");
    assert.match(A.planErrorMessage(error(400, "too long")), /could not make a plan: too long/);
  });
});

test("the quick line says what quick judgments do", () => {
  assert.equal(A.quickLine(null), "Off");
  assert.equal(A.quickLine({ quick_state: "off", quick_message: "add a Jev key" }), "Off: add a Jev key");
  assert.equal(A.quickLine({ quick_state: "watching", quick_calls: 12, quick_latency_ms: 181.4 }), "12 judgments, last in 181 ms");
  assert.equal(
    A.quickLine({ quick_state: "failed", quick_message: "quick judgments stopped: bad key" }),
    "Failed: quick judgments stopped: bad key",
  );
});

test("typed keys go along with changed settings, and blank ones do not", () => {
  const update = A.settingsUpdate({ provider: "gemini" }, { provider: "gemini", api_key: " ", jev_api_key: " jk " });
  assert.deepEqual(update, { jev_api_key: "jk" });
});
