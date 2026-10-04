// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the plugin's side of the property inspector: the status it sends (names only, never
// the token), a message for every connection state, and the requests it accepts.

import assert from "node:assert/strict";
import { test } from "node:test";

import { inspectorStatus, readInspectorRequest, statusMessage } from "../src/lib/inspector.ts";
import { sampleView } from "./fake-engine.mjs";

test("the status lists decks, cards, and elements by id and name only", () => {
  const status = inspectorStatus("connected", 4466, sampleView());
  assert.equal(status.type, "status");
  assert.equal(status.message, "Connected to Sceneuron on port 4466.");
  assert.deepEqual(status.decks[0], {
    id: "poker",
    name: "Planning poker",
    cards: [
      { id: "p1", name: "1" },
      { id: "p2", name: "2" },
      { id: "p3", name: "3" },
      { id: "p5", name: "5" },
    ],
  });
  assert.deepEqual(status.elements, [
    { id: "brb", name: "Be right back" },
    { id: "lower-third", name: "Lower third" },
  ]);
  assert.deepEqual(inspectorStatus("unpaired", 4466, null).decks, []);
});

test("the status lists every reaction preset, paired or not", () => {
  const reactions = inspectorStatus("unpaired", 4466, null).reactions;
  assert.deepEqual(reactions.map((preset) => preset.id), ["tongue", "email", "mind-blown", "ship-it", "coffee", "absolutely-right", "more-tokens", "works-on-my-gpu", "meatproxy", "clanker"]);
  assert.deepEqual(reactions[1], { id: "email", name: "📧 Could have been an email" });
  assert.deepEqual(inspectorStatus("connected", 4466, sampleView()).reactions, reactions);
});

test("every state has a message that names the port", () => {
  for (const state of ["connecting", "connected", "refused", "offline"]) {
    assert.match(statusMessage(state, 5123), /5123/, state);
  }
  assert.match(statusMessage("unpaired", 4466), /Pair the Stream Deck on this computer, or .*Connect a Stream Deck/);
  assert.match(statusMessage("refused", 4466), /pair again/);
});

test("only known requests are read", () => {
  assert.deepEqual(readInspectorRequest({ type: "refresh" }), { type: "refresh" });
  assert.deepEqual(readInspectorRequest({ type: "forget", extra: 1 }), { type: "forget" });
  assert.deepEqual(readInspectorRequest({ type: "pair", port: "4466", code: "123456" }), {
    type: "pair",
    port: "4466",
    code: "123456",
  });
  for (const payload of [null, "pair", { type: "token" }, [], 7]) {
    assert.equal(readInspectorRequest(payload), null);
  }
});
