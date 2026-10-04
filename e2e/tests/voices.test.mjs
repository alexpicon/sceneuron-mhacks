// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises how the live voice test chooses a voice from the account's free voice list: a wanted voice when the
// account has it, George when listed, else a premade voice, else any.

import assert from "node:assert/strict";
import { test } from "node:test";

import { DEFAULT_VOICE_ID, chooseVoice } from "../voices.mjs";

const george = { voice_id: DEFAULT_VOICE_ID, name: "George", category: "premade" };
const rachel = { voice_id: "rachel1", name: "Rachel", category: "premade" };
const mine = { voice_id: "mine1", name: "Mine", category: "cloned" };

test("a wanted voice is used when the account lists it, or may list it on a later page", () => {
  assert.equal(chooseVoice({ voices: [george, mine] }, "mine1"), mine);
  assert.deepEqual(chooseVoice({ voices: [george], has_more: true }, "later1"), { voice_id: "later1", name: "" });
  assert.equal(chooseVoice({ voices: [george] }, "gone1"), george, "a wanted voice the account lacks is replaced");
});

test("George comes first, then a premade voice, then any", () => {
  assert.equal(chooseVoice({ voices: [mine, rachel, george] }), george);
  assert.equal(chooseVoice({ voices: [mine, rachel] }), rachel);
  assert.equal(chooseVoice({ voices: [mine] }), mine);
  assert.equal(chooseVoice({ voices: [] }), null);
  assert.equal(chooseVoice(null), null);
});
