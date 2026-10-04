// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises render-results.js: reading a reveal's summary into votes and the average, the side toward the frame's
// center, and the panel every theme but Graphite gets beside a revealed card slot, wearing the theme's card look.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { test } from "node:test";

import { createDocument, descendants } from "./fake-dom.mjs";
import { element } from "./render-fixtures.mjs";

const require = createRequire(import.meta.url);
const R = require("../render.js");
const Results = require("../render-results.js");

const CANVAS = { width: 1920, height: 1080 };
const POKER = element("poker-card", "text", { x: 1560, y: 600, width: 300, height: 420, text: "5" });
const deck = (extra) => ({ id: "poker", element: "poker-card", raised: "p5", raised_at_ms: 5, ...extra });

function room(theme) {
  const doc = createDocument({ constructable: true });
  const root = doc.body.appendChild(doc.createElement("div"));
  const draw = (decks) => R.render(root, { revision: 1, canvas: CANVAS, theme, decks, elements: [POKER] });
  const panel = () =>
    descendants(root).find((node) => node.classList && node.classList.contains("sn-room-results")) || null;
  return { draw, panel };
}

test("a summary becomes each vote and the average, and its lines", () => {
  const shown = Results.results("Maya 3 · You 5 · Average 4");
  assert.deepEqual(shown, { rows: [{ who: "Maya", vote: "3" }, { who: "You", vote: "5" }], average: "4" });
  assert.deepEqual(Results.lines(shown), ["Maya  3", "You  5", "Average  4"]);
  assert.equal(Results.results("Voted: 2 of 5"), null);
});

test("the panel sits toward the frame's center", () => {
  assert.equal(Results.towardCenter({ x: 1560, width: 300 }, CANVAS), "left");
  assert.equal(Results.towardCenter({ x: 60, width: 300 }, CANVAS), "right");
});

test("other themes show the revealed votes beside the card in a body of their own look", () => {
  for (const theme of ["classic-mac", "windows-95", "none"]) {
    const { draw, panel } = room(theme);
    draw([deck({ covered: true, summary: "Voted: 2 of 2" })]);
    assert.equal(panel(), null, `${theme}: nothing before the reveal`);
    draw([deck({ covered: false, summary: "Maya 3 · You 5 · Average 4" })]);
    const box = panel();
    assert.ok(box && box.classList.contains("sn-body"), theme);
    assert.equal(box.textContent, "Maya  3\nYou  5\nAverage  4");
    draw([deck({ covered: false, summary: "" })]);
    assert.equal(panel(), null, `${theme}: gone with the next round`);
  }
});

test("Graphite draws its own results, not this panel", () => {
  const { draw, panel } = room("graphite");
  draw([deck({ covered: false, summary: "Maya 3 · You 5 · Average 4" })]);
  assert.equal(panel(), null);
});
