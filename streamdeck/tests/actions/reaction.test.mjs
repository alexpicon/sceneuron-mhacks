// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the "Reaction" key in the built plugin: drawn as its preset before pairing, a press that
// plays the preset through /api/flood and lights the key while it plays, a reaction played elsewhere
// lighting only its own preset's keys, and the alert sign when a press cannot work.

import assert from "node:assert/strict";
import { test } from "node:test";

import { CONTROL_TOKEN } from "../fake-sceneuron.mjs";
import { alert, appear, bundleExists, dim, image, keyDown, lit, words } from "../fake-stream-deck.mjs";
import { startPlugin } from "../harness.mjs";

const skip = bundleExists() ? false : "the plugin is not built; run npm run build";
const paired = (sceneuron) => ({ port: sceneuron.port, token: CONTROL_TOKEN });
const EMAIL = { reaction: "email" };
const SHIP = { reaction: "ship-it" };

test("before pairing, the key shows its preset's emoji and name, and says Not paired", { skip }, async (t) => {
  const { deck } = await startPlugin(t, {}, () => ({}));
  const unpaired = (svg) =>
    words(svg) === "📧 Email Not paired" && !dim(svg) && !lit(svg);
  await deck.after(appear("reaction", "key-r", EMAIL), image("key-r", unpaired), "the unpaired email key");
});

test("a press plays the preset, and the key lights up while it plays", { skip }, async (t) => {
  const { sceneuron, deck } = await startPlugin(t, {}, paired);
  const plain = image("key-r", (svg) => words(svg) === "📧 Email" && !dim(svg) && !lit(svg));
  await deck.after(appear("reaction", "key-r", EMAIL), plain, "the live email key");
  const playing = image("key-r", (svg) => lit(svg) && words(svg) === "📧 Email Playing");
  await deck.after(keyDown("reaction", "key-r", EMAIL), playing, "the lit key");
  assert.deepEqual(sceneuron.posted("/api/flood"), [{ flood: "email" }]);
  assert.deepEqual(sceneuron.view.flood, { flood: "email", nonce: 1, left_ms: 4000 });
  const from = deck.messages.length;
  sceneuron.change((view) => {
    delete view.flood;
  });
  await deck.next(plain, "the key no longer lit once Sceneuron says it ended", from);
});

test("a reaction played elsewhere lights its keys for as long as Sceneuron says", { skip }, async (t) => {
  const { sceneuron, deck } = await startPlugin(t, {}, paired);
  await deck.after(appear("reaction", "key-e", EMAIL), image("key-e", (svg) => !dim(svg)), "the email key");
  await deck.after(appear("reaction", "key-s", SHIP), image("key-s", (svg) => !dim(svg)), "the ship it key");
  const from = deck.messages.length;
  sceneuron.change((view) => {
    view.flood = { flood: "ship-it", nonce: 7, left_ms: 300 };
  });
  const lighted = await deck.next(image("key-s", lit), "the lit ship it key", from);
  const unlit = image("key-s", (svg) => !lit(svg));
  await deck.next(unlit, "the ship it key after 300 ms", deck.messages.indexOf(lighted));
  const emailDrawings = deck.messages.slice(from).filter(image("key-e", lit));
  assert.equal(emailDrawings.length, 0, "only the preset that plays lights up");
});

test("a key with no reaction or an unknown one shows the alert sign", { skip }, async (t) => {
  const { sceneuron, deck } = await startPlugin(t, {}, paired);
  const choose = image("key-r", (svg) => words(svg) === "Choose a reaction");
  await deck.after(appear("reaction", "key-r", {}), choose, "the key asking for a reaction");
  await deck.after(keyDown("reaction", "key-r", {}), alert("key-r"), "the alert for a key with no reaction");
  assert.deepEqual(sceneuron.posted("/api/flood"), []);
  const odd = { reaction: "confetti" };
  const missing = image("key-r", (svg) => words(svg) === "confetti Not found" && dim(svg));
  await deck.after(appear("reaction", "key-r", odd), missing, "the key for a preset the plugin lacks");
  await deck.after(keyDown("reaction", "key-r", odd), alert("key-r"), "the alert Sceneuron's refusal brings");
  assert.deepEqual(sceneuron.posted("/api/flood"), [{ flood: "confetti" }]);
});
