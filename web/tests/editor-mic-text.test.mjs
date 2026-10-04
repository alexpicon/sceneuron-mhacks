// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/mic-text.js: what each source that carries audio is, which microphone voice control and subtitles
// hear when none, one, or several sources have the Sceneuron Voice filter, outside OBS, and what to do next in the
// Camera & mic tab and in the AI tab.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const M = require("../editor/mic-text.js");

const source = (name, fields) =>
  Object.assign({ uuid: `uuid-${name}`, name, kind: "coreaudio_input_capture", global: false, voice: false,
    in_use: false }, fields);
const reply = (...sources) => ({ available: true, sources });

describe("the list", () => {
  test("global audio devices say so, other sources show their type", () => {
    assert.equal(M.micKind(source("Mic/Aux", { global: true })), "Global audio device");
    assert.equal(M.micKind(source("Shure", { kind: "av_capture_input" })), "av_capture_input");
    assert.equal(M.micKind(source("Unknown", { kind: undefined })), "");
  });

  test("names join as in a sentence", () => {
    assert.equal(M.listNames(["A"]), "A");
    assert.equal(M.listNames(["A", "B"]), "A and B");
    assert.equal(M.listNames(["A", "B", "C"]), "A, B, and C");
  });
});

describe("which microphone is heard", () => {
  test("nothing is said until the list loads", () => {
    assert.deepEqual(M.micSummary(null), { state: "unknown", uuid: null, text: "" });
    assert.equal(M.micNote(null, "ai"), "");
  });

  test("outside OBS there is no microphone", () => {
    const summary = M.micSummary({ available: false, sources: [] });
    assert.equal(summary.state, "unavailable");
    assert.equal(summary.text, "Sceneuron is not running inside OBS, so there is no microphone to listen to.");
    assert.equal(M.micAdvice(summary.state, "ai"), "");
  });

  test("without the filter, nothing is heard, and both places say where to add it", () => {
    const none = reply(source("Mic/Aux", { global: true }), source("Desktop Audio", { global: true }));
    assert.equal(M.micSummary(none).state, "none");
    assert.equal(
      M.micNote(none, "tab"),
      "No microphone has the Sceneuron Voice filter yet, so voice control and subtitles hear nothing. Choose your " +
        "microphone below.",
    );
    assert.equal(
      M.micNote(none, "ai"),
      "No microphone has the Sceneuron Voice filter yet, so voice control and subtitles hear nothing. Choose your " +
        "microphone in the Camera & mic tab, or in OBS open the Audio Mixer, choose Filters on your microphone, " +
        "and add the Sceneuron Voice filter.",
    );
    assert.equal(M.micSummary(reply()).state, "none", "no sources at all");
  });

  test("one filter is the one heard, whether or not it sent audio yet", () => {
    const one = reply(source("Mic/Aux", { voice: true }), source("Desktop Audio"));
    assert.deepEqual(M.micSummary(one), {
      state: "one",
      uuid: "uuid-Mic/Aux",
      text: "Sceneuron listens to Mic/Aux for voice control and subtitles.",
    });
    assert.equal(M.micNote(one, "ai"), "Sceneuron listens to Mic/Aux for voice control and subtitles.");
  });

  test("of several filters, the one in use is named and the others are not used", () => {
    const several = reply(
      source("Mic/Aux", { voice: true }),
      source("Shure MV7", { voice: true, in_use: true }),
      source("Webcam", { voice: true }),
    );
    const summary = M.micSummary(several);
    assert.equal(summary.uuid, "uuid-Shure MV7");
    assert.equal(
      summary.text,
      "Sceneuron listens to Shure MV7 for voice control and subtitles. Mic/Aux and Webcam also have the Sceneuron " +
        "Voice filter, which is not used while Shure MV7 has it.",
    );
    const two = reply(source("A", { voice: true, in_use: true }), source("B", { voice: true }));
    assert.match(M.micSummary(two).text, /B also has the Sceneuron Voice filter/);
    assert.equal(M.micAdvice("several", "tab"), "Choose one below to keep the filter on that one only.");
    assert.equal(M.micAdvice("several", "ai"), "Choose one in the Camera & mic tab to keep the filter on that one only.");
  });

  test("of several filters before any sent audio, the first to send it will be heard", () => {
    const waiting = reply(source("Mic/Aux", { voice: true }), source("Webcam", { voice: true }));
    const summary = M.micSummary(waiting);
    assert.equal(summary.uuid, null);
    assert.equal(
      summary.text,
      "Mic/Aux and Webcam have the Sceneuron Voice filter. Sceneuron hears one microphone at a time: the first of " +
        "them to send audio once voice control or subtitles start.",
    );
  });
});
