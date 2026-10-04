// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/voice-guide-text.js: the guide's sections as the Voice section lists them, with the ways to say
// each command quoted as they are said in the current mode, and the check a typed command must pass.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const G = require("../editor/voice-guide-text.js");

const GROUPS = [
  {
    title: "Reactions",
    addressed: true,
    entries: [{ say: ["mind blown", "play mind blown"], does: "plays 🤯 Mind blown at once" }],
  },
  {
    title: "Trigger phrases",
    addressed: false,
    entries: [{ say: ["ship it"], does: "plays 🚀 Ship it while automation runs, at most once per cooldown" }],
  },
  {
    title: "Cards",
    addressed: true,
    entries: [{ say: [], does: "Turn a deck's voice on in the Cards tab to raise its cards by name." }],
  },
];

describe("guideSections", () => {
  test("in always-on mode, addressed words start with the wake word and trigger phrases need none", () => {
    const sections = G.guideSections({ wake_word: "Jarvis", mode: "always_on", groups: GROUPS });
    assert.deepEqual(sections[0], {
      title: "Reactions",
      note: 'Start with "Jarvis", or type them below.',
      lines: [{ say: ['"Jarvis, mind blown"', '"Jarvis, play mind blown"'], does: "plays 🤯 Mind blown at once" }],
    });
    assert.deepEqual(sections[1].lines[0].say, ['"ship it"']);
    assert.equal(sections[1].note, "Anywhere in what you say, without the wake word.");
    assert.deepEqual(sections[2].lines, [
      { say: [], does: "Turn a deck's voice on in the Cards tab to raise its cards by name." },
    ]);
  });

  test("with push to talk, words are said as they are, and trigger phrases wait for always-on mode", () => {
    const sections = G.guideSections({ wake_word: "Sceneuron", mode: "push_to_talk", groups: GROUPS });
    assert.deepEqual(sections[0].lines[0].say, ['"mind blown"', '"play mind blown"']);
    assert.equal(sections[0].note, "Hold to talk while you say them, or type them below.");
    assert.equal(sections[1].note, "Only while voice control listens always, after the wake word or not.");
  });

  test("a guide not loaded yet has no sections", () => {
    assert.deepEqual(G.guideSections(null), []);
    assert.deepEqual(G.guideSections({}), []);
  });
});

describe("commandProblem", () => {
  test("a typed command is 1 to 200 characters, as the server takes it", () => {
    assert.equal(G.commandProblem("hacker theme"), null);
    assert.equal(G.commandProblem("  x  "), null);
    assert.equal(G.commandProblem("🤯".repeat(200)), null, "characters, not bytes");
    for (const bad of ["", "   ", null, "a".repeat(201)]) {
      assert.equal(G.commandProblem(bad), "Type 1 to 200 characters.", JSON.stringify(bad));
    }
    assert.equal(G.MAX_COMMAND_CHARS, 200);
  });
});
