// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises render-icons.js and how render() draws a theme with icons on a fake DOM: the card slots and the newest
// card, the tile a card, its cover, a starter label, or a custom card shows, the words without the emoji a tile
// shows, the role classes Graphite draws by, the vote card's note, and a theme without icons drawing none of it.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { createDocument } from "./fake-dom.mjs";
import { element } from "./render-fixtures.mjs";

const require = createRequire(import.meta.url);
const R = require("../render.js");
const Tiles = require("../render-icons.js");

const CANVAS = { width: 1920, height: 1080 };
const MEETING = element("meeting-card", "text", { width: 1160, height: 180, text: "⏱️ Wrap it up" });
const POKER = element("poker-card", "text", { width: 300, height: 420, text: "13" });
const STATUS = element("status-card", "text", { width: 1200, height: 130, text: "🔇 I'm on mute" });
const LABEL = element("name-label", "text", { width: 640, height: 110, text: "Alex Picon", align: "left" });
const PILL = { ...element("on-air", "text", { text: "Scene 01 · LIVE" }), on_air: true };
const TIMER = { ...element("on-air-time", "text", { text: "{{on-air-time}}" }), clock: true };
const SUBTITLES = { ...element("subtitles-spoken", "text", { text: "🎉 Hello" }), subtitles: true };
const DECKS = [
  { id: "meeting", element: "meeting-card", raised: "wrap-up", raised_at_ms: 10, covered: false },
  { id: "poker", element: "poker-card", raised: "p13", raised_at_ms: 30, covered: true, summary: "Voted: 3 of 5" },
  { id: "status", element: "status-card", raised: "muted", raised_at_ms: 20, covered: false },
  { id: "classroom", element: "classroom-card", raised: null, raised_at_ms: null },
];

function draw(theme, elements, extra = {}) {
  const doc = createDocument({ constructable: true });
  const root = doc.body.appendChild(doc.createElement("div"));
  R.render(root, { revision: 1, canvas: CANVAS, theme, decks: DECKS, elements, ...extra });
  return root;
}

function parts(root, id) {
  const node = root.children.find((child) => child.getAttribute("data-element-id") === id);
  const body = node.children[0].children[0].children[0].children[0];
  const tile = body.children.find((child) => child.classList.contains("sn-icon")) || null;
  const has = (name) => node.classList.contains(name);
  return { node, body, text: body.children[0], tile, has };
}

describe("slots", () => {
  test("each card slot knows its deck, raised card, cover, and summary, and only the newest card is active", () => {
    const slots = Tiles.slots({ decks: DECKS });
    assert.deepEqual(slots.get("meeting-card"), {
      deck: "meeting",
      card: "wrap-up",
      covered: false,
      summary: "",
      at: 10,
      active: false,
    });
    assert.equal(slots.get("poker-card").active, true, "the newest, though covered");
    assert.equal(slots.get("status-card").active, false);
    assert.deepEqual([slots.get("classroom-card").card, slots.get("classroom-card").active], [null, false]);
    assert.equal(Tiles.slots(null).size, 0);
  });
});

describe("tileFor", () => {
  const slot = (deck, card, covered = false) => ({ deck, card, covered, summary: "", at: 1, active: false });

  test("a card shows its icon, a cover a check, and a slot without a card nothing", () => {
    assert.deepEqual(Tiles.tileFor(MEETING, slot("meeting", "wrap-up"), "⏱️ Wrap it up"), {
      icon: "timer",
      emoji: null,
      text: "Wrap it up",
      label: "Wrap",
    });
    assert.deepEqual(Tiles.tileFor(POKER, slot("poker", "p13", true), "✓ Voted"), {
      icon: "check",
      emoji: null,
      text: "✓ Voted",
    });
    assert.equal(Tiles.tileFor(POKER, slot("poker", "p13"), "13"), null, "a value is its own face");
    assert.equal(Tiles.tileFor(POKER, slot("poker", null), ""), null);
  });

  test("subtitles, the on-air pill and timer, and boxes show no tile; other text its icon or emoji", () => {
    for (const shown of [SUBTITLES, PILL, TIMER, element("box", "box", {})]) {
      assert.equal(Tiles.tileFor(shown, undefined, "🎉 Hello"), null, shown.id);
    }
    assert.equal(Tiles.tileFor(LABEL, undefined, "Alex Picon").icon, "person");
    assert.equal(Tiles.tileFor(element("banner", "text", {}), undefined, "🎬 Rolling").emoji, "🎬");
    assert.equal(Tiles.tileFor(element("banner", "text", {}), undefined, "Rolling"), null);
  });
});

describe("render with Graphite", () => {
  test("cards show their icon in a tile, hidden from screen readers, beside their words without the emoji", () => {
    const root = draw("graphite", [MEETING, STATUS]);
    const meeting = parts(root, "meeting-card");
    assert.equal(meeting.text.textContent, "Wrap it up");
    assert.ok(meeting.has("sn-card") && meeting.has("sn-tiled") && !meeting.has("sn-tall"));
    assert.equal(meeting.tile.getAttribute("aria-hidden"), "true");
    const [svg] = meeting.tile.children;
    assert.equal(svg.namespace, "http://www.w3.org/2000/svg");
    assert.equal(svg.getAttribute("viewBox"), "0 0 24 24");
    assert.equal(svg.getAttribute("stroke"), "currentColor");
    assert.deepEqual(svg.children.map((shape) => shape.tagName), ["CIRCLE", "PATH"]);
    assert.equal(parts(root, "status-card").text.textContent, "I'm on mute");
  });

  test("the newest card is active, a covered card is a vote with the room's count, and a value has no tile", () => {
    const root = draw("graphite", [MEETING, POKER, STATUS]);
    const poker = parts(root, "poker-card");
    assert.ok(poker.has("sn-vote") && !poker.has("sn-active"), "a vote card is edged as a vote");
    assert.equal(poker.text.getAttribute("data-note"), "Voted: 3 of 5");
    assert.ok(poker.has("sn-tall"), "a tall slot stacks its tile above its words");
    assert.ok(parts(root, "status-card").has("sn-active") === false && !parts(root, "meeting-card").has("sn-active"));
    const decks = DECKS.map((deck) => (deck.id === "poker" ? { ...deck, covered: false } : deck));
    const doc = createDocument({ constructable: true });
    const again = doc.body.appendChild(doc.createElement("div"));
    R.render(again, { revision: 1, canvas: CANVAS, theme: "graphite", decks, elements: [MEETING, POKER] });
    const value = parts(again, "poker-card");
    assert.ok(value.has("sn-active") && !value.has("sn-vote") && !value.has("sn-tiled"));
    assert.equal(value.tile, null);
    assert.equal(value.text.textContent, "13");
    assert.equal(value.text.getAttribute("data-note"), "");
  });

  test("labels, subtitles, and the on-air pill and timer are marked by role", () => {
    const root = draw("graphite", [LABEL, SUBTITLES, PILL, TIMER], { on_air: { since_ms: 0 }, now_ms: 0 });
    const label = parts(root, "name-label");
    assert.ok(label.has("sn-lower-third") && label.has("sn-align-left") && label.has("sn-tiled"));
    assert.equal(label.tile.children[0].children.length, 2, "the person icon");
    const subtitles = parts(root, "subtitles-spoken");
    assert.ok(subtitles.has("sn-subtitles") && !subtitles.has("sn-lower-third"));
    assert.equal(subtitles.text.textContent, "🎉 Hello", "subtitles keep every word");
    assert.ok(parts(root, "on-air").has("sn-on-air") && parts(root, "on-air").has("sn-live"));
    assert.ok(parts(root, "on-air-time").has("sn-clock"));
    const calm = draw("graphite", [PILL], { on_air: { since_ms: null } });
    assert.ok(parts(calm, "on-air").has("sn-on-air") && !parts(calm, "on-air").has("sn-live"), "off air");
  });

  test("a tile follows its card and goes when the theme has no icons, which keeps the words and the emoji", () => {
    const doc = createDocument({ constructable: true });
    const root = doc.body.appendChild(doc.createElement("div"));
    const shot = (theme, decks) => ({ revision: 1, canvas: CANVAS, theme, decks, elements: [MEETING, LABEL] });
    R.render(root, shot("graphite", DECKS));
    const { tile } = parts(root, "meeting-card");
    const offTrack = [{ ...DECKS[0], raised: "off-track" }];
    R.render(root, shot("graphite", offTrack));
    assert.equal(parts(root, "meeting-card").tile, tile, "the same node");
    assert.equal(tile.children[0].children.length, 4, "the off-track drawing");
    R.render(root, shot("hacker", offTrack));
    const meeting = parts(root, "meeting-card");
    assert.equal(meeting.tile, null);
    assert.equal(meeting.text.textContent, "⏱️ Wrap it up");
    for (const role of ["sn-tiled", "sn-active", "sn-lower-third", "sn-align-left"]) {
      assert.ok(!meeting.has(role) && !parts(root, "name-label").has(role), role);
    }
  });
});
