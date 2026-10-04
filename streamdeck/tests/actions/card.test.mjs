// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the "Raise a card" key in the built plugin: drawn as the live card, a press that
// toggles the card through /api/card, and the alert sign when a press cannot work.

import assert from "node:assert/strict";
import { test } from "node:test";

import { CONTROL_TOKEN } from "../fake-sceneuron.mjs";
import { alert, appear, bundleExists, dim, image, keyDown, lit } from "../fake-stream-deck.mjs";
import { startPlugin } from "../harness.mjs";

const skip = bundleExists() ? false : "the plugin is not built; run npm run build";
const paired = (sceneuron) => ({ port: sceneuron.port, token: CONTROL_TOKEN });

test("a press raises the card and the next press lowers it", { skip }, async (t) => {
  const { sceneuron, deck } = await startPlugin(t, {}, paired);
  const settings = { deck: "poker", card: "p5" };
  const live = image("key-1", (svg) => />5<\/text>/.test(svg) && !dim(svg) && !lit(svg));
  await deck.after(appear("card", "key-1", settings), live, "the live card");
  await deck.after(keyDown("card", "key-1", settings), image("key-1", lit), "the lit frame");
  assert.equal(sceneuron.view.decks[0].raised, "p5");
  await deck.after(keyDown("card", "key-1", settings), live, "the card without its frame");
  assert.equal(sceneuron.view.decks[0].raised, null);
  const toggle = { ...settings, action: "toggle" };
  assert.deepEqual(sceneuron.posted("/api/card"), [toggle, toggle]);
});

test("a press that cannot work shows the alert sign", { skip }, async (t) => {
  const { sceneuron, deck } = await startPlugin(t, {}, paired);
  const missing = { deck: "poker", card: "p13" };
  await deck.after(appear("card", "key-1", missing), image("key-1", (svg) => />Not found</.test(svg)), "the key");
  await deck.after(keyDown("card", "key-1", missing), alert("key-1"), "the alert for a card Sceneuron lacks");
  assert.deepEqual(sceneuron.posted("/api/card"), [{ ...missing, action: "toggle" }]);
  await deck.after(keyDown("card", "key-1", {}), alert("key-1"), "the alert for a key with no card");
  assert.equal(sceneuron.posted("/api/card").length, 1);
});
