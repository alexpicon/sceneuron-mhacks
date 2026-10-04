// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the "Pause automation" key in the built plugin: its two states follow Sceneuron, and a
// press asks for the opposite of what is on.

import assert from "node:assert/strict";
import { test } from "node:test";

import { CONTROL_TOKEN } from "../fake-sceneuron.mjs";
import { appear, bundleExists, image, keyDown } from "../fake-stream-deck.mjs";
import { startPlugin } from "../harness.mjs";

const skip = bundleExists() ? false : "the plugin is not built; run npm run build";

function state(value) {
  return (message) => message.event === "setState" && message.context === "auto-1" && message.payload.state === value;
}

test("the automation key follows Sceneuron and flips it", { skip }, async (t) => {
  const { sceneuron, deck } = await startPlugin(t, {}, (server) => ({ port: server.port, token: CONTROL_TOKEN }));
  await deck.after(appear("automation", "auto-1", {}), image("auto-1", (svg) => />On</.test(svg)), "the running state");
  sceneuron.change((view) => {
    view.automation_enabled = false;
  });
  await deck.next(state(1), "the paused state");
  await deck.after(keyDown("automation", "auto-1", {}, 1), state(0), "the resumed state");
  assert.equal(sceneuron.view.automation_enabled, true);
  assert.deepEqual(sceneuron.posted("/api/automation"), [{ enabled: true }]);
});
