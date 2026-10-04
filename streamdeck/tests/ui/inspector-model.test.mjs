// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the inspector page's model: the fields each action shows, the choices in each list,
// which keep a saved choice the plugin does not know, and the settings a choice produces.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { runInThisContext } from "node:vm";

import { inspectorStatus } from "../../src/lib/inspector.ts";
import { sampleView } from "../fake-engine.mjs";
import { PLUGIN_DIR } from "../fake-stream-deck.mjs";

/** The inspector's model script, loaded the way a page loads it but with `module` to export through. */
function loadModel() {
  const source = readFileSync(path.join(PLUGIN_DIR, "ui", "inspector-model.js"), "utf8");
  const module = { exports: {} };
  runInThisContext(`(function (module) {\n${source}\n})`)(module);
  return module.exports;
}

const model = loadModel();
const connected = inspectorStatus("connected", 4466, sampleView());

test("each action shows its own fields", () => {
  assert.equal(model.kindOf("ai.chaski.sceneuron.card"), "card");
  assert.equal(model.kindOf("ai.chaski.sceneuron.other"), null);
  assert.equal(model.kindOf("toString"), null);
  assert.deepEqual(model.fieldsFor("card"), ["deck", "card"]);
  assert.deepEqual(model.fieldsFor("automation"), []);
  assert.deepEqual(model.fieldsFor(null), []);
});

test("the push to talk and room keys have no fields to choose", () => {
  for (const [uuid, kind] of [
    ["ai.chaski.sceneuron.talk", "talk"],
    ["ai.chaski.sceneuron.reveal", "reveal"],
    ["ai.chaski.sceneuron.next-round", "next-round"],
  ]) {
    assert.equal(model.kindOf(uuid), kind);
    assert.deepEqual(model.fieldsFor(kind), []);
  }
});

test("a Reaction key chooses among every preset, before pairing as after", () => {
  assert.equal(model.kindOf("ai.chaski.sceneuron.reaction"), "reaction");
  assert.deepEqual(model.fieldsFor("reaction"), ["reaction"]);
  const unpaired = inspectorStatus("unpaired", 4466, null);
  const choices = model.reactionOptions(unpaired, { reaction: "email" });
  assert.deepEqual(
    choices.map((option) => [option.value, option.selected]),
    [
      ["", false],
      ["tongue", false],
      ["email", true],
      ["mind-blown", false],
      ["ship-it", false],
      ["coffee", false],
      ["absolutely-right", false],
      ["more-tokens", false],
      ["works-on-my-gpu", false],
      ["meatproxy", false],
      ["clanker", false],
    ],
  );
  assert.equal(choices[0].label, "Choose a reaction");
  assert.equal(choices[2].label, "📧 Could have been an email");
  assert.deepEqual(model.reactionOptions(unpaired, { reaction: "gone" }).at(-1), {
    value: "gone",
    label: "gone (not found)",
    selected: true,
  });
  const waiting = model.reactionOptions(model.WAITING, {});
  assert.deepEqual(waiting, [{ value: "", label: "Choose a reaction", selected: true }]);
  assert.deepEqual(model.updatedSettings("reaction", unpaired, {}, "reaction", "coffee"), { reaction: "coffee" });
});

test("choices list what Sceneuron has and select the saved one", () => {
  const decks = model.deckOptions(connected, "card", { deck: "meeting" });
  assert.deepEqual(
    decks.map((option) => [option.value, option.label, option.selected]),
    [
      ["", "Choose a deck", false],
      ["poker", "Planning poker", false],
      ["meeting", "Meeting signals", true],
    ],
  );
  assert.equal(model.deckOptions(connected, "lower", {})[0].label, "All decks");
  assert.equal(model.deckOptions(connected, "lower", {})[0].selected, true);
  const cards = model.cardOptions(connected, { deck: "poker", card: "p3" });
  assert.deepEqual(
    cards.filter((option) => option.selected).map((option) => option.value),
    ["p3"],
  );
});

test("a saved choice Sceneuron does not have stays, marked only while connected", () => {
  const missing = model.elementOptions(connected, { element: "gone" }).at(-1);
  assert.deepEqual(missing, { value: "gone", label: "gone (not found)", selected: true });
  const offline = model.elementOptions(model.WAITING, { element: "gone" }).at(-1);
  assert.deepEqual(offline, { value: "gone", label: "gone", selected: true });
});

test("choosing a deck for a card key points it at that deck's first card", () => {
  const settings = model.updatedSettings("card", connected, { deck: "poker", card: "p3" }, "deck", "meeting");
  assert.deepEqual(settings, { deck: "meeting", card: "wrap-up" });
  assert.deepEqual(model.updatedSettings("card", connected, { deck: "poker", card: "p3" }, "deck", ""), {});
  assert.deepEqual(model.updatedSettings("lower", connected, { deck: "poker" }, "deck", ""), {});
  assert.deepEqual(model.updatedSettings("element", connected, {}, "element", "brb"), { element: "brb" });
});

test("only status messages are read as status", () => {
  assert.equal(model.readStatus(connected), connected);
  assert.equal(model.readStatus({ type: "paired", ok: true }), null);
  assert.equal(model.readStatus({ type: "status", decks: "x", elements: [] }), null);
  assert.equal(model.readStatus(null), null);
});
