// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the "Push to talk" key in the built plugin: holding and letting go, letting go when
// the key disappears or its device is unplugged while held, a let-go owed while the link was down
// that goes out once it is back, and what the key shows of voice control, which Sceneuron's word
// on whether push to talk is held wins over the key.

import assert from "node:assert/strict";
import { test } from "node:test";

import { CONTROL_TOKEN } from "../fake-sceneuron.mjs";
import { sampleView } from "../fake-engine.mjs";
import {
  alert,
  appear,
  bundleExists,
  deviceDisconnect,
  dim,
  disappear,
  image,
  keyDown,
  keyUp,
  lit,
  words,
} from "../fake-stream-deck.mjs";
import { startPlugin, waitFor } from "../harness.mjs";

const skip = bundleExists() ? false : "the plugin is not built; run npm run build";
const paired = (sceneuron) => ({ port: sceneuron.port, token: CONTROL_TOKEN });
const HELD = { down: true };
const LET_GO = { down: false };

const idle = (svg) => words(svg) === "Talk" && !lit(svg) && !dim(svg);
const talking = (svg) => words(svg).endsWith("Talk Talking") && lit(svg);

/** A plugin with a live talk key at "talk-1". */
async function talkKey(t, options = {}) {
  const started = await startPlugin(t, options, paired);
  await started.deck.after(appear("talk", "talk-1", {}), image("talk-1", idle), "the live talk key");
  return started;
}

test("holding the key holds push to talk, and letting go lets go", { skip }, async (t) => {
  const { sceneuron, deck } = await talkKey(t);
  await deck.after(keyDown("talk", "talk-1", {}), image("talk-1", talking), "the held key");
  await waitFor(() => sceneuron.talking, "push to talk held");
  await deck.after(keyUp("talk", "talk-1", {}), image("talk-1", idle), "the let-go key");
  await waitFor(() => !sceneuron.talking, "push to talk let go");
  assert.deepEqual(sceneuron.posted("/api/voice/talk"), [HELD, LET_GO]);
});

test("a key that disappears while held lets go", { skip }, async (t) => {
  const { sceneuron, deck } = await talkKey(t);
  await deck.after(keyDown("talk", "talk-1", {}), image("talk-1", talking), "the held key");
  deck.send(disappear("talk", "talk-1", {}));
  await waitFor(() => sceneuron.posted("/api/voice/talk").length === 2, "the let-go");
  assert.deepEqual(sceneuron.posted("/api/voice/talk"), [HELD, LET_GO]);
  assert.equal(sceneuron.talking, false);
});

test("a device unplugged while its key is held lets go", { skip }, async (t) => {
  const { sceneuron, deck } = await talkKey(t);
  await deck.after(keyDown("talk", "talk-1", {}), image("talk-1", talking), "the held key");
  deck.send(deviceDisconnect());
  await waitFor(() => sceneuron.posted("/api/voice/talk").length === 2, "the let-go");
  assert.deepEqual(sceneuron.posted("/api/voice/talk"), [HELD, LET_GO]);
});

test("a key let go while Sceneuron is unreachable lets go once it is back", { skip }, async (t) => {
  const { sceneuron, deck } = await talkKey(t);
  await deck.after(keyDown("talk", "talk-1", {}), image("talk-1", talking), "the held key");
  sceneuron.stopProving();
  sceneuron.endStreams();
  await deck.next(image("talk-1", (svg) => />Pair again</.test(svg)), "the refused key");
  await deck.after(keyUp("talk", "talk-1", {}), alert("talk-1"), "the alert for a let-go that cannot go out");
  assert.deepEqual(sceneuron.posted("/api/voice/talk"), [HELD]);
  sceneuron.resumeProving();
  await waitFor(() => sceneuron.posted("/api/voice/talk").length === 2, "the owed let-go");
  assert.deepEqual(sceneuron.posted("/api/voice/talk"), [HELD, LET_GO]);
  assert.equal(sceneuron.talking, false);
});

test("while voice control is off the key says so, and a press sends nothing", { skip }, async (t) => {
  const view = { ...sampleView(), voice: { enabled: false, talking: false } };
  const { sceneuron, deck } = await startPlugin(t, { view }, paired);
  const off = image("talk-1", (svg) => />Voice off</.test(svg) && dim(svg));
  await deck.after(appear("talk", "talk-1", {}), off, "the key saying Voice off");
  await deck.after(keyDown("talk", "talk-1", {}), alert("talk-1"), "the alert sign");
  deck.send(keyUp("talk", "talk-1", {}));
  sceneuron.change((shown) => {
    shown.voice.enabled = true;
  });
  await deck.next(image("talk-1", (svg) => />Voice on</.test(svg) && !dim(svg)), "the key saying Voice on");
  assert.deepEqual(sceneuron.posted("/api/voice/talk"), []);
});

test("push to talk held elsewhere lights the key", { skip }, async (t) => {
  const view = { ...sampleView(), voice: { enabled: true, talking: false } };
  const { sceneuron, deck } = await startPlugin(t, { view }, paired);
  await deck.after(appear("talk", "talk-1", {}), image("talk-1", (svg) => !lit(svg)), "the idle key");
  sceneuron.change((shown) => {
    shown.voice.talking = true;
  });
  await deck.next(image("talk-1", talking), "the key lit by the editor's hold");
  assert.deepEqual(sceneuron.posted("/api/voice/talk"), []);
});

test("a held key that Sceneuron says no longer holds push to talk stops looking held", { skip }, async (t) => {
  const view = { ...sampleView(), voice: { enabled: true, talking: false } };
  const { sceneuron, deck } = await startPlugin(t, { view }, paired);
  const ready = (svg) => />Voice on</.test(svg) && !lit(svg) && !dim(svg);
  await deck.after(appear("talk", "talk-1", {}), image("talk-1", ready), "the live idle key");
  await deck.after(keyDown("talk", "talk-1", {}), image("talk-1", talking), "the held key");
  const from = deck.messages.length;
  sceneuron.change((shown) => {
    shown.voice.talking = false;
  });
  await deck.next(image("talk-1", ready), "the key no longer lit", from);
  assert.deepEqual(sceneuron.posted("/api/voice/talk"), [HELD]);
});
