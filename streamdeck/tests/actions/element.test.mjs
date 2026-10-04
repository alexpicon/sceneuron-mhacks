// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the "Show or hide" key in the built plugin: the element's state drawn on the key, and
// a press that toggles it through /api/control/element, which holds the change.

import assert from "node:assert/strict";
import { test } from "node:test";

import { CONTROL_TOKEN } from "../fake-sceneuron.mjs";
import { appear, bundleExists, dim, image, keyDown, lit } from "../fake-stream-deck.mjs";
import { startPlugin } from "../harness.mjs";

const skip = bundleExists() ? false : "the plugin is not built; run npm run build";
const paired = (sceneuron) => ({ port: sceneuron.port, token: CONTROL_TOKEN });

test("a press shows a hidden element and holds it, and the next hides it again", { skip }, async (t) => {
  const { sceneuron, deck } = await startPlugin(t, {}, paired);
  const settings = { element: "brb" };
  const hidden = (svg) => />Hidden</.test(svg) && !lit(svg) && !dim(svg);
  await deck.after(appear("element", "key-1", settings), image("key-1", hidden), "the hidden element");
  const shown = (svg) => />Shown</.test(svg) && />Held</.test(svg) && lit(svg);
  await deck.after(keyDown("element", "key-1", settings), image("key-1", shown), "the shown element");
  const hiddenHeld = (svg) => hidden(svg) && />Held</.test(svg);
  await deck.after(keyDown("element", "key-1", settings), image("key-1", hiddenHeld), "the hidden element, held");
  const toggle = { element: "brb", action: "toggle" };
  assert.deepEqual(sceneuron.posted("/api/control/element"), [toggle, toggle]);
});
