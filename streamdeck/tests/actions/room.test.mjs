// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the "Reveal votes" and "Next round" keys in the built plugin: each posts its action to
// the room, waiting longer than most requests for a room that is slow to answer, a press with no
// room joined changes nothing and turns the keys to "No room" for a few seconds, and a view that
// carries the room's counts shows how many voted and stops a press without a room.

import assert from "node:assert/strict";
import { test } from "node:test";

import { CONTROL_TOKEN } from "../fake-sceneuron.mjs";
import { sampleView } from "../fake-engine.mjs";
import { alert, appear, bundleExists, dim, image, keyDown, lit, words } from "../fake-stream-deck.mjs";
import { startPlugin, waitFor } from "../harness.mjs";

const skip = bundleExists() ? false : "the plugin is not built; run npm run build";
const paired = (sceneuron) => ({ port: sceneuron.port, token: CONTROL_TOKEN });

const plain = (title) => (svg) => words(svg) === title && !dim(svg);
const noRoom = (svg) => words(svg).endsWith(" No room") && dim(svg);
const revealed = (svg) => words(svg) === "Reveal votes Revealed" && lit(svg);

test("in a room, Reveal votes reveals and Next round starts the next round", { skip }, async (t) => {
  const { sceneuron, deck } = await startPlugin(t, { room: true }, paired);
  await deck.after(appear("reveal", "reveal-1", {}), image("reveal-1", plain("Reveal votes")), "the reveal key");
  await deck.after(appear("next-round", "next-1", {}), image("next-1", plain("Next round")), "the next round key");
  deck.send(keyDown("reveal", "reveal-1", {}));
  await waitFor(() => sceneuron.room.revealed, "the reveal");
  deck.send(keyDown("next-round", "next-1", {}));
  await waitFor(() => sceneuron.room.round === 2, "the next round");
  assert.deepEqual(sceneuron.room, { revealed: false, round: 2 });
  assert.deepEqual(sceneuron.posted("/api/room"), [{ action: "reveal" }, { action: "next" }]);
  assert.deepEqual(deck.messages.filter((message) => alert("reveal-1")(message) || alert("next-1")(message)), []);
});

test("with no room joined a press changes nothing, shows the alert, and the keys say No room", { skip }, async (t) => {
  const { sceneuron, deck } = await startPlugin(t, {}, paired);
  await deck.after(appear("reveal", "reveal-1", {}), image("reveal-1", plain("Reveal votes")), "the reveal key");
  await deck.after(appear("next-round", "next-1", {}), image("next-1", plain("Next round")), "the next round key");
  await deck.after(keyDown("reveal", "reveal-1", {}), alert("reveal-1"), "the alert sign");
  await deck.next(image("reveal-1", noRoom), "the reveal key saying No room");
  await deck.next(image("next-1", noRoom), "the next round key saying No room");
  assert.equal(sceneuron.room, null);
  assert.deepEqual(sceneuron.posted("/api/room"), [{ action: "reveal" }]);
});

test("No room lasts a few seconds, then the keys look pressable again", { skip }, async (t) => {
  const { deck } = await startPlugin(t, {}, paired);
  await deck.after(appear("reveal", "reveal-1", {}), image("reveal-1", plain("Reveal votes")), "the reveal key");
  await deck.after(keyDown("reveal", "reveal-1", {}), image("reveal-1", noRoom), "the key saying No room");
  const from = deck.messages.length;
  await deck.next(image("reveal-1", plain("Reveal votes")), "the reveal key back to its plain face", from);
});

test("a room that takes longer than most requests to answer still counts as done", { skip }, async (t) => {
  const { sceneuron, deck } = await startPlugin(t, { room: true, slow: { "/api/room": 6_000 } }, paired);
  await deck.after(appear("reveal", "reveal-1", {}), image("reveal-1", plain("Reveal votes")), "the reveal key");
  const from = deck.messages.length;
  deck.send(keyDown("reveal", "reveal-1", {}));
  await waitFor(() => sceneuron.room.revealed, "the reveal", 8_000);
  await new Promise((resolve) => setTimeout(resolve, 200));
  assert.deepEqual(deck.messages.slice(from).filter(alert("reveal-1")), [], "no alert sign");
});

test("a view with the room's counts shows them, and a press with no room sends nothing", { skip }, async (t) => {
  const view = { ...sampleView(), room: { joined: false, revealed: false, voted: 0, people: 0 } };
  const { sceneuron, deck } = await startPlugin(t, { view, room: true }, paired);
  await deck.after(appear("reveal", "reveal-1", {}), image("reveal-1", noRoom), "the key saying No room");
  await deck.after(keyDown("reveal", "reveal-1", {}), alert("reveal-1"), "the alert sign");
  assert.deepEqual(sceneuron.posted("/api/room"), []);

  sceneuron.change((shown) => {
    shown.room = { joined: true, revealed: false, voted: 2, people: 3 };
  });
  await deck.next(image("reveal-1", (svg) => words(svg) === "Reveal votes 2/3 voted"), "the count of votes");
  await deck.after(keyDown("reveal", "reveal-1", {}), image("reveal-1", revealed), "the revealed key");
  assert.deepEqual(sceneuron.posted("/api/room"), [{ action: "reveal" }]);
});
