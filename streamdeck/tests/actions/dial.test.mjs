// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the "Card dial" in the built plugin: each tick of a turn steps the deck once through
// /api/card, starting from the card the strip shows even while the deck is lowered and from where
// the turn before left it, a press lowers the card that is up and the next press raises it again,
// a long touch lowers the deck, and the alert sign when no deck is chosen.

import assert from "node:assert/strict";
import { test } from "node:test";

import { CONTROL_TOKEN } from "../fake-sceneuron.mjs";
import { alert, appear, bundleExists, dialEvent, dim, lit, strip } from "../fake-stream-deck.mjs";
import { startPlugin } from "../harness.mjs";

const skip = bundleExists() ? false : "the plugin is not built; run npm run build";
const paired = (sceneuron) => ({ port: sceneuron.port, token: CONTROL_TOKEN });
const settings = { deck: "poker" };

/** Matches the strip of `dial-1` at `position`, framed or not as `up` says. */
function at(position, up) {
  return strip("dial-1", (svg) => svg.includes(`>${position}<`) && lit(svg) === up && !dim(svg));
}

/** A plugin whose dial stands on the poker deck, with `raised` up. */
async function dialOn(t, raised, position) {
  const started = await startPlugin(t, {}, paired);
  started.sceneuron.change((view) => {
    view.decks[0].raised = raised;
  });
  await started.deck.after(appear("dial", "dial-1", settings, "Encoder"), at(position, raised !== null), "the strip");
  return started;
}

const turn = (ticks) => dialEvent("dialRotate", "dial-1", settings, { pressed: false, ticks });
const next = { deck: "poker", action: "next" };
const previous = { deck: "poker", action: "previous" };
const raise = (card) => ({ deck: "poker", card, action: "raise" });

test("each tick of a turn steps the deck once, and the strip shows the card that is up", { skip }, async (t) => {
  const { sceneuron, deck } = await dialOn(t, null, "1/4");
  await deck.after(turn(2), at("3/4", true), "p3");
  assert.equal(sceneuron.view.decks[0].raised, "p3");
  await deck.after(turn(-3), at("4/4", true), "p5");
  assert.equal(sceneuron.view.decks[0].raised, "p5");
  assert.deepEqual(sceneuron.posted("/api/card"), [raise("p2"), next, previous, previous, previous]);
});

test("a turn from a lowered deck starts at the card the strip shows", { skip }, async (t) => {
  const { sceneuron, deck } = await dialOn(t, "p3", "3/4");
  await deck.after(dialEvent("dialDown", "dial-1", settings), at("3/4", false), "the lowered card");
  await deck.after(turn(1), at("4/4", true), "p5");
  assert.equal(sceneuron.view.decks[0].raised, "p5");
  await deck.after(dialEvent("dialDown", "dial-1", settings), at("4/4", false), "p5 lowered");
  await deck.after(turn(-2), at("2/4", true), "p2");
  assert.equal(sceneuron.view.decks[0].raised, "p2");
  const toggle = (card) => ({ deck: "poker", card, action: "toggle" });
  assert.deepEqual(sceneuron.posted("/api/card"), [toggle("p3"), raise("p5"), toggle("p5"), raise("p3"), previous]);
});

test("turns sent faster than Sceneuron answers each start where the one before left off", { skip }, async (t) => {
  const { sceneuron, deck } = await dialOn(t, null, "1/4");
  deck.send(turn(1));
  deck.send(turn(1));
  await deck.after(turn(-3), at("4/4", true), "p5");
  assert.equal(sceneuron.view.decks[0].raised, "p5");
  assert.deepEqual(sceneuron.posted("/api/card"), [raise("p2"), next, previous, previous, previous]);
});

test("a press lowers the card that is up, and the next press raises it again", { skip }, async (t) => {
  const { sceneuron, deck } = await dialOn(t, "p3", "3/4");
  await deck.after(dialEvent("dialDown", "dial-1", settings), at("3/4", false), "the lowered card");
  assert.equal(sceneuron.view.decks[0].raised, null);
  await deck.after(dialEvent("dialDown", "dial-1", settings), at("3/4", true), "the card up again");
  assert.equal(sceneuron.view.decks[0].raised, "p3");
  const toggle = { deck: "poker", card: "p3", action: "toggle" };
  assert.deepEqual(sceneuron.posted("/api/card"), [toggle, toggle]);
});

test("a long touch lowers the deck, and a short one toggles like a press", { skip }, async (t) => {
  const { sceneuron, deck } = await dialOn(t, "p2", "2/4");
  const touch = (hold) => dialEvent("touchTap", "dial-1", settings, { tapPos: [100, 50], hold });
  await deck.after(touch(true), at("2/4", false), "the lowered deck");
  await deck.after(touch(false), at("2/4", true), "the card up again");
  assert.deepEqual(sceneuron.posted("/api/card"), [
    { deck: "poker", action: "lower" },
    { deck: "poker", card: "p2", action: "toggle" },
  ]);
});

test("a dial with no deck shows the alert sign on a turn or a press", { skip }, async (t) => {
  const { sceneuron, deck } = await startPlugin(t, {}, paired);
  const blank = strip("dial-1", (svg) => />Card dial</.test(svg) && !dim(svg));
  await deck.after(appear("dial", "dial-1", {}, "Encoder"), blank, "the strip");
  await deck.after(dialEvent("dialRotate", "dial-1", {}, { pressed: false, ticks: 1 }), alert("dial-1"), "an alert");
  await deck.after(dialEvent("dialDown", "dial-1", {}), alert("dial-1"), "another alert");
  assert.deepEqual(sceneuron.posted("/api/card"), []);
});
