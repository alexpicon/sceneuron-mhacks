// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises render-cards.js and how render() draws Graphite's card looks on a fake DOM: which look and face each
// card gets, the vote tab's words, the side a bubble's tail hangs from, the poker card turning over in the render
// that uncovers it without its value in the page before, the newest card's pulse, hidden cards marked gone, and
// every other theme drawing none of it.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { createDocument, descendants } from "./fake-dom.mjs";
import { element } from "./render-fixtures.mjs";

const require = createRequire(import.meta.url);
const R = require("../render.js");
const Cards = require("../render-cards.js");

const CANVAS = { width: 1920, height: 1080 };
const POKER = element("poker-card", "text", { x: 1560, y: 600, width: 300, height: 420, text: "13" });
const STATUS = element("status-card", "text", { x: 360, y: 40, width: 1200, height: 130, text: "🖥️ Presenting" });
const CLASSROOM = element("classroom-card", "text", { x: 60, y: 400, width: 760, height: 220, text: "B" });
const slot = (deck, card, extra = {}) => ({ deck, card, covered: false, summary: "", at: 1, active: true, ...extra });

describe("cardLook", () => {
  const look = (s, text = "13", props = POKER.props) => Cards.cardLook("graphite", s, text, props, CANVAS);

  test("a covered poker card is face down with the room's count on its tab", () => {
    const covered = slot("poker", "p13", { covered: true, summary: "Voted: 3 of 5" });
    assert.deepEqual(look(covered, "✓ Voted"), { kind: "poker", face: "back", tab: "Voted · 3 of 5" });
  });

  test("an uncovered poker card shows its value, a question mark, or a coffee break with the mug", () => {
    assert.deepEqual(look(slot("poker", "p13")), {
      kind: "poker",
      face: "front",
      value: "13",
      icon: null,
      caption: "STORY POINTS",
      results: null,
      side: "left",
    });
    assert.equal(look(slot("poker", "unsure"), "?").value, "?");
    assert.deepEqual(look(slot("poker", "break"), "☕"), {
      kind: "poker",
      face: "front",
      value: "",
      icon: "mug",
      caption: "COFFEE BREAK",
      results: null,
      side: "left",
    });
  });

  test("status cards get a dot by card, and a custom one a gray dot", () => {
    const dot = (card) => look(slot("status", card), "x", STATUS.props).dot;
    assert.deepEqual(["presenting", "on-call", "brb", "lunch", "muted", "focus", "custom"].map(dot), [
      "red",
      "red",
      "amber",
      "amber",
      "grey",
      "grey",
      "grey",
    ]);
  });

  test("classroom cards are bubbles, and quiz answers show their letter", () => {
    assert.deepEqual(look(slot("classroom", "hand"), "✋ Hand raised", CLASSROOM.props), {
      kind: "bubble",
      tail: "right",
    });
    assert.deepEqual(look(slot("classroom", "answer-b"), " B ", CLASSROOM.props), {
      kind: "quiz",
      letter: "B",
      tail: "right",
    });
  });

  test("other themes, empty slots, other decks, and covered non-poker cards get no look", () => {
    assert.equal(Cards.cardLook("hacker", slot("poker", "p13"), "13", POKER.props, CANVAS), null);
    assert.equal(look(slot("poker", null)), null);
    assert.equal(look(undefined), null);
    assert.equal(look(slot("meeting", "wrap-up")), null);
    assert.equal(look(slot("status", "muted", { covered: true })), null);
  });
});

describe("tabText and tailSide", () => {
  test("a revealed summary becomes each vote and the average; before the reveal there are none", () => {
    assert.deepEqual(Cards.results("Maya 2 · You 5 · Mary Ann 13 · Average 6.7"), {
      rows: [
        { who: "Maya", vote: "2" },
        { who: "You", vote: "5" },
        { who: "Mary Ann", vote: "13" },
      ],
      average: "6.7",
    });
    assert.deepEqual(Cards.results("Leo ? · Sam ☕"), {
      rows: [
        { who: "Leo", vote: "?" },
        { who: "Sam", vote: "☕" },
      ],
      average: null,
    });
    for (const before of ["", "Voted: 3 of 5", "No votes"]) {
      assert.equal(Cards.results(before), null, before);
    }
  });

  test("the tab reads the summary with a middle dot, or the cover's words", () => {
    assert.equal(Cards.tabText("Voted: 2 of 3", "✓ Voted"), "Voted · 2 of 3");
    assert.equal(Cards.tabText("", "✓ Voted"), "Voted");
    assert.equal(Cards.tabText("Everyone voted", ""), "Everyone voted");
    assert.equal(Cards.tabText("", ""), "Voted");
  });

  test("the tail hangs from the end nearer the frame's center", () => {
    assert.equal(Cards.tailSide({ x: 60, width: 760 }, CANVAS), "right");
    assert.equal(Cards.tailSide({ x: 1100, width: 760 }, CANVAS), "left");
    assert.equal(Cards.tailSide({ x: 1100, width: 760 }, null), "left");
  });
});

function room(theme = "graphite") {
  const doc = createDocument({ constructable: true });
  const root = doc.body.appendChild(doc.createElement("div"));
  const draw = (decks, elements, extra = {}) =>
    R.render(root, { revision: 1, canvas: CANVAS, theme, decks, elements, ...extra });
  const node = (id) => root.children.find((child) => child.getAttribute("data-element-id") === id);
  const find = (id, cls) => descendants(node(id)).find((child) => child.classList.contains(cls)) || null;
  return { draw, node, find };
}

const pokerDeck = (extra) => ({ id: "poker", element: "poker-card", raised: "p13", raised_at_ms: 5, ...extra });

describe("render", () => {
  test("a covered poker card turns face up in the render that uncovers it, its value written only then", () => {
    const { draw, node, find } = room();
    const cover = { ...POKER, props: { ...POKER.props, text: "✓ Voted" } };
    draw([pokerDeck({ covered: true, summary: "Voted: 3 of 5" })], [cover]);
    const flipper = find("poker-card", "sn-gp-flipper");
    assert.ok(node("poker-card").classList.contains("sn-gp-poker"));
    assert.equal(flipper.classList.contains("sn-gp-up"), false);
    assert.equal(find("poker-card", "sn-gp-tab-text").textContent, "Voted · 3 of 5");
    assert.equal(find("poker-card", "sn-gp-value").textContent, "", "the covered value is not in the page");
    draw([pokerDeck({ covered: false })], [POKER]);
    assert.equal(find("poker-card", "sn-gp-flipper"), flipper, "the same node turns over");
    assert.equal(flipper.classList.contains("sn-gp-up"), true);
    assert.equal(find("poker-card", "sn-gp-value").textContent, "13");
    assert.equal(find("poker-card", "sn-gp-caption").textContent, "STORY POINTS");
  });

  test("after the reveal, everyone's votes and the average show beside the card, toward the center", () => {
    const { draw, node, find } = room();
    const cover = { ...POKER, props: { ...POKER.props, text: "✓ Voted" } };
    draw([pokerDeck({ covered: true, summary: "Voted: 2 of 3" })], [cover]);
    assert.equal(find("poker-card", "sn-gp-results").classList.contains("sn-gp-on"), false);
    draw([pokerDeck({ covered: false, summary: "Maya 3 · You 13 · Sam 13 · Average 9.7" })], [POKER]);
    const box = find("poker-card", "sn-gp-results");
    assert.ok(box.classList.contains("sn-gp-on") && box.classList.contains("sn-gp-results-left"));
    assert.equal(box.textContent, "Maya3You13Sam13Average9.7");
    assert.ok(!descendants(find("poker-card", "sn-gp-flipper")).includes(box), "it does not turn over with the card");
    draw([pokerDeck({ covered: false, summary: "" })], [POKER]);
    assert.equal(find("poker-card", "sn-gp-results").classList.contains("sn-gp-on"), false);
    assert.ok(node("poker-card"));
  });

  test("a coffee break shows the mug icon and its words, no emoji", () => {
    const { draw, find } = room();
    draw([pokerDeck({ raised: "break" })], [{ ...POKER, props: { ...POKER.props, text: "☕" } }]);
    const value = find("poker-card", "sn-gp-value");
    assert.equal(value.textContent, "");
    assert.equal(value.children[0].tagName, "SVG");
    assert.equal(find("poker-card", "sn-gp-caption").textContent, "COFFEE BREAK");
  });

  test("status pills, bubbles, and quiz answers get their classes and nodes", () => {
    const { draw, node, find } = room();
    const decks = [
      { id: "status", element: "status-card", raised: "brb", raised_at_ms: 1 },
      { id: "classroom", element: "classroom-card", raised: "answer-b", raised_at_ms: 2 },
    ];
    draw(decks, [STATUS, CLASSROOM]);
    const has = (id, cls) => node(id).classList.contains(cls);
    assert.ok(has("status-card", "sn-gp-status") && has("status-card", "sn-gp-dot-amber"));
    assert.ok(has("classroom-card", "sn-gp-bubble") && has("classroom-card", "sn-gp-quiz"));
    assert.ok(has("classroom-card", "sn-gp-tail-right"));
    assert.equal(find("classroom-card", "sn-gp-letter").textContent, "B");
    draw([decks[0], { ...decks[1], raised: "hand" }], [STATUS, CLASSROOM]);
    assert.ok(!has("classroom-card", "sn-gp-quiz") && has("classroom-card", "sn-gp-bubble"));
    assert.equal(find("classroom-card", "sn-gp-answer"), null);
  });

  test("the newest card pulses each time it is raised, not when the page loads with it up", () => {
    const { draw, node } = room();
    draw([pokerDeck({ raised_at_ms: 5 })], [POKER]);
    assert.equal(node("poker-card").classList.contains("sn-gp-pulse"), false);
    draw([pokerDeck({ raised_at_ms: 9 })], [POKER]);
    assert.equal(node("poker-card").classList.contains("sn-gp-pulse"), true);
  });

  test("a lowered status card fades out as its pill, with its tile and words, not as the slot's full panel", () => {
    const { draw, node, find } = room();
    draw([{ id: "status", element: "status-card", raised: "brb", raised_at_ms: 1 }], [STATUS]);
    const words = node("status-card").textContent;
    const lowered = { ...STATUS, props: { ...STATUS.props, visible: false, text: "" } };
    draw([{ id: "status", element: "status-card", raised: null }], [lowered]);
    const has = (cls) => node("status-card").classList.contains(cls);
    assert.ok(has("sn-gone") && has("sn-gp-status") && has("sn-tiled"), "still a pill while it goes");
    assert.ok(find("status-card", "sn-icon"), "its tile stays");
    assert.equal(node("status-card").textContent, words, "its words stay");
    draw([{ id: "status", element: "status-card", raised: "lunch", raised_at_ms: 2 }], [STATUS]);
    assert.ok(has("sn-gp-dot-amber") && !has("sn-gone"), "the next card takes its own look");
  });

  test("a hidden element is marked gone, unless drawn as a ghost", () => {
    const { draw, node } = room();
    const hidden = { ...POKER, props: { ...POKER.props, visible: false } };
    draw([], [hidden]);
    assert.equal(node("poker-card").classList.contains("sn-gone"), true);
    draw([], [POKER]);
    assert.equal(node("poker-card").classList.contains("sn-gone"), false);
  });

  test("another theme draws none of the card looks, and switching to it drops them", () => {
    const { draw, node, find } = room();
    draw([pokerDeck({ covered: true })], [POKER]);
    assert.ok(find("poker-card", "sn-gp-flipper"));
    draw([pokerDeck({ covered: true })], [POKER], { theme: "hacker" });
    assert.equal(find("poker-card", "sn-gp-flipper"), null);
    assert.equal(node("poker-card").classList.names.size > 0, true);
    for (const cls of node("poker-card").classList.names) {
      assert.ok(!cls.startsWith("sn-gp-"), cls);
    }
  });
});
