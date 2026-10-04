// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises reading the control view: well-formed views, defaults for optional card fields, the
// optional voice, room, and flood parts, refusal of anything misshapen, including a full Sceneuron
// snapshot, and the lookups by id.

import assert from "node:assert/strict";
import { test } from "node:test";

import { findCard, findDeck, findElement, parseControlView } from "../src/lib/view.ts";
import { sampleView } from "./fake-engine.mjs";

test("a well-formed view reads back unchanged", () => {
  const view = sampleView();
  view.decks[0].raised = "p2";
  view.decks[0].raised_by = "camera";
  view.decks[0].hide_at_ms = 1234;
  view.elements[0].held = true;
  assert.deepEqual(parseControlView(view), view);
});

test("the reaction playing is read when present and left out when misshapen", () => {
  const flood = { flood: "email", nonce: 9, left_ms: 3500 };
  assert.deepEqual(parseControlView({ ...sampleView(), flood }), { ...sampleView(), flood });
  const ended = parseControlView({ ...sampleView(), flood: { flood: "email", nonce: 9, left_ms: -5 } });
  assert.equal(ended.flood.left_ms, 0);
  for (const broken of [{ flood: 7, nonce: 1 }, { flood: "email" }, "email", null]) {
    assert.deepEqual(parseControlView({ ...sampleView(), flood: broken }), sampleView(), JSON.stringify(broken));
  }
});

test("optional card fields fall back and unknown fields are dropped", () => {
  const view = sampleView();
  view.extra = "ignored";
  view.decks[0].cards[0] = { id: "p1", name: "1" };
  view.decks[0].raised_by = "telepathy";
  const parsed = parseControlView(view);
  assert.deepEqual(parsed.decks[0].cards[0], { id: "p1", name: "1", text: "", color: null, background: null });
  assert.equal(parsed.decks[0].raised_by, null);
  assert.equal("extra" in parsed, false);
});

test("the voice and room parts are read when present and left out when misshapen", () => {
  const voice = { enabled: true, talking: false };
  const room = { joined: true, revealed: false, voted: 2, people: 3 };
  assert.deepEqual(parseControlView({ ...sampleView(), voice, room }), { ...sampleView(), voice, room });
  const partial = parseControlView({ ...sampleView(), room: { joined: false } });
  assert.deepEqual(partial.room, { joined: false, revealed: false, voted: 0, people: 0 });
  const broken = parseControlView({ ...sampleView(), voice: { enabled: "yes" }, room: { voted: 2 } });
  assert.deepEqual(broken, sampleView(), "the rest of the view still reads");
  assert.equal(parseControlView({ ...sampleView(), room: { ...room, voted: -1 } }).room.voted, 0);
});

/** `sampleView()` with `change` applied to its first deck or element. */
function changed(part, change) {
  const view = sampleView();
  change(view[part][0]);
  return view;
}

test("misshapen views are refused", () => {
  const broken = [
    null,
    [],
    "view",
    { ...sampleView(), revision: "1" },
    { ...sampleView(), automation_enabled: 1 },
    { ...sampleView(), decks: {} },
    { ...sampleView(), elements: [{ id: "x" }] },
    { ...sampleView(), decks: [{ id: "d", name: "D" }] },
    { ...sampleView(), decks: [{ id: "d", name: "D", raised: null, cards: [{ id: 3, name: "x" }] }] },
    changed("decks", (deck) => delete deck.raised),
    changed("decks", (deck) => (deck.raised = 5)),
    changed("elements", (element) => delete element.visible),
    changed("elements", (element) => delete element.held),
    changed("elements", (element) => (element.visible = "yes")),
  ];
  for (const value of broken) {
    assert.equal(parseControlView(value), null, JSON.stringify(value)?.slice(0, 80));
  }
});

test("a full Sceneuron snapshot is not taken for a control view", () => {
  const element = { id: "brb", name: "Be right back", kind: "text", overridden: ["visible"], automated: [] };
  const snapshot = {
    revision: 7,
    project_revision: 2,
    now_ms: 1000,
    automation_enabled: true,
    elements: [{ ...element, props: { visible: true, text: "BRB" }, effect: null }],
    decks: [],
    events: [],
  };
  assert.equal(parseControlView(snapshot), null);
  const deck = { id: "poker", name: "Planning poker", element: "poker-card", raised: "p5", raised_by: "manual" };
  assert.equal(parseControlView({ ...snapshot, elements: [], decks: [{ ...deck, hide_at_ms: null }] }), null);
});

test("lookups find decks, cards, and elements by id", () => {
  const view = sampleView();
  assert.equal(findDeck(view, "meeting").name, "Meeting signals");
  assert.equal(findCard(findDeck(view, "poker"), "p3").name, "3");
  assert.equal(findElement(view, "brb").name, "Be right back");
  assert.equal(findDeck(view, undefined), undefined);
  assert.equal(findDeck(null, "poker"), undefined);
  assert.equal(findCard(undefined, "p3"), undefined);
  assert.equal(findCard(findDeck(view, "poker"), null), undefined);
});
