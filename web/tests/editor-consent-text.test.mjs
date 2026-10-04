// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/consent-text.js: consent gating for watching and for camera images, the consent wording and where
// data goes, and Google's free-tier note wherever data goes to the Gemini API, the assistant's included.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const C = require("../editor/consent-text.js");

const settings = {
  available: true,
  enabled: false,
  consent: false,
  provider: "openai-compatible",
  base_url: "https://integrate.api.nvidia.com/v1",
  vision_model: "meta/llama-3.2-90b-vision-instruct",
  planner_model: "meta/llama-3.2-90b-vision-instruct",
  has_key: true,
  image_width: 768,
};
const gemini = {
  ...settings,
  provider: "gemini",
  base_url: "https://generativelanguage.googleapis.com",
  vision_model: "gemini-3.8-flash",
  planner_model: "gemini-3.8-flash",
};
const freeTier = (sent) =>
  `With a free-tier key (a Google project without billing), Google may use ${sent} to improve its products, and ` +
  "people may review it. A key from a project with billing is not used that way.";

describe("consent gating", () => {
  test("watching needs consent, and the switch says why it is off", () => {
    assert.deepEqual(C.watchSwitch(settings), {
      checked: false,
      disabled: true,
      note: "Tick the consent box above first.",
    });
    const consented = C.watchSwitch({ ...settings, consent: true, enabled: true });
    assert.equal(consented.checked, true);
    assert.equal(consented.disabled, false);
    assert.equal(C.watchSwitch({ ...settings, consent: false, enabled: true }).checked, false);
  });

  test("withdrawing consent also stops watching", () => {
    assert.deepEqual(C.consentUpdate(true), { consent: true });
    assert.deepEqual(C.consentUpdate(false), { consent: false, enabled: false });
  });

  test("the assistant may use the camera only with consent", () => {
    assert.equal(C.cameraChoice(settings).disabled, true);
    assert.equal(C.cameraChoice(settings).note, "Needs your consent in the AI tab. Plans work without it.");
    assert.equal(C.cameraChoice({ ...settings, consent: true }).disabled, false);
    assert.equal(C.cameraChoice({ ...settings, consent: true }).note, "Sends one still image with your request.");
    assert.equal(C.cameraChoice(null).disabled, true);
  });

  test("the wording says what is sent, where, and how often", () => {
    const points = Object.fromEntries(C.consentPoints(settings, { min_interval_ms: 2500, refresh_ms: 20000 }));
    assert.match(points.What, /768 pixels wide/);
    assert.equal(points.Where, "integrate.api.nvidia.com (an OpenAI-compatible service).");
    assert.match(points.When, /Only while "Watch the camera with AI" is on: at most one image every 2.5 s/);
    assert.match(points.When, /20 s/);
    assert.match(points.Key, /stays on this computer/);
    assert.equal(points.Google, undefined, "a service away from Google's address gets no Gemini note");
    const local = Object.fromEntries(C.consentPoints({ ...settings, base_url: "http://localhost:11434/v1" }, null));
    assert.equal(local.Where, "localhost:11434, a model on your own computer.");
  });

  test("each provider is named where data goes", () => {
    assert.equal(C.destination(gemini), "generativelanguage.googleapis.com (Google Gemini)");
    assert.equal(C.destination(settings), "integrate.api.nvidia.com (an OpenAI-compatible service)");
    const unknown = { provider: "other", base_url: "https://example.com" };
    assert.equal(C.destination(unknown), "example.com (the AI provider)");
  });
});

describe("Google's free-tier note", () => {
  test("the consent names Google's address and what a free-tier key lets Google do, right after it", () => {
    const points = C.consentPoints(gemini, null);
    assert.deepEqual(points.map(([label]) => label), ["What", "Where", "Google", "When", "Assistant", "Key"]);
    const named = Object.fromEntries(points);
    assert.equal(named.Where, "generativelanguage.googleapis.com (Google Gemini).");
    assert.equal(named.Google, freeTier("what is sent, camera images included,"));
  });

  test("the note follows the Gemini API's address, not the provider's name", () => {
    assert.equal(C.dataUse(gemini), freeTier("what is sent"));
    const withPort = { ...gemini, base_url: "https://generativelanguage.googleapis.com:443/" };
    assert.equal(C.dataUse(withPort), freeTier("what is sent"));
    const openAiAtGoogle = {
      provider: "openai-compatible",
      base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
    };
    assert.equal(C.dataUse(openAiAtGoogle), freeTier("what is sent"), "Google's OpenAI-compatible endpoint");
    const points = Object.fromEntries(C.consentPoints(openAiAtGoogle, null));
    assert.equal(points.Google, freeTier("what is sent, camera images included,"));
    assert.equal(C.dataUse(settings), "");
    assert.equal(C.dataUse({ ...settings, provider: "openai-compatible", base_url: "https://example.com/v1" }), "");
    assert.equal(C.dataUse(null), "");
    assert.equal(C.dataUse({ ...gemini, base_url: "not a url" }), "");
  });

  test("a stand-in for Gemini that is not Google's address gets no note", () => {
    for (const base_url of ["http://127.0.0.1:8090", "http://192.168.1.20:8090", "https://gemini-proxy.example.com"]) {
      const standIn = { ...gemini, base_url };
      assert.equal(C.dataUse(standIn), "", base_url);
      assert.equal(Object.fromEntries(C.consentPoints(standIn, null)).Google, undefined, base_url);
    }
    const local = Object.fromEntries(C.consentPoints({ ...gemini, base_url: "http://127.0.0.1:8090" }, null));
    assert.equal(local.Where, "127.0.0.1:8090, a model on your own computer.");
  });

  test("the assistant's camera choice says it with consent, and before it, since plans go to Google either way", () => {
    const consented = C.cameraChoice({ ...gemini, consent: true }).note;
    assert.equal(consented, `Sends one still image with your request. ${freeTier("what is sent")}`);
    const before = C.cameraChoice(gemini);
    assert.equal(before.disabled, true);
    assert.equal(
      before.note,
      `Needs your consent in the AI tab. Plans work without it. ${freeTier("your request and project")}`,
    );
  });
});
