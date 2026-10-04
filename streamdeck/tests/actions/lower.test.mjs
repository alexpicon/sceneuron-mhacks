// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the "Lower cards" key in the built plugin: one request per deck with a card up when
// no deck is chosen, only the chosen deck otherwise, and the alert sign while it cannot act.

import assert from "node:assert/strict";
import { test } from "node:test";

import { CONTROL_TOKEN } from "../fake-sceneuron.mjs";
import { alert, appear, bundleExists, image, keyDown, lit } from "../fake-stream-deck.mjs";
import { startPlugin } from "../harness.mjs";

const skip = bundleExists() ? false : "the plugin is not built; run npm run build";
const paired = (sceneuron) => ({ port: sceneuron.port, token: CONTROL_TOKEN });

/** A plugin next to a Sceneuron with a card up in both of its decks. */
async function bothRaised(t) {
  const started = await startPlugin(t, {}, paired);
  started.sceneuron.change((view) => {
    view.decks[0].raised = "p3";
    view.decks[1].raised = "agree";
  });
  return started;
}

test("with no deck chosen a press lowers every deck with a card up, one request each", { skip }, async (t) => {
  const { sceneuron, deck } = await bothRaised(t);
  await deck.after(appear("lower", "key-1", {}), image("key-1", lit), "the lit key");
  await deck.after(keyDown("lower", "key-1", {}), image("key-1", (svg) => !lit(svg)), "the unframed key");
  assert.deepEqual(sceneuron.posted("/api/card"), [
    { deck: "poker", action: "lower" },
    { deck: "meeting", action: "lower" },
  ]);
  assert.deepEqual(
    sceneuron.view.decks.map((entry) => entry.raised),
    [null, null],
  );
});

test("with a deck chosen a press lowers only that deck", { skip }, async (t) => {
  const { sceneuron, deck } = await bothRaised(t);
  const settings = { deck: "meeting" };
  await deck.after(appear("lower", "key-1", settings), image("key-1", lit), "the lit key");
  await deck.after(keyDown("lower", "key-1", settings), image("key-1", (svg) => !lit(svg)), "the unframed key");
  assert.deepEqual(sceneuron.posted("/api/card"), [{ deck: "meeting", action: "lower" }]);
  assert.equal(sceneuron.view.decks[0].raised, "p3");
});

test("a press shows the alert sign while the plugin is not paired", { skip }, async (t) => {
  const { sceneuron, deck } = await startPlugin(t, {}, () => ({}));
  await deck.after(appear("lower", "key-1", {}), image("key-1", (svg) => />Not paired</.test(svg)), "the key");
  await deck.after(keyDown("lower", "key-1", {}), alert("key-1"), "the alert sign");
  assert.equal(sceneuron.requests.length, 0);
});
